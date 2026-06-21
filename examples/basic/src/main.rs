use std::sync::Arc;
use ultimo::middleware::BoxedMiddleware;
use ultimo::prelude::*;

#[derive(Deserialize, Validate)]
struct CreateUser {
    #[validate(length(min = 3, max = 50))]
    name: String,
    #[validate(email)]
    email: String,
}

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let mut app = Ultimo::new();

    // Global middleware
    app.use_middleware(middleware::builtin::logger());
    app.use_middleware(
        middleware::builtin::Cors::new()
            .allow_origin("*")
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .build(),
    );

    // Simple GET route
    app.get("/", |ctx: Context| async move {
        ctx.json(json!({
            "message": "Hello Ultimo!",
            "version": "0.1.0"
        }))
        .await
    });

    // Path parameters
    app.get("/users/:id", |ctx: Context| async move {
        let id = ctx.req.param("id")?;
        ctx.json(json!({
            "id": id,
            "name": "John Doe"
        }))
        .await
    });

    // Query parameters
    app.get("/search", |ctx: Context| async move {
        let query = ctx.req.query("q").unwrap_or_else(|| "".to_string());
        ctx.json(json!({
            "query": query,
            "results": []
        }))
        .await
    });

    // POST with validation
    app.post("/users", |ctx: Context| async move {
        let input: CreateUser = ctx.req.json().await?;
        validate(&input)?;

        let user = User {
            id: 1,
            name: input.name,
            email: input.email,
        };

        ctx.status(201).await;
        ctx.json(user).await
    });

    // Multiple path parameters
    app.get("/users/:userId/posts/:postId", |ctx: Context| async move {
        let user_id = ctx.req.param("userId")?;
        let post_id = ctx.req.param("postId")?;

        ctx.json(json!({
            "userId": user_id,
            "postId": post_id,
            "title": "Sample Post"
        }))
        .await
    });

    // Custom middleware example
    let custom_middleware: BoxedMiddleware = Arc::new(|ctx: Context, next| {
        Box::pin(async move {
            ctx.set("request_id", "12345").await;
            let result = next(ctx).await;
            result
        })
    });
    app.use_middleware(custom_middleware);

    // Route that uses middleware data
    app.get("/info", |ctx: Context| async move {
        let request_id = ctx
            .get("request_id")
            .await
            .unwrap_or_else(|| "none".to_string());
        ctx.json(json!({
            "requestId": request_id,
            "framework": "Ultimo"
        }))
        .await
    });

    // HTML response
    app.get("/html", |ctx: Context| async move {
        ctx.html(r#"<!doctype html><html><head><meta charset="utf-8"><title>Ultimo Basic Demo</title></head><body>
<div style="background:#fef3c7;border:1px solid #f59e0b;border-radius:0.5rem;padding:0.5rem 1rem;margin-bottom:1rem;font-size:0.85rem;color:#92400e;">
⚡ <strong>Live demo</strong> — hosted on Render free tier. First request may take ~30s due to cold start.
Powered by <a href="https://github.com/ultimo-rs/ultimo" style="color:#92400e;font-weight:600;">Ultimo</a>.
</div>
<h1>Hello from Ultimo!</h1><p>A fast, type-safe Rust web framework</p>
<h2>Try these endpoints:</h2>
<ul>
<li><a href="/users/42">/users/42</a> — path params</li>
<li><a href="/search?q=rust">/search?q=rust</a> — query params</li>
<li><a href="/json">/json</a> — JSON response</li>
</ul></body></html>"#)
            .await
    });

    // Text response
    app.get("/text", |ctx: Context| async move {
        ctx.text("Plain text response from Ultimo").await
    });

    // Redirect
    app.get("/redirect", |ctx: Context| async move {
        ctx.redirect("/").await
    });

    // Error handling example
    app.get("/error", |_ctx: Context| async move {
        Err(UltimoError::BadRequest(
            "This is an intentional error".to_string(),
        ))
    });

    println!("\n🚀 Starting Ultimo server...");
    println!("📝 Try these endpoints:");
    println!("   GET  http://localhost:3000/");
    println!("   GET  http://localhost:3000/users/123");
    println!("   GET  http://localhost:3000/search?q=rust");
    println!("   POST http://localhost:3000/users (JSON: {{\"name\":\"Alice\",\"email\":\"alice@example.com\"}})");
    println!("   GET  http://localhost:3000/html");
    println!("   GET  http://localhost:3000/redirect");
    println!();

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    app.listen(&addr).await
}
