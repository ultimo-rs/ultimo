//! HTTP response builder and utilities
//!
//! Internal response building that gets wrapped by Context methods.

use crate::error::{Result, UltimoError};
use http_body_util::Full;
use hyper::{body::Bytes, header::HeaderValue, Response as HyperResponse, StatusCode};
use serde::Serialize;
use std::collections::HashMap;

/// HTTP Response type used throughout Ultimo
pub type Response = HyperResponse<Full<Bytes>>;

/// Response builder for constructing HTTP responses
#[derive(Debug)]
pub struct ResponseBuilder {
    status: StatusCode,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl ResponseBuilder {
    /// Create a new response builder with 200 OK status
    pub fn new() -> Self {
        Self {
            status: StatusCode::OK,
            headers: HashMap::new(),
            body: None,
        }
    }

    /// Set the HTTP status code
    pub fn status(mut self, status: u16) -> Self {
        self.status = StatusCode::from_u16(status).unwrap_or(StatusCode::OK);
        self
    }

    /// Add a header to the response
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Set the response body as bytes
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set JSON response body and content-type
    pub fn json<T: Serialize>(self, value: &T) -> Result<Self> {
        let json = serde_json::to_vec(value)?;
        Ok(self.header("Content-Type", "application/json").body(json))
    }

    /// Set text response body and content-type
    pub fn text(self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.header("Content-Type", "text/plain; charset=utf-8")
            .body(text.into_bytes())
    }

    /// Set HTML response body and content-type
    pub fn html(self, html: impl Into<String>) -> Self {
        let html = html.into();
        self.header("Content-Type", "text/html; charset=utf-8")
            .body(html.into_bytes())
    }

    /// Build the final HTTP response
    pub fn build(self) -> Result<Response> {
        let mut response = HyperResponse::builder().status(self.status);

        // Add all headers
        for (name, value) in self.headers {
            response = response.header(
                name.as_str(),
                HeaderValue::from_str(&value)
                    .map_err(|_| UltimoError::Internal("Invalid header value".to_string()))?,
            );
        }

        // Set body
        let body = self.body.unwrap_or_default();
        response
            .body(Full::new(Bytes::from(body)))
            .map_err(|e| UltimoError::Internal(format!("Failed to build response: {}", e)))
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for common responses
pub mod helpers {
    use super::*;

    /// Create a JSON response
    pub fn json<T: Serialize>(value: &T) -> Result<Response> {
        ResponseBuilder::new().json(value)?.build()
    }

    /// Create a text response
    pub fn text(text: impl Into<String>) -> Result<Response> {
        ResponseBuilder::new().text(text).build()
    }

    /// Create an HTML response
    pub fn html(html: impl Into<String>) -> Result<Response> {
        ResponseBuilder::new().html(html).build()
    }

    /// Create a redirect response
    pub fn redirect(location: &str, status: Option<u16>) -> Result<Response> {
        let status = status.unwrap_or(302);
        ResponseBuilder::new()
            .status(status)
            .header("Location", location)
            .build()
    }

    /// Create a not found response
    pub fn not_found() -> Result<Response> {
        ResponseBuilder::new()
            .status(404)
            .json(&serde_json::json!({
                "error": "NotFound",
                "message": "The requested resource was not found"
            }))?
            .build()
    }

    /// Create an error response from UltimoError
    pub fn error_response(error: &UltimoError) -> Result<Response> {
        let status = error.status_code();
        let body = error.to_error_response();
        ResponseBuilder::new().status(status).json(&body)?.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_response() {
        let result = helpers::json(&json!({"message": "Hello"}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_response() {
        let result = helpers::text("Hello World");
        assert!(result.is_ok());
    }

    #[test]
    fn test_html_response() {
        let result = helpers::html("<h1>Hello</h1>");
        assert!(result.is_ok());
    }

    #[test]
    fn test_redirect_response() {
        let result = helpers::redirect("/login", Some(301));
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    }

    #[test]
    fn test_response_builder() {
        let result = ResponseBuilder::new()
            .status(201)
            .header("X-Custom", "value")
            .text("Created")
            .build();

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }
}
