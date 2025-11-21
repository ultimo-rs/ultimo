//! Error handling system for Ultimo
//!
//! Provides comprehensive error types that automatically convert to proper HTTP responses
//! with structured JSON error messages.

use serde::Serialize;
use std::fmt;

/// Main error type for Ultimo framework
#[derive(Debug)]
pub enum UltimoError {
    /// HTTP error with status code and message
    Http { status: u16, message: String },
    /// Validation error with field-level details
    Validation {
        message: String,
        details: Vec<ValidationError>,
    },
    /// Authentication error (401)
    Unauthorized(String),
    /// Authorization error (403)
    Forbidden(String),
    /// Not found error (404)
    NotFound(String),
    /// Internal server error (500)
    Internal(String),
    /// Bad request error (400)
    BadRequest(String),
    /// Hyper-specific errors
    Hyper(hyper::Error),
    /// HTTP errors
    HttpError(hyper::http::Error),
    /// JSON serialization/deserialization errors
    Json(serde_json::Error),
    /// IO errors
    Io(std::io::Error),
}

/// Field-level validation error
#[derive(Debug, Clone, Serialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// Standard error response format
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<ValidationError>>,
}

impl UltimoError {
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            UltimoError::Http { status, .. } => *status,
            UltimoError::Validation { .. } => 400,
            UltimoError::Unauthorized(_) => 401,
            UltimoError::Forbidden(_) => 403,
            UltimoError::NotFound(_) => 404,
            UltimoError::BadRequest(_) => 400,
            UltimoError::Internal(_) => 500,
            UltimoError::Hyper(_) => 500,
            UltimoError::HttpError(_) => 500,
            UltimoError::Json(_) => 400,
            UltimoError::Io(_) => 500,
        }
    }

    /// Convert error to JSON response body
    pub fn to_error_response(&self) -> ErrorResponse {
        match self {
            UltimoError::Http { message, .. } => ErrorResponse {
                error: "HttpError".to_string(),
                message: message.clone(),
                details: None,
            },
            UltimoError::Validation { message, details } => ErrorResponse {
                error: "ValidationError".to_string(),
                message: message.clone(),
                details: Some(details.clone()),
            },
            UltimoError::Unauthorized(msg) => ErrorResponse {
                error: "Unauthorized".to_string(),
                message: msg.clone(),
                details: None,
            },
            UltimoError::Forbidden(msg) => ErrorResponse {
                error: "Forbidden".to_string(),
                message: msg.clone(),
                details: None,
            },
            UltimoError::NotFound(msg) => ErrorResponse {
                error: "NotFound".to_string(),
                message: msg.clone(),
                details: None,
            },
            UltimoError::BadRequest(msg) => ErrorResponse {
                error: "BadRequest".to_string(),
                message: msg.clone(),
                details: None,
            },
            UltimoError::Internal(msg) => ErrorResponse {
                error: "InternalError".to_string(),
                message: msg.clone(),
                details: None,
            },
            UltimoError::Hyper(err) => ErrorResponse {
                error: "ServerError".to_string(),
                message: format!("HTTP server error: {}", err),
                details: None,
            },
            UltimoError::HttpError(err) => ErrorResponse {
                error: "ServerError".to_string(),
                message: format!("HTTP error: {}", err),
                details: None,
            },
            UltimoError::Json(err) => ErrorResponse {
                error: "JsonError".to_string(),
                message: format!("JSON parsing error: {}", err),
                details: None,
            },
            UltimoError::Io(err) => ErrorResponse {
                error: "IoError".to_string(),
                message: format!("IO error: {}", err),
                details: None,
            },
        }
    }
}

