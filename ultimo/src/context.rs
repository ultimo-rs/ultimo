//! Request/Response context for handling HTTP requests and responses
//!
//! Provides a unified interface for handling HTTP requests and building responses.

use crate::{
    error::{Result, UltimoError},
    response::{Response, ResponseBuilder},
    router::Params,
};
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{body::Incoming, Request as HyperRequest};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "database")]
use crate::database::Database;

/// Request wraps the incoming HTTP request and provides easy access to request data
pub struct Request {
    method: hyper::Method,
    uri: hyper::Uri,
    headers: hyper::HeaderMap,
    params: Params,
    body: Arc<RwLock<Option<Bytes>>>,
}

impl Request {
    /// Create a new Request from a Hyper request and path parameters
    pub async fn new(req: HyperRequest<Incoming>, params: Params) -> Result<Self> {
        let (parts, body) = req.into_parts();

        // Collect the full body
        let collected = body
            .collect()
            .await
            .map_err(|e| UltimoError::Internal(format!("Failed to read body: {}", e)))?;
        let body_bytes = collected.to_bytes();

        Ok(Self {
            method: parts.method,
            uri: parts.uri,
            headers: parts.headers,
            params,
            body: Arc::new(RwLock::new(Some(body_bytes))),
        })
    }

    /// Get a path parameter by name
    pub fn param(&self, name: &str) -> Result<&str> {
        self.params
            .get(name)
            .map(|s| s.as_str())
            .ok_or_else(|| UltimoError::BadRequest(format!("Missing path parameter: {}", name)))
    }

    /// Get all path parameters at once
    pub fn params(&self) -> &Params {
        &self.params
    }

    /// Get a query parameter by name
    pub fn query(&self, name: &str) -> Option<String> {
        self.uri.query().and_then(|q| {
            q.split('&').find_map(|pair| {
                let (key, value) = pair.split_once('=')?;
                if key == name {
                    Some(value.to_string())
                } else {
                    None
                }
            })
        })
    }

    /// Get all query parameters at once
    pub fn queries(&self) -> HashMap<String, Vec<String>> {
        let mut result: HashMap<String, Vec<String>> = HashMap::new();

        if let Some(query) = self.uri.query() {
            for pair in query.split('&') {
                let mut parts = pair.splitn(2, '=');
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    result
                        .entry(key.to_string())
                        .or_default()
                        .push(value.to_string());
                }
            }
        }

        result
    }

    /// Get a header value by name
    pub fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }

    /// Get the request path
    pub fn path(&self) -> &str {
        self.uri.path()
    }

    /// Get the full URL as a string
    pub fn url(&self) -> String {
        self.uri.to_string()
    }

    /// Get the request method
    pub fn method(&self) -> &hyper::Method {
        &self.method
    }

    /// Parse request body as JSON
    pub async fn json<T: DeserializeOwned>(&self) -> Result<T> {
        let body = self.body.read().await;
        let bytes = body
            .as_ref()
            .ok_or_else(|| UltimoError::BadRequest("Body already consumed".to_string()))?;

        serde_json::from_slice(bytes).map_err(UltimoError::Json)
    }

    /// Parse request body as text
    pub async fn text(&self) -> Result<String> {
        let body = self.body.read().await;
        let bytes = body
            .as_ref()
            .ok_or_else(|| UltimoError::BadRequest("Body already consumed".to_string()))?;

        String::from_utf8(bytes.to_vec())
            .map_err(|e| UltimoError::BadRequest(format!("Invalid UTF-8: {}", e)))
    }

    /// Get request body as bytes
    pub async fn bytes(&self) -> Result<Bytes> {
        let body = self.body.read().await;
        body.as_ref()
            .cloned()
            .ok_or_else(|| UltimoError::BadRequest("Body already consumed".to_string()))
    }
}

/// Context holds request data and provides response building methods
pub struct Context {
    pub req: Request,
    state: Arc<RwLock<HashMap<String, String>>>,
    response_status: Arc<RwLock<Option<u16>>>,
    response_headers: Arc<RwLock<HashMap<String, String>>>,

    #[cfg(feature = "database")]
    database: Option<Database>,
}

