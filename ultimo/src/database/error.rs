//! Database error types

use std::fmt;

/// Database-specific errors
#[derive(Debug)]
pub enum DatabaseError {
    /// Connection error
    Connection(String),

    /// Query execution error
    Query(String),

    /// Transaction error
    Transaction(String),

    /// Pool error
    Pool(String),

    /// Migration error
    Migration(String),

    /// Database not configured
    NotConfigured,
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection(msg) => write!(f, "Database connection error: {}", msg),
            Self::Query(msg) => write!(f, "Database query error: {}", msg),
            Self::Transaction(msg) => write!(f, "Database transaction error: {}", msg),
            Self::Pool(msg) => write!(f, "Database pool error: {}", msg),
            Self::Migration(msg) => write!(f, "Database migration error: {}", msg),
            Self::NotConfigured => write!(
                f,
                "Database not configured. Use app.with_sqlx() or app.with_diesel()"
            ),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<DatabaseError> for crate::UltimoError {
    fn from(err: DatabaseError) -> Self {
        crate::UltimoError::Internal(err.to_string())
    }
}