impl fmt::Display for UltimoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UltimoError::Http { status, message } => {
                write!(f, "HTTP {}: {}", status, message)
            }
            UltimoError::Validation { message, .. } => write!(f, "Validation error: {}", message),
            UltimoError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            UltimoError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            UltimoError::NotFound(msg) => write!(f, "Not found: {}", msg),
            UltimoError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            UltimoError::Internal(msg) => write!(f, "Internal error: {}", msg),
            UltimoError::Hyper(err) => write!(f, "Hyper error: {}", err),
            UltimoError::HttpError(err) => write!(f, "HTTP error: {}", err),
            UltimoError::Json(err) => write!(f, "JSON error: {}", err),
            UltimoError::Io(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for UltimoError {}

// Conversions from common error types
impl From<hyper::Error> for UltimoError {
    fn from(err: hyper::Error) -> Self {
        UltimoError::Hyper(err)
    }
}

impl From<hyper::http::Error> for UltimoError {
    fn from(err: hyper::http::Error) -> Self {
        UltimoError::HttpError(err)
    }
}

impl From<serde_json::Error> for UltimoError {
    fn from(err: serde_json::Error) -> Self {
        UltimoError::Json(err)
    }
}

impl From<std::io::Error> for UltimoError {
    fn from(err: std::io::Error) -> Self {
        UltimoError::Io(err)
    }
}

/// Result type alias for Ultimo operations
pub type Result<T> = std::result::Result<T, UltimoError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(UltimoError::Unauthorized("test".into()).status_code(), 401);
        assert_eq!(UltimoError::Forbidden("test".into()).status_code(), 403);
        assert_eq!(UltimoError::NotFound("test".into()).status_code(), 404);
        assert_eq!(UltimoError::BadRequest("test".into()).status_code(), 400);
        assert_eq!(UltimoError::Internal("test".into()).status_code(), 500);
    }

    #[test]
    fn test_error_response_format() {
        let err = UltimoError::Validation {
            message: "Invalid input".to_string(),
            details: vec![ValidationError {
                field: "email".to_string(),
                message: "Invalid email format".to_string(),
            }],
        };

        let response = err.to_error_response();
        assert_eq!(response.error, "ValidationError");
        assert_eq!(response.message, "Invalid input");
        assert!(response.details.is_some());
        assert_eq!(response.details.unwrap().len(), 1);
    }

    #[test]
    fn test_http_error_status_code() {
        let err = UltimoError::Http {
            status: 418,
            message: "I'm a teapot".to_string(),
        };
        assert_eq!(err.status_code(), 418);
    }

    #[test]
    fn test_validation_error_status_code() {
        let err = UltimoError::Validation {
            message: "Validation failed".to_string(),
            details: vec![],
        };
        assert_eq!(err.status_code(), 400);
    }

    #[test]
    fn test_error_conversions() {
        // Test JSON error conversion
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json");
        assert!(json_err.is_err());
        let ultimo_err = UltimoError::from(json_err.unwrap_err());
        assert_eq!(ultimo_err.status_code(), 400);

        // Test IO error conversion
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let ultimo_err = UltimoError::from(io_err);
        assert_eq!(ultimo_err.status_code(), 500);
    }

    #[test]
    fn test_error_display_formatting() {
        let err = UltimoError::NotFound("User not found".to_string());
        assert_eq!(format!("{}", err), "Not found: User not found");

        let err = UltimoError::Unauthorized("Invalid token".to_string());
        assert_eq!(format!("{}", err), "Unauthorized: Invalid token");

        let err = UltimoError::Http {
            status: 503,
            message: "Service unavailable".to_string(),
        };
        assert_eq!(format!("{}", err), "HTTP 503: Service unavailable");
    }

    #[test]
    fn test_all_error_response_types() {
        // Test Http error response
        let err = UltimoError::Http {
            status: 500,
            message: "Server error".to_string(),
        };
        let response = err.to_error_response();
        assert_eq!(response.error, "HttpError");
        assert_eq!(response.message, "Server error");

        // Test Unauthorized error response
        let err = UltimoError::Unauthorized("No token".to_string());
        let response = err.to_error_response();
        assert_eq!(response.error, "Unauthorized");
        assert_eq!(response.message, "No token");

        // Test Forbidden error response
        let err = UltimoError::Forbidden("Access denied".to_string());
        let response = err.to_error_response();
        assert_eq!(response.error, "Forbidden");
        assert_eq!(response.message, "Access denied");

        // Test NotFound error response
        let err = UltimoError::NotFound("Resource not found".to_string());
        let response = err.to_error_response();
        assert_eq!(response.error, "NotFound");
        assert_eq!(response.message, "Resource not found");

        // Test BadRequest error response
        let err = UltimoError::BadRequest("Invalid data".to_string());
        let response = err.to_error_response();
        assert_eq!(response.error, "BadRequest");
        assert_eq!(response.message, "Invalid data");

        // Test Internal error response
        let err = UltimoError::Internal("Database failure".to_string());
        let response = err.to_error_response();
        assert_eq!(response.error, "InternalError");
        assert_eq!(response.message, "Database failure");
    }

    #[test]
    fn test_validation_error_with_multiple_fields() {
        let err = UltimoError::Validation {
            message: "Multiple validation errors".to_string(),
            details: vec![
                ValidationError {
                    field: "email".to_string(),
                    message: "Invalid format".to_string(),
                },
                ValidationError {
                    field: "password".to_string(),
                    message: "Too short".to_string(),
                },
            ],
        };

        let response = err.to_error_response();
        assert_eq!(response.error, "ValidationError");
        let details = response.details.unwrap();
        assert_eq!(details.len(), 2);
        assert_eq!(details[0].field, "email");
        assert_eq!(details[1].field, "password");
    }

    #[test]
    fn test_json_error_conversion_and_response() {
        let json_err = serde_json::from_str::<serde_json::Value>("not valid json");
        let ultimo_err = UltimoError::from(json_err.unwrap_err());
        let response = ultimo_err.to_error_response();

        assert_eq!(response.error, "JsonError");
        assert!(response.message.contains("JSON parsing error"));
        assert!(response.details.is_none());
    }

    #[test]
    fn test_io_error_conversion_and_response() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let ultimo_err = UltimoError::from(io_err);
        let response = ultimo_err.to_error_response();

        assert_eq!(response.error, "IoError");
        assert!(response.message.contains("IO error"));
        assert!(response.details.is_none());
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse {
            error: "TestError".to_string(),
            message: "Test message".to_string(),
            details: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("TestError"));
        assert!(json.contains("Test message"));
    }

    #[test]
    fn test_validation_error_serialization() {
        let validation_err = ValidationError {
            field: "username".to_string(),
            message: "Required field".to_string(),
        };

        let json = serde_json::to_string(&validation_err).unwrap();
        assert!(json.contains("username"));
        assert!(json.contains("Required field"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send<T: Send>() {}

        // UltimoError should be Send but not necessarily Sync
        assert_send::<UltimoError>();
    }
}
