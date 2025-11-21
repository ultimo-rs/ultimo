use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ultimo::database::sqlx::SqlxPool;
use ultimo::prelude::*;

#[derive(Debug, Serialize, FromRow)]
struct User {
    id: i32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUserInput {
    name: String,
    email: String,
}

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let mut app = Ultimo::new();

    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/ultimo_example".to_string());

    println!("🔌 Connecting to database: {}", database_url);

    let pool = SqlxPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    println!("✅ Database connected");

    // Run migrations
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            email VARCHAR(255) NOT NULL UNIQUE
        )
        "#,
    )
    .execute(pool.pool())
    .await
    .expect("Failed to create table");

    println!("✅ Database migrations complete");

    // Attach database to app
    app.with_sqlx(pool);

    // Add CORS middleware
    app.use_middleware(ultimo::middleware::builtin::cors());

    // Health check endpoint
    app.get("/health", |ctx: Context| async move {
        let db = ctx.sqlx::<sqlx::Postgres>()?;

        // Check database connection
        sqlx::query("SELECT 1")
            .execute(db)
            .await
            .map_err(|e| UltimoError::Internal(format!("Database health check failed: {}", e)))?;

        ctx.json(json!({
            "status": "healthy",
            "database": "connected"
        }))
        .await
    });

    // List all users
    app.get("/users", |ctx: Context| async move {
        let db = ctx.sqlx::<sqlx::Postgres>()?;

        let users = sqlx::query_as::<_, User>("SELECT id, name, email FROM users ORDER BY id")
            .fetch_all(db)
            .await
            .map_err(|e| UltimoError::Internal(format!("Database query failed: {}", e)))?;

        ctx.json(json!({
            "users": users,
            "total": users.len()
        }))
        .await
    });

    // Get user by ID
    app.get("/users/:id", |ctx: Context| async move {
        let id: i32 = ctx
            .req
            .param("id")?
            .parse()
            .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;

        let user = sqlx::query_as::<_, User>("SELECT id, name, email FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await
            .map_err(|e| UltimoError::Internal(format!("Database query failed: {}", e)))?;

        match user {
            Some(user) => ctx.json(user).await,
            None => Err(UltimoError::NotFound("User not found".to_string())),
        }
    });

    // Create user
    app.post("/users", |ctx: Context| async move {
        let input: CreateUserInput = ctx.req.json().await?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;

        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id, name, email",
        )
        .bind(&input.name)
        .bind(&input.email)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") {
                UltimoError::BadRequest("Email already exists".to_string())
            } else {
                UltimoError::Internal(format!("Database query failed: {}", e))
            }
        })?;

        ctx.status(201).await;
        ctx.json(user).await
    });

    // Update user
    app.put("/users/:id", |ctx: Context| async move {
        let id: i32 = ctx
            .req
            .param("id")?
            .parse()
            .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;
        let input: CreateUserInput = ctx.req.json().await?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;

        let user = sqlx::query_as::<_, User>(
            "UPDATE users SET name = $1, email = $2 WHERE id = $3 RETURNING id, name, email",
        )
        .bind(&input.name)
        .bind(&input.email)
        .bind(id)
        .fetch_optional(db)
        .await
        .map_err(|e| UltimoError::Internal(format!("Database query failed: {}", e)))?;

        match user {
            Some(user) => ctx.json(user).await,
            None => Err(UltimoError::NotFound("User not found".to_string())),
        }
    });

    // Delete user
    app.delete("/users/:id", |ctx: Context| async move {
        let id: i32 = ctx
            .req
            .param("id")?
            .parse()
            .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;

        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(db)
            .await
            .map_err(|e| UltimoError::Internal(format!("Database query failed: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(UltimoError::NotFound("User not found".to_string()));
        }

        ctx.status(204).await;
        ctx.text("").await
    });

    // Transaction example: Transfer (multi-step operation)
    app.post("/transfer", |ctx: Context| async move {
        #[derive(Deserialize)]
        struct TransferRequest {
            from_user_id: i32,
            to_user_id: i32,
            amount: i32,
        }

        let input: TransferRequest = ctx.req.json().await?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;

        // Start transaction
        let mut tx = db
            .begin()
            .await
            .map_err(|e| UltimoError::Internal(format!("Failed to start transaction: {}", e)))?;

        // This is a simplified example - in production you'd have a credits/balance column
        // For demo, we'll just verify both users exist
        let from_user =
            sqlx::query_as::<_, User>("SELECT id, name, email FROM users WHERE id = $1")
                .bind(input.from_user_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| UltimoError::Internal(e.to_string()))?
                .ok_or_else(|| UltimoError::NotFound("From user not found".to_string()))?;

        let to_user = sqlx::query_as::<_, User>("SELECT id, name, email FROM users WHERE id = $1")
            .bind(input.to_user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| UltimoError::Internal(e.to_string()))?
            .ok_or_else(|| UltimoError::NotFound("To user not found".to_string()))?;

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| UltimoError::Internal(format!("Failed to commit transaction: {}", e)))?;

        ctx.json(json!({
            "success": true,
            "from": from_user,
            "to": to_user,
            "amount": input.amount
        }))
        .await
    });

    println!("🚀 Server running on http://127.0.0.1:3000");
    println!();
    println!("Try these endpoints:");
    println!("  GET    http://127.0.0.1:3000/health");
    println!("  GET    http://127.0.0.1:3000/users");
    println!("  POST   http://127.0.0.1:3000/users");
    println!("  GET    http://127.0.0.1:3000/users/1");
    println!("  PUT    http://127.0.0.1:3000/users/1");
    println!("  DELETE http://127.0.0.1:3000/users/1");
    println!();
    println!("Example POST:");
    println!("  curl -X POST http://127.0.0.1:3000/users \\");
    println!("    -H 'Content-Type: application/json' \\");
    println!("    -d '{{\"name\":\"Alice\",\"email\":\"alice@example.com\"}}'");
    println!();

    app.listen("127.0.0.1:3000").await
}
