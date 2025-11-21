//! Diesel integration for Ultimo
//!
//! Provides database access with Diesel ORM.
//!
//! # Features
//!
//! - Connection pooling with r2d2
//! - PostgreSQL, MySQL, SQLite support
//! - Schema management
//! - Type-safe queries
//! - Migration support
//!
//! # Example
//!
//! ```rust,ignore
//! use ultimo::prelude::*;
//! use ultimo::database::diesel::DieselPool;
//! use diesel::prelude::*;
//!
//! // Define schema
//! table! {
//!     users (id) {
//!         id -> Integer,
//!         name -> Text,
//!         email -> Text,
//!     }
//! }
//!
//! #[derive(Queryable, Serialize)]
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
//!     let pool = DieselPool::<diesel::PgConnection>::new("postgres://localhost/mydb")?;
//!     app.with_diesel(pool);
//!     
//!     // Query users
//!     app.get("/users", |ctx: Context| async move {
//!         let mut conn = ctx.diesel()?;
//!         let users = users::table
//!             .load::<User>(&mut *conn)
//!             .map_err(|e| UltimoError::Internal(e.to_string()))?;
//!         ctx.json(users).await
//!     });
//!     
//!     // Get user by ID
//!     app.get("/users/:id", |ctx: Context| async move {
//!         let id: i32 = ctx.req.param("id")?.parse()?;
//!         let mut conn = ctx.diesel()?;
//!         
//!         let user = users::table
//!             .find(id)
//!             .first::<User>(&mut *conn)
//!             .map_err(|e| UltimoError::Internal(e.to_string()))?;
//!         
//!         ctx.json(user).await
//!     });
//!     
//!     // Create user
//!     app.post("/users", |mut ctx: Context| async move {
//!         #[derive(Deserialize, Insertable)]
//!         #[diesel(table_name = users)]
//!         struct NewUser {
//!             name: String,
//!             email: String,
//!         }
//!         
//!         let input: NewUser = ctx.req.json().await?;
//!         let mut conn = ctx.diesel()?;
//!         
//!         let user = diesel::insert_into(users::table)
//!             .values(&input)
//!             .get_result::<User>(&mut *conn)
//!             .map_err(|e| UltimoError::Internal(e.to_string()))?;
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
//! use ultimo::database::diesel::DieselPool;
//! use diesel::prelude::*;
//!
//! app.post("/transfer", |mut ctx: Context| async move {
//!     let mut conn = ctx.diesel()?;
//!     
//!     // Execute in transaction
//!     conn.transaction::<_, diesel::result::Error, _>(|conn| {
//!         diesel::update(accounts::table.find(from_account))
//!             .set(accounts::balance.eq(accounts::balance - 100))
//!             .execute(conn)?;
//!         
//!         diesel::update(accounts::table.find(to_account))
//!             .set(accounts::balance.eq(accounts::balance + 100))
//!             .execute(conn)?;
//!         
//!         Ok(())
//!     })
//!     .map_err(|e| UltimoError::Internal(e.to_string()))?;
//!     
//!     ctx.json(json!({"success": true})).await
//! });
//! ```

use super::DatabaseError;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

/// Diesel connection pool
#[derive(Clone)]
pub struct DieselPool<Conn>
where
    Conn: diesel::Connection + diesel::r2d2::R2D2Connection + 'static,
{
    pool: Pool<ConnectionManager<Conn>>,
}

impl<Conn> DieselPool<Conn>
where
    Conn: diesel::Connection + diesel::r2d2::R2D2Connection + 'static,
{
    /// Create a new Diesel pool
    pub fn new(database_url: &str) -> Result<Self, DatabaseError> {
        let manager = ConnectionManager::<Conn>::new(database_url);
        let pool = Pool::builder()
            .build(manager)
            .map_err(|e| DatabaseError::Pool(e.to_string()))?;
        Ok(Self { pool })
    }

    /// Create a pool with custom configuration
    pub fn with_config(database_url: &str, max_size: u32) -> Result<Self, DatabaseError> {
        let manager = ConnectionManager::<Conn>::new(database_url);
        let pool = Pool::builder()
            .max_size(max_size)
            .build(manager)
            .map_err(|e| DatabaseError::Pool(e.to_string()))?;
        Ok(Self { pool })
    }

    /// Get a connection from the pool
    pub fn get(&self) -> Result<PooledConnection<ConnectionManager<Conn>>, DatabaseError> {
        self.pool
            .get()
            .map_err(|e| DatabaseError::Pool(e.to_string()))
    }

    /// Get a reference to the underlying pool
    pub fn pool(&self) -> &Pool<ConnectionManager<Conn>> {
        &self.pool
    }

    /// Check if the database connection is healthy
    pub async fn health_check(&self) -> Result<(), DatabaseError> {
        // Get connection to verify pool is healthy
        let _conn = self.get()?;
        Ok(())
    }
}

/// Type aliases for common Diesel connection types
#[cfg(feature = "diesel-postgres")]
pub type PgPool = DieselPool<diesel::PgConnection>;

#[cfg(feature = "diesel-mysql")]
pub type MySqlPool = DieselPool<diesel::MysqlConnection>;

#[cfg(feature = "diesel-sqlite")]
pub type SqlitePool = DieselPool<diesel::SqliteConnection>;
