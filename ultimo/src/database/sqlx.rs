//! SQLx integration for Ultimo
//!
//! Provides async database access with compile-time checked queries.
//!
//! # Features
//!
//! - Connection pooling
//! - PostgreSQL, MySQL, SQLite support
//! - Compile-time query verification
//! - Automatic migrations
//! - Transaction support
//!
//! # Example
//!
//! ```rust,ignore
//! use ultimo::prelude::*;
//! use ultimo::database::sqlx::SqlxPool;
//! use sqlx::FromRow;
//!
//! #[derive(Serialize, FromRow)]
//! struct User {
//!     id: i32,
//!     name: String,
//!     email: String,
//! }
//!
//! #[tokio::main]
//! async fn main() -> ultimo::Result<()> {
//!     let mut app = Ultimo::new();
//!     
//!     // Connect to database
//!     let pool = SqlxPool::connect("postgres://localhost/mydb").await?;
//!     app.with_sqlx(pool);
//!     
//!     // Query users
//!     app.get("/users", |ctx: Context| async move {
//!         let db = ctx.sqlx()?;
//!         let users = sqlx::query_as::<_, User>("SELECT id, name, email FROM users")
//!             .fetch_all(db)
//!             .await?;
//!         ctx.json(users).await
//!     });
//!     
//!     // Get user by ID
//!     app.get("/users/:id", |ctx: Context| async move {
//!         let id: i32 = ctx.req.param("id")?.parse()?;
//!         let db = ctx.sqlx()?;
//!         
//!         let user = sqlx::query_as::<_, User>(
//!             "SELECT id, name, email FROM users WHERE id = $1"
//!         )
//!         .bind(id)
//!         .fetch_one(db)
//!         .await?;
//!         
//!         ctx.json(user).await
//!     });
//!     
//!     // Create user
//!     app.post("/users", |mut ctx: Context| async move {
//!         #[derive(Deserialize)]
//!         struct CreateUser {
//!             name: String,
//!             email: String,
//!         }
//!         
//!         let input: CreateUser = ctx.req.json().await?;
//!         let db = ctx.sqlx()?;
//!         
//!         let user = sqlx::query_as::<_, User>(
//!             "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id, name, email"
//!         )
//!         .bind(&input.name)
//!         .bind(&input.email)
//!         .fetch_one(db)
//!         .await?;
//!         
//!         ctx.json(user).await
//!     });
//!     
//!     app.listen("127.0.0.1:3000").await
//! }
//! ```
//!
//! # Transactions
//!
//! ```rust,ignore
//! use ultimo::prelude::*;
//! use ultimo::database::sqlx::SqlxPool;
//!
//! app.post("/transfer", |mut ctx: Context| async move {
//!     let db = ctx.sqlx()?;
//!     
//!     // Start transaction
//!     let mut tx = db.begin().await?;
//!     
//!     // Execute queries in transaction
//!     sqlx::query("UPDATE accounts SET balance = balance - 100 WHERE id = $1")
//!         .bind(from_account)
//!         .execute(&mut *tx)
//!         .await?;
//!     
//!     sqlx::query("UPDATE accounts SET balance = balance + 100 WHERE id = $1")
//!         .bind(to_account)
//!         .execute(&mut *tx)
//!         .await?;
//!     
//!     // Commit transaction
//!     tx.commit().await?;
//!     
//!     ctx.json(json!({"success": true})).await
//! });
//! ```

use super::DatabaseError;

/// Type alias for SQLx Postgres pool
#[cfg(feature = "sqlx-postgres")]
pub type PgPool = sqlx::PgPool;

/// Type alias for SQLx MySQL pool
#[cfg(feature = "sqlx-mysql")]
pub type MySqlPool = sqlx::MySqlPool;

/// Type alias for SQLx SQLite pool
#[cfg(feature = "sqlx-sqlite")]
pub type SqlitePool = sqlx::SqlitePool;

/// SQLx database pool wrapper
#[derive(Clone)]
pub struct SqlxPool<DB: sqlx::Database> {
    pool: sqlx::Pool<DB>,
}

impl<DB: sqlx::Database> SqlxPool<DB> {
    /// Create a new pool from an existing sqlx pool
    pub fn new(pool: sqlx::Pool<DB>) -> Self {
        Self { pool }
    }

    /// Get a reference to the underlying pool
    pub fn pool(&self) -> &sqlx::Pool<DB> {
        &self.pool
    }

    /// Get the underlying pool (consuming self)
    pub fn into_inner(self) -> sqlx::Pool<DB> {
        self.pool
    }
}

#[cfg(feature = "sqlx-postgres")]
impl SqlxPool<sqlx::Postgres> {
    /// Connect to a PostgreSQL database
    pub async fn connect(url: &str) -> Result<Self, DatabaseError> {
        let pool = sqlx::PgPool::connect(url)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        Ok(Self::new(pool))
    }

    /// Connect with custom pool options
    pub async fn connect_with_options(
        options: sqlx::postgres::PgPoolOptions,
        url: &str,
    ) -> Result<Self, DatabaseError> {
        let pool = options
            .connect(url)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        Ok(Self::new(pool))
    }
}

#[cfg(feature = "sqlx-mysql")]
impl SqlxPool<sqlx::MySql> {
    /// Connect to a MySQL database
    pub async fn connect(url: &str) -> Result<Self, DatabaseError> {
        let pool = sqlx::MySqlPool::connect(url)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        Ok(Self::new(pool))
    }

    /// Connect with custom pool options
    pub async fn connect_with_options(
        options: sqlx::mysql::MySqlPoolOptions,
        url: &str,
    ) -> Result<Self, DatabaseError> {
        let pool = options
            .connect(url)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        Ok(Self::new(pool))
    }
}

#[cfg(feature = "sqlx-sqlite")]
impl SqlxPool<sqlx::Sqlite> {
    /// Connect to a SQLite database
    pub async fn connect(url: &str) -> Result<Self, DatabaseError> {
        let pool = sqlx::SqlitePool::connect(url)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        Ok(Self::new(pool))
    }

    /// Connect with custom pool options
    pub async fn connect_with_options(
        options: sqlx::sqlite::SqlitePoolOptions,
        url: &str,
    ) -> Result<Self, DatabaseError> {
        let pool = options
            .connect(url)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        Ok(Self::new(pool))
    }

    /// Check if the database connection is healthy
    pub async fn health_check(&self) -> Result<(), DatabaseError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;
        Ok(())
    }
}

/// Convert SQLx errors to DatabaseError
impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Database(e) => DatabaseError::Query(e.to_string()),
            sqlx::Error::PoolTimedOut => {
                DatabaseError::Pool("Connection pool timed out".to_string())
            }
            sqlx::Error::PoolClosed => DatabaseError::Pool("Connection pool is closed".to_string()),
            _ => DatabaseError::Query(err.to_string()),
        }
    }
}
