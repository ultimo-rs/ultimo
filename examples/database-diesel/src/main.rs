use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use ultimo::database::diesel::DieselPool;
use ultimo::prelude::*;

// Define the database schema
mod schema {
    diesel::table! {
        users (id) {
            id -> Int4,
            name -> Varchar,
            email -> Varchar,
        }
    }
}

use schema::users;

// User model
#[derive(Debug, Serialize, Deserialize, Queryable, Selectable)]
#[diesel(table_name = users)]
struct User {
    id: i32,
    name: String,
    email: String,
}

// Input for creating users
#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = users)]
struct CreateUserInput {
    name: String,
    email: String,
}

// Input for updating users
#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = users)]
struct UpdateUserInput {
    name: Option<String>,
    email: Option<String>,
}

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get database URL from environment or use default
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/ultimo_example".to_string());

    println!("🔌 Connecting to database: {}", database_url);

    // Create Diesel connection pool
    let pool = DieselPool::<diesel::PgConnection>::new(&database_url)
        .map_err(|e| UltimoError::Internal(format!("Failed to create pool: {}", e)))?;

    println!("✅ Database connected");

    // Run migrations (create table if not exists)
    {
        let mut conn = pool
            .get()
            .map_err(|e| UltimoError::Internal(format!("Failed to get connection: {}", e)))?;

        diesel::sql_query(
            "CREATE TABLE IF NOT EXISTS users (
                id SERIAL PRIMARY KEY,
                name VARCHAR NOT NULL,
                email VARCHAR NOT NULL UNIQUE
            )",
        )
        .execute(&mut *conn)
        .map_err(|e| UltimoError::Internal(format!("Migration failed: {}", e)))?;
    }

    println!("✅ Database migrations complete");

    // Create Ultimo app
    let mut app = Ultimo::new();
    app.with_diesel(pool);

    // Health check endpoint
    app.get("/health", |ctx: Context| async move {
        // Test database connection by getting a connection from the pool
        let _conn = ctx.diesel::<diesel::PgConnection>()?;

        ctx.json(serde_json::json!({
            "status": "healthy",
            "database": "connected"
        }))
        .await
    });

    // List all users
    app.get("/users", |ctx: Context| async move {
        let mut conn = ctx.diesel::<diesel::PgConnection>()?;

        let users_list = users::table
            .select(User::as_select())
            .load(&mut *conn)
            .map_err(|e| UltimoError::Internal(format!("Database query failed: {}", e)))?;

        ctx.json(serde_json::json!({
            "users": users_list,
            "total": users_list.len()
        }))
        .await
    });

    // Get user by ID
    app.get("/users/:id", |ctx: Context| async move {
        let id: i32 = ctx
            .req
            .param("id")?
            .parse()
            .map_err(|_| UltimoError::BadRequest("Invalid id format".to_string()))?;

        let mut conn = ctx.diesel::<diesel::PgConnection>()?;

        let user = users::table
            .find(id)
            .select(User::as_select())
            .first(&mut *conn)
            .optional()
            .map_err(|e| UltimoError::Internal(format!("Database query failed: {}", e)))?
            .ok_or_else(|| UltimoError::NotFound(format!("User with id {} not found", id)))?;

        ctx.json(user).await
    });

    // Create user
    app.post("/users", |ctx: Context| async move {
        let input: CreateUserInput = ctx.req.json().await?;
        let mut conn = ctx.diesel::<diesel::PgConnection>()?;

        let user = diesel::insert_into(users::table)
            .values(&input)
            .returning(User::as_returning())
            .get_result(&mut *conn)
            .map_err(|e| {
                if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                    UltimoError::BadRequest("Email already exists".to_string())
                } else {
                    UltimoError::Internal(format!("Database insert failed: {}", e))
                }
            })?;

        ctx.json(user).await
    });

    // Update user
    app.put("/users/:id", |ctx: Context| async move {
        let id: i32 = ctx
            .req
            .param("id")?
            .parse()
            .map_err(|_| UltimoError::BadRequest("Invalid id format".to_string()))?;

        let input: UpdateUserInput = ctx.req.json().await?;
        let mut conn = ctx.diesel::<diesel::PgConnection>()?;

        let user = diesel::update(users::table.find(id))
            .set(&input)
            .returning(User::as_returning())
            .get_result(&mut *conn)
            .optional()
            .map_err(|e| UltimoError::Internal(format!("Database update failed: {}", e)))?
            .ok_or_else(|| UltimoError::NotFound(format!("User with id {} not found", id)))?;

        ctx.json(user).await
    });

    // Delete user
    app.delete("/users/:id", |ctx: Context| async move {
        let id: i32 = ctx
            .req
            .param("id")?
            .parse()
            .map_err(|_| UltimoError::BadRequest("Invalid id format".to_string()))?;

        let mut conn = ctx.diesel::<diesel::PgConnection>()?;

        let deleted = diesel::delete(users::table.find(id))
            .execute(&mut *conn)
            .map_err(|e| UltimoError::Internal(format!("Database delete failed: {}", e)))?;

        if deleted == 0 {
            return Err(UltimoError::NotFound(format!(
                "User with id {} not found",
                id
            )));
        }

        ctx.status(204).await;
        ctx.text("").await
    });

    // Transaction example: Create multiple users atomically
    app.post("/users/batch", |ctx: Context| async move {
        let inputs: Vec<CreateUserInput> = ctx.req.json().await?;
        let mut conn = ctx.diesel::<diesel::PgConnection>()?;

        // Run in transaction
        let users_list = conn
            .transaction::<_, diesel::result::Error, _>(|conn| {
                let mut created_users = Vec::new();
                for input in inputs {
                    let user = diesel::insert_into(users::table)
                        .values(&input)
                        .returning(User::as_returning())
                        .get_result(conn)?;
                    created_users.push(user);
                }
                Ok(created_users)
            })
            .map_err(|e| {
                if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                    UltimoError::BadRequest("One or more emails already exist".to_string())
                } else {
                    UltimoError::Internal(format!("Transaction failed: {}", e))
                }
            })?;

        ctx.json(serde_json::json!({
            "created": users_list.len(),
            "users": users_list
        }))
        .await
    });

    println!("🚀 Server running on http://127.0.0.1:3001");
    println!();
    println!("Try these endpoints:");
    println!("  GET    http://127.0.0.1:3001/health");
    println!("  GET    http://127.0.0.1:3001/users");
    println!("  POST   http://127.0.0.1:3001/users");
    println!("  GET    http://127.0.0.1:3001/users/1");
    println!("  PUT    http://127.0.0.1:3001/users/1");
    println!("  DELETE http://127.0.0.1:3001/users/1");
    println!("  POST   http://127.0.0.1:3001/users/batch");
    println!();
    println!("Example POST:");
    println!("  curl -X POST http://127.0.0.1:3001/users \\");
    println!("    -H 'Content-Type: application/json' \\");
    println!("    -d '{{\"name\":\"Alice\",\"email\":\"alice@example.com\"}}'");
    println!();
    println!("Example batch POST:");
    println!("  curl -X POST http://127.0.0.1:3001/users/batch \\");
    println!("    -H 'Content-Type: application/json' \\");
    println!("    -d '[{{\"name\":\"Alice\",\"email\":\"alice@example.com\"}},{{\"name\":\"Bob\",\"email\":\"bob@example.com\"}}]'");

    app.listen("127.0.0.1:3001").await
}
