use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use ultimo::prelude::*;
use ultimo::rpc::RpcMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GetUserInput {
    id: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct CreateUserInput {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct EmptyParams {}

#[derive(Debug, Deserialize, Serialize)]
struct UserListResponse {
    users: Vec<User>,
    total: usize,
}

type UserStore = Arc<Mutex<Vec<User>>>;

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    println!("🚀 Ultimo OpenAPI Demo - RPC Mode");
    println!();

    // Initialize user store
    let users: UserStore = Arc::new(Mutex::new(vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
        },
    ]));

    let mut app = Ultimo::new();

    // Add CORS middleware
    app.use_middleware(ultimo::middleware::builtin::cors());

    // Add logger middleware
    app.use_middleware(ultimo::middleware::builtin::logger());

    // Initialize RPC registry in JSON-RPC mode
    let rpc = RpcRegistry::new_with_mode(RpcMode::JsonRpc);

    // Register getUser
    let users_get = users.clone();
    rpc.query_with_types(
        "getUser",
        move |input: GetUserInput| {
            let users = users_get.clone();
            async move {
                let users_data = users.lock().unwrap();
                users_data
                    .iter()
                    .find(|u| u.id == input.id)
                    .cloned()
                    .ok_or_else(|| UltimoError::NotFound("User not found".to_string()))
            }
        },
        "{ id: number }".to_string(),
        "{ id: number; name: string; email: string }".to_string(),
    );

    // Register listUsers
    let users_list = users.clone();
    rpc.query_with_types(
        "listUsers",
        move |_input: EmptyParams| {
            let users = users_list.clone();
            async move {
                let users_data = users.lock().unwrap().clone();
                Ok(UserListResponse {
                    total: users_data.len(),
                    users: users_data,
                })
            }
        },
        "{}".to_string(),
        "{ users: User[]; total: number }".to_string(),
    );

    // Register createUser
    let users_create = users.clone();
    rpc.mutation_with_types(
        "createUser",
        move |input: CreateUserInput| {
            let users = users_create.clone();
            async move {
                let mut users_data = users.lock().unwrap();
                let new_id = users_data.iter().map(|u| u.id).max().unwrap_or(0) + 1;
                let new_user = User {
                    id: new_id,
                    name: input.name,
                    email: input.email,
                };
                users_data.push(new_user.clone());
                Ok(new_user)
            }
        },
        "{ name: string; email: string }".to_string(),
        "{ id: number; name: string; email: string }".to_string(),
    );

    // Generate OpenAPI spec from RPC registry
    println!("📋 Generating OpenAPI specification...");
    let openapi = rpc.generate_openapi("User API - RPC Mode", "1.0.0", "/api");

    // Save OpenAPI spec to file
    openapi
        .write_to_file("openapi-rpc.json")
        .expect("Failed to write OpenAPI spec");
    println!("✅ OpenAPI spec saved to: openapi-rpc.json");
    println!();

    // Serve OpenAPI spec endpoint
    let openapi_clone = openapi.clone();
    app.get("/openapi.json", move |ctx: Context| {
        let spec = openapi_clone.clone();
        async move { ctx.json(spec).await }
    });

    // Serve Swagger UI at /docs
    let openapi_docs = openapi.clone();
    app.get("/docs", move |ctx: Context| {
        let html = openapi_docs.swagger_ui_html("/openapi.json");
        async move { ctx.html(html).await }
    });

    // Mount JSON-RPC endpoint (supports single, batch, notifications, and legacy format)
    let rpc_handler = rpc.clone();
    app.post("/api", move |ctx: Context| {
        let rpc = rpc_handler.clone();
        async move {
            let body = ctx.req.bytes().await?;
            let output = rpc.handle_request(&body).await;
            match output.into_body() {
                Some(bytes) => {
                    let value: serde_json::Value = serde_json::from_slice(&bytes)
                        .map_err(|e| UltimoError::Internal(e.to_string()))?;
                    ctx.json(value).await
                }
                None => {
                    ctx.status(204).await;
                    ctx.text("").await
                }
            }
        }
    });

    println!("🌐 Server starting on http://127.0.0.1:3000");
    println!();
    println!("Available endpoints:");
    println!("  POST /api (JSON-RPC 2.0)");
    println!();
    println!("📖 Interactive API Documentation:");
    println!("  Swagger UI: http://127.0.0.1:3000/docs");
    println!("  OpenAPI:    http://127.0.0.1:3000/openapi.json");
    println!();

    app.listen("127.0.0.1:3000").await
}
