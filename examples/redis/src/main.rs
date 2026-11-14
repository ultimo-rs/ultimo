//! Two instances of one app share sessions and a rate limit through Redis, as
//! they would behind a load balancer.
//!
//! ```bash
//! docker run --rm -p 6379:6379 redis:7-alpine
//! REDIS_URL=redis://127.0.0.1:6379 cargo run -p redis-example
//! ```
//!
//! Open http://localhost:3000, log in, then open http://localhost:3001: the
//! session is already there. (Cookies ignore the port, so the browser sends the
//! same session cookie to both instances.)
use std::sync::Arc;
use ultimo::middleware::builtin::{RateLimitKey, RateLimiter};
use ultimo::prelude::*;
use ultimo::redis::{connect, RedisRateLimitStore, RedisSessionStore};
use ultimo::session::{session, SessionConfig};

const PAGE: &str = r#"<!doctype html>
<meta charset="utf-8"><title>Ultimo Redis demo</title>
<h1>Redis demo — instance <span id="port"></span></h1>
<p>
  <a href="http://localhost:3000">instance :3000</a> ·
  <a href="http://localhost:3001">instance :3001</a>
</p>
<p>
  <input id="name" placeholder="your name" value="ada">
  <button onclick="post('/login?name=' + encodeURIComponent(name.value))">Log in</button>
  <button onclick="post('/logout')">Log out</button>
</p>
<p id="me"></p>
<h2>Shared rate limit: 5 requests / 10 s across both instances</h2>
<p><button onclick="burst()">Send 8 requests (alternating :3000 / :3001)</button></p>
<pre id="burst"></pre>
<script>
document.getElementById('port').textContent = location.port;
async function me() {
  const body = await (await fetch('/me')).json();
  document.getElementById('me').textContent = body.name
    ? `Logged in as ${body.name} (session written by instance :${body.logged_in_on})`
    : 'Not logged in';
}
async function post(path) { await fetch(path, { method: 'POST' }); me(); }
async function burst() {
  const lines = [];
  for (let i = 0; i < 8; i++) {
    const port = i % 2 === 0 ? 3000 : 3001;
    const res = await fetch(`http://localhost:${port}/limited`);
    lines.push(`#${i + 1} → :${port}  ${res.status}`);
  }
  document.getElementById('burst').textContent = lines.join('\n');
}
me();
</script>"#;

fn app(conn: ultimo::redis::client::aio::ConnectionManager, port: u16) -> Ultimo {
    let mut app = Ultimo::new();
    // secure(false): the demo runs over plain http://localhost.
    app.use_middleware(session(
        RedisSessionStore::new(conn.clone()),
        SessionConfig::default().secure(false),
    ));

    app.get("/", |ctx: Context| async move { ctx.html(PAGE).await });

    app.post("/login", move |ctx: Context| async move {
        let name = ctx.req.query("name").unwrap_or_else(|| "anonymous".into());
        let session = ctx.session().await;
        session.regenerate(); // new id on login (session fixation)
        session.set("name", &name).await?;
        session.set("logged_in_on", &port).await?;
        ctx.json(json!({ "ok": true })).await
    });
    app.post("/logout", |ctx: Context| async move {
        ctx.session().await.destroy();
        ctx.json(json!({ "ok": true })).await
    });
    app.get("/me", |ctx: Context| async move {
        let session = ctx.session().await;
        let name: Option<String> = session.get("name").await?;
        let logged_in_on: Option<u16> = session.get("logged_in_on").await?;
        ctx.json(json!({ "name": name, "logged_in_on": logged_in_on }))
            .await
    });

    // Only /limited is rate limited, with one budget shared by both
    // instances. The page on :3000 also calls :3001, so the response — 429s
    // included — needs a CORS header.
    let limiter = RateLimiter::new(5, 10)
        .key(RateLimitKey::Global)
        .scope("demo-limited")
        .store(RedisRateLimitStore::new(conn))
        .build();
    app.use_middleware(Arc::new(move |ctx, next| {
        if ctx.req.path() != "/limited" {
            return next(ctx);
        }
        let limiter = limiter.clone();
        Box::pin(async move {
            let mut res = limiter(ctx, next).await?;
            res.headers_mut()
                .insert("Access-Control-Allow-Origin", "*".parse().unwrap());
            Ok(res)
        })
    }));
    app.get(
        "/limited",
        |ctx: Context| async move { ctx.text("ok").await },
    );
    app
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let conn = connect(&url).await?;

    println!("→ http://localhost:3000 and http://localhost:3001 (Redis: {url})");
    let a = app(conn.clone(), 3000);
    let b = app(conn, 3001);
    tokio::try_join!(a.listen("127.0.0.1:3000"), b.listen("127.0.0.1:3001"))?;
    Ok(())
}
