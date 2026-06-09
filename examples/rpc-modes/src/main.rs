use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use ultimo::prelude::*;
use ultimo::rpc::{RpcMode, TS};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize, Serialize, TS)]
struct EmptyParams {}

#[derive(Debug, Deserialize, Serialize, TS)]
struct GetUserInput {
    id: u32,
}

#[derive(Debug, Deserialize, Serialize, TS)]
struct CreateUserInput {
    name: String,
    email: String,
}

#[derive(Debug, Serialize, TS)]
struct UserListResponse {
    users: Vec<User>,
    total: usize,
}

type UserStore = Arc<Mutex<Vec<User>>>;

#[tokio::main]
async fn main() -> ultimo::Result<()> {
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

    // ============================================
    // REST MODE EXAMPLE
    // ============================================
    println!("🌐 Starting REST Mode Example");
    println!("Individual endpoints with GET/POST methods");
    println!();

    let mut rest_app = Ultimo::new();

    // Create RPC registry in REST mode
    let rest_rpc = RpcRegistry::new_with_mode(RpcMode::Rest);

    // Register query (will use GET in REST mode)
    let users_list = users.clone();
    rest_rpc.query("listUsers", move |_input: EmptyParams| {
        let users = users_list.clone();
        async move {
            let users_data = users.lock().unwrap().clone();
            Ok(UserListResponse {
                total: users_data.len(),
                users: users_data,
            })
        }
    });

    // Register another query
    let users_get = users.clone();
    rest_rpc.query("getUserById", move |input: GetUserInput| {
        let users = users_get.clone();
        async move {
            let user = users
                .lock()
                .unwrap()
                .iter()
                .find(|u| u.id == input.id)
                .cloned()
                .ok_or_else(|| UltimoError::NotFound("User not found".to_string()))?;
            Ok(user)
        }
    });

    // Register mutation (will use POST in REST mode)
    let users_create = users.clone();
    rest_rpc.mutation("createUser", move |input: CreateUserInput| {
        let users = users_create.clone();
        async move {
            let mut users_guard = users.lock().unwrap();
            let new_id = users_guard.iter().map(|u| u.id).max().unwrap_or(0) + 1;
            let new_user = User {
                id: new_id,
                name: input.name,
                email: input.email,
            };
            users_guard.push(new_user.clone());
            Ok(new_user)
        }
    });

    // Generate REST client
    rest_rpc.generate_client_file("ultimo-client-rest.ts")?;
    println!("✅ REST client generated: ultimo-client-rest.ts");
    println!("   - GET  /api/listUsers");
    println!("   - GET  /api/getUserById?id=1");
    println!("   - POST /api/createUser");
    println!();

    // Mount REST endpoints manually (in a real app, this would be automated)
    let rpc_for_list = rest_rpc.clone();
    rest_app.get("/api/listUsers", move |ctx: Context| {
        let rpc = rpc_for_list.clone();
        async move {
            let params = serde_json::json!({});
            let result = rpc.call("listUsers", params).await?;
            ctx.json(result).await
        }
    });

    let rpc_for_create = rest_rpc.clone();
    rest_app.post("/api/createUser", move |ctx: Context| {
        let rpc = rpc_for_create.clone();
        async move {
            let body: CreateUserInput = ctx.req.json().await?;
            let result = rpc.call("createUser", serde_json::to_value(body)?).await?;
            ctx.json(result).await
        }
    });

    println!("REST Mode: Would listen on http://localhost:3001");
    println!("  GET  /api/listUsers");
    println!("  POST /api/createUser");
    println!();

    // ============================================
    // JSON-RPC MODE EXAMPLE (Current/Default)
    // ============================================
    println!("⚡ Starting JSON-RPC Mode Example");
    println!("Single endpoint with method dispatch");
    println!();

    let mut jsonrpc_app = Ultimo::new();

    // Create RPC registry in JSON-RPC mode (default)
    let jsonrpc_rpc = RpcRegistry::new(); // or new_with_mode(RpcMode::JsonRpc)

    // Register procedures (mode doesn't matter for registration in JsonRpc mode)
    let users_list = users.clone();
    jsonrpc_rpc.query("listUsers", move |_input: EmptyParams| {
        let users = users_list.clone();
        async move {
            let users_data = users.lock().unwrap().clone();
            Ok(UserListResponse {
                total: users_data.len(),
                users: users_data,
            })
        }
    });

    let users_get = users.clone();
    jsonrpc_rpc.query("getUserById", move |input: GetUserInput| {
        let users = users_get.clone();
        async move {
            let user = users
                .lock()
                .unwrap()
                .iter()
                .find(|u| u.id == input.id)
                .cloned()
                .ok_or_else(|| UltimoError::NotFound("User not found".to_string()))?;
            Ok(user)
        }
    });

    let users_create = users.clone();
    jsonrpc_rpc.mutation("createUser", move |input: CreateUserInput| {
        let users = users_create.clone();
        async move {
            let mut users_guard = users.lock().unwrap();
            let new_id = users_guard.iter().map(|u| u.id).max().unwrap_or(0) + 1;
            let new_user = User {
                id: new_id,
                name: input.name,
                email: input.email,
            };
            users_guard.push(new_user.clone());
            Ok(new_user)
        }
    });

    // Generate JSON-RPC client
    jsonrpc_rpc.generate_client_file("ultimo-client-jsonrpc.ts")?;
    println!("✅ JSON-RPC client generated: ultimo-client-jsonrpc.ts");
    println!("   - POST /rpc (all methods)");
    println!();

    // Mount JSON-RPC endpoint
    let rpc_handler = jsonrpc_rpc.clone();
    jsonrpc_app.post("/rpc", move |ctx: Context| {
        let rpc = rpc_handler.clone();
        async move {
            let req: RpcRequest = ctx.req.json().await?;
            let result = rpc.call(&req.method, req.params).await?;
            ctx.json(result).await
        }
    });

    println!("JSON-RPC Mode: Would listen on http://localhost:3000");
    println!();

    // ============================================
    // COMPARISON
    // ============================================
    println!("📊 Comparison:");
    println!();
    println!("REST Mode:");
    println!("  ✅ Clear URLs in Network tab");
    println!("  ✅ HTTP caching works (GET requests)");
    println!("  ✅ RESTful conventions");
    println!("  ⚠️  More complex routing");
    println!();
    println!("JSON-RPC Mode:");
    println!("  ✅ Simple routing (one endpoint)");
    println!("  ✅ Easy batching");
    println!("  ✅ Single middleware point");
    println!("  ⚠️  All requests look same in Network tab");
    println!();
    println!("Choose based on your needs! 🎯");

    Ok(())
}
