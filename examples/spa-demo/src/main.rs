//! SPA Demo — serves a minimal Single Page Application from `./dist`
//! with gzip/brotli compression and an API route.
//!
//! Run with:  cargo run -p spa-demo
//! Then open: http://127.0.0.1:3000
//!
//! Routes:
//!   GET /api/hello          → JSON {"message":"Hello from Ultimo!"}
//!   GET /assets/*           → static files served from ./dist
//!   GET <anything else>     → ./dist/index.html  (SPA client-side routing)

use ultimo::middleware::builtin::{compression, logger};
use ultimo::prelude::*;

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    tracing_subscriber::fmt::init();

    let mut app = Ultimo::new();

    // Global middleware: log every request, then compress the response.
    app.use_middleware(logger());
    app.use_middleware(compression());

    // API route — registered before SPA fallback so it takes precedence.
    app.get("/api/hello", |ctx: Context| async move {
        ctx.json(serde_json::json!({ "message": "Hello from Ultimo!" }))
            .await
    });

    // Serve static assets (JS, CSS, images) from ./dist under /assets.
    app.serve_static("/assets", "./dist");

    // SPA fallback: any unmatched GET returns ./dist/index.html so the
    // client-side router can handle navigation.
    app.serve_spa("./dist", "index.html");

    println!("→  http://127.0.0.1:3000");
    app.listen("127.0.0.1:3000").await
}
