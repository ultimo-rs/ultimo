//! Session auth example — a tiny full-stack demo of Ultimo's session feature.
//!
//! The backend serves a single HTML page whose JavaScript logs in/out against
//! cookie-backed session routes. Run it, then open http://127.0.0.1:3000.
//!
//! ```text
//! cargo run -p session-auth-example
//! ```

use ultimo::prelude::*;
use ultimo::session::{session, MemoryStore, SessionConfig};

/// The frontend: a minimal login UI driven entirely by `fetch` against the
/// session API below. Cookies flow automatically (same-origin).
const PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Ultimo · Session Auth Demo</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 28rem; margin: 4rem auto; padding: 0 1rem; }
    input, button { font: inherit; padding: 0.5rem 0.75rem; border-radius: 0.5rem; border: 1px solid #ccc; }
    button { cursor: pointer; border-color: #4f46e5; background: #4f46e5; color: white; }
    #status { margin-top: 1rem; padding: 0.75rem; border-radius: 0.5rem; background: #f4f4f5; }
  </style>
</head>
<body>
  <div style="background:#fef3c7;border:1px solid #f59e0b;border-radius:0.5rem;padding:0.5rem 1rem;margin-bottom:1rem;font-size:0.85rem;color:#92400e;">
    ⚡ <strong>Live demo</strong> — hosted on Render free tier. First request may take ~30s due to cold start.
    Powered by <a href="https://github.com/ultimo-rs/ultimo" style="color:#92400e;font-weight:600;">Ultimo</a>.
  </div>
  <h1>Ultimo Session Auth</h1>
  <p>Log in to set a cookie-backed session, then refresh — you stay logged in.</p>
  <div>
    <input id="username" placeholder="username" value="ada" />
    <button id="login">Log in</button>
    <button id="logout">Log out</button>
  </div>
  <div id="status">…</div>

  <script>
    const statusEl = document.getElementById('status');
    async function refresh() {
      const res = await fetch('/api/me');
      const { user } = await res.json();
      statusEl.textContent = user ? `Logged in as ${user}` : 'Not logged in';
    }
    // Read the CSRF token the server set in a (non-HttpOnly) cookie.
    function csrfToken() {
      return document.cookie.split('; ').find((c) => c.startsWith('csrf_token='))?.split('=')[1] || '';
    }
    document.getElementById('login').onclick = async () => {
      const username = document.getElementById('username').value || 'guest';
      await fetch('/api/login', {
        method: 'POST',
        headers: { 'content-type': 'application/json', 'x-csrf-token': csrfToken() },
        body: JSON.stringify({ username }),
      });
      refresh();
    };
    document.getElementById('logout').onclick = async () => {
      await fetch('/api/logout', { method: 'POST', headers: { 'x-csrf-token': csrfToken() } });
      refresh();
    };
    refresh();
  </script>
</body>
</html>"#;

#[derive(Deserialize)]
struct LoginBody {
    username: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut app = Ultimo::new_without_defaults();

    // CSRF protection (double-submit cookie). secure(false) for local HTTP dev.
    app.use_middleware(ultimo::csrf::Csrf::new().secure(false).build());

    // Cookie-backed sessions. secure(false) so the cookie works over local HTTP;
    // in production keep the default secure(true) and serve over HTTPS.
    app.use_middleware(session(
        MemoryStore::new(),
        SessionConfig::default().secure(false),
    ));

    // Serve the frontend.
    app.get("/", |ctx: Context| async move { ctx.html(PAGE).await });

    // Log in: store the user in the session and rotate the id (fixation defense).
    app.post("/api/login", |ctx: Context| async move {
        let body: LoginBody = ctx.req.json().await?;
        let s = ctx.session().await;
        s.set("user", &body.username).await?;
        s.regenerate();
        ctx.json(json!({ "ok": true, "user": body.username })).await
    });

    // Who am I? Reads the session (None if not logged in).
    app.get("/api/me", |ctx: Context| async move {
        let user: Option<String> = ctx.session().await.get("user").await?;
        ctx.json(json!({ "user": user })).await
    });

    // Log out: destroy the session + expire the cookie.
    app.post("/api/logout", |ctx: Context| async move {
        ctx.session().await.destroy();
        ctx.json(json!({ "ok": true })).await
    });

    println!("🔐 Session auth demo: http://127.0.0.1:3000");
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    app.listen(&addr).await
}
