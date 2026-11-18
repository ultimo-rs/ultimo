use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use ultimo::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUserInput {
    name: String,
    email: String,
}

type UserStore = Arc<Mutex<Vec<User>>>;

/// The key this demo accepts for writes. A real API would issue and check
/// keys with `ultimo::auth::api_key` (the `api-key` feature).
const DEMO_API_KEY: &str = "demo-key";

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    println!("🚀 Ultimo OpenAPI Demo - Regular REST Mode");
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

    // Writes (POST/DELETE) need the demo API key in `X-API-Key`. The OpenAPI
    // spec below documents this, so Swagger UI can send the key for you.
    app.use_middleware(
        ultimo::guard::Guard::new(|ctx: &Context| {
            let writes = matches!(ctx.req.method().as_str(), "POST" | "DELETE");
            let authorized = ctx.req.header("x-api-key").as_deref() == Some(DEMO_API_KEY);
            if writes && !authorized {
                ultimo::guard::Decision::respond(401, "Missing or wrong X-API-Key")
            } else {
                ultimo::guard::Decision::Allow
            }
        })
        .build(),
    );

    // Regular REST endpoints - no RPC
    let users_get = users.clone();
    app.get("/api/users/:id", move |ctx: Context| {
        let users = users_get.clone();
        async move {
            let id_str = ctx.req.param("id")?;
            let id: u32 = id_str
                .parse()
                .map_err(|_| UltimoError::BadRequest("Invalid 'id' parameter".to_string()))?;

            let user = {
                let users_data = users.lock().unwrap();
                users_data.iter().find(|u| u.id == id).cloned()
            };

            let user = user.ok_or_else(|| UltimoError::NotFound("User not found".to_string()))?;

            ctx.json(user).await
        }
    });

    let users_list = users.clone();
    app.get("/api/users", move |ctx: Context| {
        let users = users_list.clone();
        async move {
            let users_data = users.lock().unwrap().clone();
            ctx.json(users_data).await
        }
    });

    let users_create = users.clone();
    app.post("/api/users", move |ctx: Context| {
        let users = users_create.clone();
        async move {
            let input: CreateUserInput = ctx.req.json().await?;
            let new_user = {
                let mut users_data = users.lock().unwrap();
                let new_id = users_data.iter().map(|u| u.id).max().unwrap_or(0) + 1;
                let new_user = User {
                    id: new_id,
                    name: input.name,
                    email: input.email,
                };
                users_data.push(new_user.clone());
                new_user
            };
            ctx.json(new_user).await
        }
    });

    // DELETE /api/users/:id
    let users_delete = users.clone();
    app.delete("/api/users/:id", move |ctx: Context| {
        let users = users_delete.clone();
        async move {
            let id: u32 = ctx
                .req
                .param("id")?
                .parse()
                .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;

            {
                let mut users_data = users.lock().unwrap();
                let index = users_data
                    .iter()
                    .position(|u| u.id == id)
                    .ok_or_else(|| UltimoError::NotFound("User not found".to_string()))?;
                users_data.remove(index);
            }

            ctx.status(204).await;
            ctx.text("").await
        }
    });

    // Describe the REST endpoints with OpenAPI
    use ultimo::openapi::{
        OpenApiBuilder, Operation, Parameter, PathItem, RequestBody, Response, Schema,
        SecurityScheme,
    };

    println!("📋 Generating OpenAPI specification...");
    let server_url = std::env::var("BASE_URL").unwrap_or_else(|_| "/".to_string());

    let mut openapi = OpenApiBuilder::new()
        .title("User API - REST Mode")
        .version("1.0.0")
        .description("User management API using regular REST endpoints")
        .server(&server_url, Some("API server".to_string()))
        .tag("users", Some("User management operations".to_string()))
        // Writes need an API key. Declaring the scheme gives Swagger UI an
        // "Authorize" button; the guard above is what actually enforces it.
        .security_scheme(
            "apiKey",
            SecurityScheme::api_key_header("X-API-Key")
                .description(format!("Use `{DEMO_API_KEY}` in this demo.")),
        )
        .build();

    let user_schema = Schema::object([
        ("id", Schema::integer()),
        ("name", Schema::string()),
        ("email", Schema::string().format("email")),
    ]);
    openapi.add_schema("User", user_schema);
    let id_param = || Parameter::path("id", Schema::integer()).description("User ID");

    // GET /api/users/{id} + DELETE /api/users/{id}
    openapi.add_path(
        "/api/users/{id}",
        PathItem::new()
            .get(
                Operation::new()
                    .summary("Get user by ID")
                    .description("Retrieve a single user by their ID")
                    .operation_id("getUser")
                    .tag("users")
                    .parameter(id_param())
                    .response(
                        "200",
                        Response::json("User found", Schema::reference("User")),
                    )
                    .response("404", Response::new("User not found")),
            )
            .delete(
                Operation::new()
                    .summary("Delete user")
                    .description("Delete a user by ID. Requires an API key.")
                    .operation_id("deleteUser")
                    .tag("users")
                    .parameter(id_param())
                    .security("apiKey")
                    .response("204", Response::new("User deleted successfully"))
                    .response("401", Response::new("Missing or wrong API key"))
                    .response("404", Response::new("User not found")),
            ),
    );

    // GET /api/users + POST /api/users
    let mut create_body = RequestBody::json(Schema::object([
        ("name", Schema::string()),
        ("email", Schema::string().format("email")),
    ]))
    .description("User to create");
    if let Some(media) = create_body.content.get_mut("application/json") {
        media.example = Some(serde_json::json!({ "name": "Carol", "email": "carol@example.com" }));
    }
    openapi.add_path(
        "/api/users",
        PathItem::new()
            .get(
                Operation::new()
                    .summary("List all users")
                    .description("Retrieve a list of all users")
                    .operation_id("listUsers")
                    .tag("users")
                    .response(
                        "200",
                        Response::json("List of users", Schema::array(Schema::reference("User"))),
                    ),
            )
            .post(
                Operation::new()
                    .summary("Create user")
                    .description("Create a new user. Requires an API key.")
                    .operation_id("createUser")
                    .tag("users")
                    .security("apiKey")
                    .request_body(create_body)
                    .response(
                        "200",
                        Response::json("User created", Schema::reference("User")),
                    )
                    .response("401", Response::new("Missing or wrong API key")),
            ),
    );

    // Save OpenAPI spec
    openapi
        .write_to_file("openapi-rest.json")
        .expect("Failed to write OpenAPI spec");
    println!("✅ OpenAPI spec saved to: openapi-rest.json");
    println!();

    // Serve OpenAPI spec endpoint
    let openapi_clone = openapi.clone();
    app.get("/openapi.json", move |ctx: Context| {
        let spec = openapi_clone.clone();
        async move { ctx.json(spec).await }
    });

    // Root route — redirect to Swagger UI
    app.get("/", |ctx: Context| async move {
        ctx.html(r#"<!doctype html><html><head><meta http-equiv="refresh" content="0;url=/docs"></head><body>
<div style="background:#fef3c7;border:1px solid #f59e0b;border-radius:0.5rem;padding:0.5rem 1rem;margin-bottom:1rem;font-size:0.85rem;color:#92400e;">
⚡ <strong>Live demo</strong> — hosted on Render free tier. First request may take ~30s due to cold start.
Powered by <a href="https://github.com/ultimo-rs/ultimo" style="color:#92400e;font-weight:600;">Ultimo</a>.
</div>
<p>Redirecting to <a href="/docs">/docs</a>...</p></body></html>"#).await
    });

    // Serve Swagger UI at /docs
    let openapi_docs = openapi.clone();
    app.get("/docs", move |ctx: Context| {
        let html = openapi_docs.swagger_ui_html("/openapi.json");
        async move { ctx.html(html).await }
    });

    println!("🌐 Server starting on http://127.0.0.1:3000");
    println!();
    println!("Available endpoints:");
    println!("  GET  /api/users/:id");
    println!("  GET  /api/users");
    println!("  POST /api/users        (needs X-API-Key: {DEMO_API_KEY})");
    println!("  DELETE /api/users/:id  (needs X-API-Key: {DEMO_API_KEY})");
    println!();
    println!("📖 Interactive API Documentation:");
    println!("  Swagger UI: http://127.0.0.1:3000/docs");
    println!("  OpenAPI:    http://127.0.0.1:3000/openapi.json");
    println!();

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    app.listen(&addr).await
}
