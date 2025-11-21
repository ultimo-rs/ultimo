// Database API Styles Example
//
// This example demonstrates that Ultimo's database integration works seamlessly
// with ANY API routing style. The same database logic works with:
// 1. Traditional REST (GET /users, POST /users, DELETE /users/:id)
// 2. JSON-RPC (single endpoint: POST /rpc)
// 3. RPC-style REST (multiple endpoints: GET /listUsers, POST /createUser)
//
// Key point: Database access via ctx.sqlx() and ctx.diesel() is AGNOSTIC
// to your API routing style. Choose the style that fits your use case!

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ultimo::database::sqlx::SqlxPool;
use ultimo::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
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

#[derive(Debug, Deserialize)]
struct UpdateUserInput {
    name: Option<String>,
    email: Option<String>,
}

// Shared database logic - works with ANY API style!
mod db_operations {
    use super::*;
    use sqlx::PgPool;

    pub async fn list_users(pool: &PgPool) -> ultimo::Result<Vec<User>> {
        sqlx::query_as::<_, User>("SELECT id, name, email FROM users ORDER BY id")
            .fetch_all(pool)
            .await
            .map_err(|e| UltimoError::Internal(format!("Database query failed: {}", e)))
    }

    pub async fn get_user(pool: &PgPool, id: i32) -> ultimo::Result<User> {
        sqlx::query_as::<_, User>("SELECT id, name, email FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| UltimoError::Internal(format!("Database query failed: {}", e)))?
            .ok_or_else(|| UltimoError::NotFound(format!("User with id {} not found", id)))
    }

    pub async fn create_user(pool: &PgPool, input: CreateUserInput) -> ultimo::Result<User> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id, name, email",
        )
        .bind(&input.name)
        .bind(&input.email)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") {
                UltimoError::BadRequest("Email already exists".to_string())
            } else {
                UltimoError::Internal(format!("Database insert failed: {}", e))
            }
        })
    }

    pub async fn update_user(
        pool: &PgPool,
        id: i32,
        input: UpdateUserInput,
    ) -> ultimo::Result<User> {
        let user = if let (Some(name), Some(email)) = (&input.name, &input.email) {
            sqlx::query_as::<_, User>(
                "UPDATE users SET name = $1, email = $2 WHERE id = $3 RETURNING id, name, email",
            )
            .bind(name)
            .bind(email)
            .bind(id)
            .fetch_optional(pool)
            .await
        } else if let Some(name) = &input.name {
            sqlx::query_as::<_, User>(
                "UPDATE users SET name = $1 WHERE id = $2 RETURNING id, name, email",
            )
            .bind(name)
            .bind(id)
            .fetch_optional(pool)
            .await
        } else if let Some(email) = &input.email {
            sqlx::query_as::<_, User>(
                "UPDATE users SET email = $1 WHERE id = $2 RETURNING id, name, email",
            )
            .bind(email)
            .bind(id)
            .fetch_optional(pool)
            .await
        } else {
            return Err(UltimoError::BadRequest("No fields to update".to_string()));
        };

        user.map_err(|e| UltimoError::Internal(format!("Database update failed: {}", e)))?
            .ok_or_else(|| UltimoError::NotFound(format!("User with id {} not found", id)))
    }

    pub async fn delete_user(pool: &PgPool, id: i32) -> ultimo::Result<()> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| UltimoError::Internal(format!("Database delete failed: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(UltimoError::NotFound(format!(
                "User with id {} not found",
                id
            )));
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/ultimo_example".to_string());

    println!("🔌 Connecting to database: {}", database_url);

    let pool = SqlxPool::connect(&database_url)
        .await
        .map_err(|e| UltimoError::Internal(format!("Failed to create pool: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            email VARCHAR NOT NULL UNIQUE
        )",
    )
    .execute(pool.pool())
    .await
    .map_err(|e| UltimoError::Internal(format!("Migration failed: {}", e)))?;

    println!("✅ Database connected and migrated");
    println!();

    // ============================================
    // STYLE 1: TRADITIONAL REST
    // ============================================
    println!("🌐 Style 1: Traditional REST");
    println!("   - GET    /rest/users       - List all users");
    println!("   - GET    /rest/users/:id   - Get user by ID");
    println!("   - POST   /rest/users       - Create new user");
    println!("   - PUT    /rest/users/:id   - Update user");
    println!("   - DELETE /rest/users/:id   - Delete user");
    println!();

    let mut app = Ultimo::new();
    app.with_sqlx(pool);

    // Traditional REST endpoints
    app.get("/rest/users", |ctx: Context| async move {
        let db = ctx.sqlx::<sqlx::Postgres>()?;
        let users = db_operations::list_users(db).await?;
        ctx.json(users).await
    });

    app.get("/rest/users/:id", |ctx: Context| async move {
        let id: i32 = ctx
            .req
            .param("id")?
            .parse()
            .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;
        let user = db_operations::get_user(db, id).await?;
        ctx.json(user).await
    });

    app.post("/rest/users", |ctx: Context| async move {
        let input: CreateUserInput = ctx.req.json().await?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;
        let user = db_operations::create_user(db, input).await?;
        ctx.json(user).await
    });

    app.put("/rest/users/:id", |ctx: Context| async move {
        let id: i32 = ctx
            .req
            .param("id")?
            .parse()
            .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;
        let input: UpdateUserInput = ctx.req.json().await?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;
        let user = db_operations::update_user(db, id, input).await?;
        ctx.json(user).await
    });

    app.delete("/rest/users/:id", |ctx: Context| async move {
        let id: i32 = ctx
            .req
            .param("id")?
            .parse()
            .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;
        db_operations::delete_user(db, id).await?;
        ctx.status(204).await;
        ctx.text("").await
    });

    // ============================================
    // STYLE 2: JSON-RPC (Single Endpoint)
    // ============================================
    println!("🔌 Style 2: JSON-RPC (Single Endpoint)");
    println!("   - POST /rpc");
    println!("     Body: {{ \"method\": \"listUsers\", \"params\": {{}} }}");
    println!("     Body: {{ \"method\": \"getUser\", \"params\": {{ \"id\": 1 }} }}");
    println!("     Body: {{ \"method\": \"createUser\", \"params\": {{ \"name\": \"...\", \"email\": \"...\" }} }}");
    println!();

    #[derive(Deserialize)]
    struct RpcRequest {
        method: String,
        params: serde_json::Value,
    }

    app.post("/rpc", |ctx: Context| async move {
        let request: RpcRequest = ctx.req.json().await?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;

        let result: serde_json::Value = match request.method.as_str() {
            "listUsers" => {
                let users = db_operations::list_users(db).await?;
                serde_json::to_value(users)?
            }
            "getUser" => {
                #[derive(Deserialize)]
                struct GetUserParams {
                    id: i32,
                }
                let params: GetUserParams = serde_json::from_value(request.params)?;
                let user = db_operations::get_user(db, params.id).await?;
                serde_json::to_value(user)?
            }
            "createUser" => {
                let params: CreateUserInput = serde_json::from_value(request.params)?;
                let user = db_operations::create_user(db, params).await?;
                serde_json::to_value(user)?
            }
            "updateUser" => {
                #[derive(Deserialize)]
                struct UpdateUserParams {
                    id: i32,
                    #[serde(flatten)]
                    input: UpdateUserInput,
                }
                let params: UpdateUserParams = serde_json::from_value(request.params)?;
                let user = db_operations::update_user(db, params.id, params.input).await?;
                serde_json::to_value(user)?
            }
            "deleteUser" => {
                #[derive(Deserialize)]
                struct DeleteUserParams {
                    id: i32,
                }
                let params: DeleteUserParams = serde_json::from_value(request.params)?;
                db_operations::delete_user(db, params.id).await?;
                serde_json::json!({"success": true})
            }
            _ => {
                return Err(UltimoError::NotFound(format!(
                    "Method '{}' not found",
                    request.method
                )));
            }
        };

        ctx.json(result).await
    });

    // ============================================
    // STYLE 3: RPC-Style REST (Multiple Endpoints)
    // ============================================
    println!("🎯 Style 3: RPC-Style REST");
    println!("   - GET  /rpc-rest/listUsers");
    println!("   - GET  /rpc-rest/getUser?id=1");
    println!("   - POST /rpc-rest/createUser");
    println!("   - POST /rpc-rest/updateUser");
    println!("   - POST /rpc-rest/deleteUser");
    println!();

    app.get("/rpc-rest/listUsers", |ctx: Context| async move {
        let db = ctx.sqlx::<sqlx::Postgres>()?;
        let users = db_operations::list_users(db).await?;
        ctx.json(users).await
    });

    app.get("/rpc-rest/getUser", |ctx: Context| async move {
        let id: i32 = ctx
            .req
            .query("id")
            .ok_or_else(|| UltimoError::BadRequest("Missing 'id' query parameter".to_string()))?
            .parse()
            .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;
        let user = db_operations::get_user(db, id).await?;
        ctx.json(user).await
    });

    app.post("/rpc-rest/createUser", |ctx: Context| async move {
        let input: CreateUserInput = ctx.req.json().await?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;
        let user = db_operations::create_user(db, input).await?;
        ctx.json(user).await
    });

    app.post("/rpc-rest/updateUser", |ctx: Context| async move {
        #[derive(Deserialize)]
        struct UpdateRequest {
            id: i32,
            #[serde(flatten)]
            input: UpdateUserInput,
        }
        let request: UpdateRequest = ctx.req.json().await?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;
        let user = db_operations::update_user(db, request.id, request.input).await?;
        ctx.json(user).await
    });

    app.post("/rpc-rest/deleteUser", |ctx: Context| async move {
        #[derive(Deserialize)]
        struct DeleteRequest {
            id: i32,
        }
        let request: DeleteRequest = ctx.req.json().await?;
        let db = ctx.sqlx::<sqlx::Postgres>()?;
        db_operations::delete_user(db, request.id).await?;
        ctx.json(serde_json::json!({"success": true})).await
    });

    // ============================================
    // Documentation Page
    // ============================================
    app.get("/", |ctx: Context| async move {
        let html = include_str!("../index.html");
        ctx.html(html).await
    });

    println!("🚀 Server running on http://127.0.0.1:3003");
    println!("📚 Documentation: http://127.0.0.1:3003/");
    println!();
    println!("💡 The same database operations work with ALL three API styles!");

    app.listen("127.0.0.1:3003").await
}