impl Context {
    /// Create a new context from a request and params
    pub async fn new(req: HyperRequest<Incoming>, params: Params) -> Result<Self> {
        let request = Request::new(req, params).await?;

        Ok(Self {
            req: request,
            state: Arc::new(RwLock::new(HashMap::new())),
            response_status: Arc::new(RwLock::new(None)),
            response_headers: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "database")]
            database: None,
        })
    }

    /// Attach a database to this context (internal use)
    #[cfg(feature = "database")]
    pub(crate) fn attach_database(&mut self, db: Database) {
        self.database = Some(db);
    }

    /// Get the database pool (SQLx)
    #[cfg(feature = "sqlx")]
    pub fn sqlx<DB: sqlx::Database>(&self) -> Result<&sqlx::Pool<DB>> {
        let db = self
            .database
            .as_ref()
            .ok_or(crate::database::DatabaseError::NotConfigured)?;

        let sqlx_pool = db.as_sqlx::<DB>()?;
        Ok(sqlx_pool.pool())
    }

    /// Get a Diesel connection from the pool
    #[cfg(feature = "diesel")]
    pub fn diesel<Conn>(
        &self,
    ) -> Result<diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<Conn>>>
    where
        Conn: diesel::Connection + diesel::r2d2::R2D2Connection + 'static,
    {
        let db = self
            .database
            .as_ref()
            .ok_or(crate::database::DatabaseError::NotConfigured)?;

        let diesel_pool = db.as_diesel::<Conn>()?;
        diesel_pool.get().map_err(Into::into)
    }

    /// Get the database (generic access)
    #[cfg(feature = "database")]
    pub fn database(&self) -> Result<&Database> {
        self.database
            .as_ref()
            .ok_or(crate::database::DatabaseError::NotConfigured.into())
    }

    /// Set a value in the context state (shared between middleware)
    pub async fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut state = self.state.write().await;
        state.insert(key.into(), value.into());
    }

    /// Get a value from the context state
    pub async fn get(&self, key: &str) -> Option<String> {
        let state = self.state.read().await;
        state.get(key).cloned()
    }

    /// Set the response status code
    pub async fn status(&self, code: u16) {
        let mut status = self.response_status.write().await;
        *status = Some(code);
    }

    /// Add a response header
    pub async fn header(&self, name: impl Into<String>, value: impl Into<String>) {
        let mut headers = self.response_headers.write().await;
        headers.insert(name.into(), value.into());
    }

    /// Build response with collected status and headers
    async fn build_response(&self, mut builder: ResponseBuilder) -> ResponseBuilder {
        // Apply status if set
        if let Some(status) = *self.response_status.read().await {
            builder = builder.status(status);
        }

        // Apply headers
        let headers = self.response_headers.read().await;
        for (name, value) in headers.iter() {
            builder = builder.header(name.clone(), value.clone());
        }

        builder
    }

    /// Return a JSON response
    pub async fn json<T: Serialize>(&self, value: T) -> Result<Response> {
        let builder = self.build_response(ResponseBuilder::new()).await;
        builder.json(&value)?.build()
    }

    /// Return a text response
    pub async fn text(&self, text: impl Into<String>) -> Result<Response> {
        let builder = self.build_response(ResponseBuilder::new()).await;
        builder.text(text).build()
    }

    /// Return an HTML response
    pub async fn html(&self, html: impl Into<String>) -> Result<Response> {
        let builder = self.build_response(ResponseBuilder::new()).await;
        builder.html(html).build()
    }

    /// Return a redirect response
    pub async fn redirect(&self, location: &str) -> Result<Response> {
        let status = self.response_status.read().await.unwrap_or(302);
        let builder = ResponseBuilder::new()
            .status(status)
            .header("Location", location);
        builder.build()
    }

    /// Return a not found response
    pub async fn not_found(&self) -> Result<Response> {
        self.status(404).await;
        self.json(serde_json::json!({
            "error": "NotFound",
            "message": "The requested resource was not found"
        }))
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_api() {
        // Test the Params type directly
        let mut params = Params::new();
        params.insert("id".to_string(), "123".to_string());
        params.insert("name".to_string(), "test".to_string());

        assert_eq!(params.get("id"), Some(&"123".to_string()));
        assert_eq!(params.get("name"), Some(&"test".to_string()));
        assert_eq!(params.get("missing"), None);
    }

    #[test]
    fn test_query_parsing() {
        // Test query string parsing logic
        let uri: hyper::Uri = "/search?q=rust&page=2".parse().unwrap();

        let query_str = uri.query().unwrap();
        let mut found_q = false;
        let mut found_page = false;

        for pair in query_str.split('&') {
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() == 2 {
                if parts[0] == "q" && parts[1] == "rust" {
                    found_q = true;
                }
                if parts[0] == "page" && parts[1] == "2" {
                    found_page = true;
                }
            }
        }

        assert!(found_q);
        assert!(found_page);
    }

    #[test]
    fn test_queries_parsing_with_duplicates() {
        // Test parsing multiple values for same key
        let uri: hyper::Uri = "/search?tags=rust&tags=web&page=1".parse().unwrap();

        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(query) = uri.query() {
            for pair in query.split('&') {
                let mut parts = pair.splitn(2, '=');
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    result
                        .entry(key.to_string())
                        .or_default()
                        .push(value.to_string());
                }
            }
        }

        assert_eq!(
            result.get("tags").unwrap(),
            &vec!["rust".to_string(), "web".to_string()]
        );
        assert_eq!(result.get("page").unwrap(), &vec!["1".to_string()]);
    }

    #[test]
    fn test_uri_path() {
        let uri: hyper::Uri = "/api/users/123".parse().unwrap();
        assert_eq!(uri.path(), "/api/users/123");
    }

    #[test]
    fn test_uri_to_string() {
        let uri: hyper::Uri = "/api/users?page=1".parse().unwrap();
        assert_eq!(uri.to_string(), "/api/users?page=1");
    }

    #[test]
    fn test_method_types() {
        let get = hyper::Method::GET;
        let post = hyper::Method::POST;
        let put = hyper::Method::PUT;
        let delete = hyper::Method::DELETE;

        assert_eq!(get, hyper::Method::GET);
        assert_eq!(post, hyper::Method::POST);
        assert_eq!(put, hyper::Method::PUT);
        assert_eq!(delete, hyper::Method::DELETE);
    }

    #[tokio::test]
    async fn test_state_operations() {
        // Test state HashMap operations
        let state = Arc::new(RwLock::new(HashMap::new()));

        {
            let mut s = state.write().await;
            s.insert("user_id".to_string(), "123".to_string());
            s.insert("role".to_string(), "admin".to_string());
        }

        {
            let s = state.read().await;
            assert_eq!(s.get("user_id"), Some(&"123".to_string()));
            assert_eq!(s.get("role"), Some(&"admin".to_string()));
            assert_eq!(s.get("missing"), None);
        }
    }

    #[tokio::test]
    async fn test_response_status_tracking() {
        let status = Arc::new(RwLock::new(None));

        {
            let mut s = status.write().await;
            *s = Some(404);
        }

        {
            let s = status.read().await;
            assert_eq!(*s, Some(404));
        }
    }

    #[tokio::test]
    async fn test_response_headers_tracking() {
        let headers = Arc::new(RwLock::new(HashMap::new()));

        {
            let mut h = headers.write().await;
            h.insert("x-custom".to_string(), "value".to_string());
            h.insert("content-type".to_string(), "application/json".to_string());
        }

        {
            let h = headers.read().await;
            assert_eq!(h.get("x-custom"), Some(&"value".to_string()));
            assert_eq!(h.get("content-type"), Some(&"application/json".to_string()));
        }
    }

    #[test]
    fn test_json_deserialization() {
        // Test JSON parsing
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct User {
            name: String,
            age: u32,
        }

        let json_str = r#"{"name":"John","age":30}"#;
        let user: User = serde_json::from_str(json_str).unwrap();

        assert_eq!(user.name, "John");
        assert_eq!(user.age, 30);
    }

    #[test]
    fn test_text_parsing() {
        let text = "Hello, World!";
        let bytes = Bytes::from(text);
        let parsed = String::from_utf8(bytes.to_vec()).unwrap();

        assert_eq!(parsed, "Hello, World!");
    }

    #[test]
    fn test_bytes_operations() {
        let data = "binary data";
        let bytes = Bytes::from(data);

        assert_eq!(bytes, Bytes::from("binary data"));
        assert_eq!(bytes.len(), 11);
    }
}
