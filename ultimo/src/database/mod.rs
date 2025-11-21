//! Database integration for Ultimo
//!
//! Provides first-class support for SQLx and Diesel ORMs with connection pooling,
//! context integration, and automatic cleanup.
//!
//! # Features
//!
//! - Connection pooling for SQLx and Diesel
//! - Access database from Context: `ctx.db()`
//! - Automatic transaction management
//! - Health check endpoints
//! - Migration support
//!
//! # Example with SQLx
//!
//! ```rust,ignore
//! use ultimo::prelude::*;
//! use ultimo::database::sqlx::SqlxPool;
//!
//! #[tokio::main]
//! async fn main() -> ultimo::Result<()> {
//!     let mut app = Ultimo::new();
//!     
//!     // Initialize database pool
//!     let pool = SqlxPool::connect("postgres://localhost/mydb").await?;
//!     app.with_sqlx(pool.clone());
//!     
//!     // Use in handlers
//!     app.get("/users", |ctx: Context| async move {
//!         let db = ctx.sqlx()?;
//!         let users = sqlx::query_as::<_, User>("SELECT * FROM users")
//!             .fetch_all(db)
//!             .await?;
//!         ctx.json(users).await
//!     });
//!     
//!     app.listen("127.0.0.1:3000").await
//! }
//! ```
//!
//! # Example with Diesel
//!
//! ```rust,ignore
//! use ultimo::prelude::*;
//! use ultimo::database::diesel::DieselPool;
//!
//! #[tokio::main]
//! async fn main() -> ultimo::Result<()> {
//!     let mut app = Ultimo::new();
//!     
//!     // Initialize database pool
//!     let pool = DieselPool::new("postgres://localhost/mydb")?;
//!     app.with_diesel(pool.clone());
//!     
//!     // Use in handlers
//!     app.get("/users", |ctx: Context| async move {
//!         let conn = ctx.diesel()?;
//!         let users = users::table.load::<User>(&mut *conn)?;
//!         ctx.json(users).await
//!     });
//!     
//!     app.listen("127.0.0.1:3000").await
//! }
//! ```

#[cfg(feature = "sqlx")]
pub mod sqlx;

#[cfg(feature = "diesel")]
pub mod diesel;

mod error;

#[cfg(test)]
mod tests;

pub use error::DatabaseError;

use std::any::Any;
use std::sync::Arc;

/// Database connection stored in context
#[derive(Clone)]
pub enum Database {
    #[cfg(feature = "sqlx")]
    Sqlx(Arc<dyn Any + Send + Sync>),
    #[cfg(feature = "diesel")]
    Diesel(Arc<dyn Any + Send + Sync>),
}

impl Database {
    /// Create a new database connection wrapper from SQLx pool
    #[cfg(feature = "sqlx")]
    pub fn from_sqlx<DB: ::sqlx::Database>(pool: crate::database::sqlx::SqlxPool<DB>) -> Self {
        Self::Sqlx(Arc::new(pool))
    }

    /// Create a new database connection wrapper from Diesel pool
    #[cfg(feature = "diesel")]
    pub fn from_diesel<Conn>(pool: crate::database::diesel::DieselPool<Conn>) -> Self
    where
        Conn: ::diesel::Connection + ::diesel::r2d2::R2D2Connection + 'static,
    {
        Self::Diesel(Arc::new(pool))
    }

    /// Get SQLx pool reference
    #[cfg(feature = "sqlx")]
    pub fn as_sqlx<DB: ::sqlx::Database>(
        &self,
    ) -> Result<&crate::database::sqlx::SqlxPool<DB>, DatabaseError> {
        match self {
            Self::Sqlx(pool) => pool
                .downcast_ref::<crate::database::sqlx::SqlxPool<DB>>()
                .ok_or_else(|| DatabaseError::Connection("Invalid SQLx pool type".into())),
            #[allow(unreachable_patterns)]
            _ => Err(DatabaseError::NotConfigured),
        }
    }

    /// Get Diesel pool reference
    #[cfg(feature = "diesel")]
    pub fn as_diesel<Conn>(
        &self,
    ) -> Result<&crate::database::diesel::DieselPool<Conn>, DatabaseError>
    where
        Conn: ::diesel::Connection + ::diesel::r2d2::R2D2Connection + 'static,
    {
        match self {
            Self::Diesel(pool) => pool
                .downcast_ref::<crate::database::diesel::DieselPool<Conn>>()
                .ok_or_else(|| DatabaseError::Connection("Invalid Diesel pool type".into())),
            #[allow(unreachable_patterns)]
            _ => Err(DatabaseError::NotConfigured),
        }
    }
}
