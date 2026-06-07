//! JWT auth example — a tiny full-stack demo of Ultimo's `jwt` feature and the
//! authorization guards.
//!
//! `/api/login` issues a short-lived HS256 token carrying scopes; the page stores
//! it and sends it as `Authorization: Bearer <token>`. `/api/me` requires any
//! valid token; `/api/admin` additionally requires the `admin` scope (granted
//! only to the user "admin"), demonstrating `ctx.require_scope`.
//!
//! ```text
//! cargo run -p jwt-auth-example
//! ```
use std::time::{SystemTime, UNIX_EPOCH};
use ultimo::auth::jwt::Jwt;
use ultimo::prelude::*;

const PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Ultimo · JWT Auth Demo</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 30rem; margin: 4rem auto; padding: 0 1rem; }
    input, button { font: inherit; padding: 0.5rem 0.75rem; border-radius: 0.5rem; border: 1px solid #ccc; }
    button { cursor: pointer; border-color: #4f46e5; background: #4f46e5; color: white; }
    #status { margin-top: 1rem; padding: 0.75rem; border-radius: 0.5rem; background: #f4f4f5; }
    .hint { color: #666; font-size: 0.9rem; }
  </style>
</head>
<body>
  <h1>Ultimo JWT Auth + Guards</h1>
  <p class="hint">Log in as <code>admin</code> to get the <code>admin</code> scope; any other name gets only <code>user</code>.</p>
  <div>
    <input id="username" placeholder="username" value="ada" />
    <button id="login">Log in</button>
  </div>
  <p>
    <button id="me">Call /api/me</button>
    <button id="admin">Call /api/admin</button>
    <button id="logout">Forget token</button>
  </p>
  <div id="status">…</div>

  <script>
    const statusEl = document.getElementById('status');
    let token = null;
    const authHeaders = () => (token ? { authorization: 'Bearer ' + token } : {});
    document.getElementById('login').onclick = async () => {
      const username = document.getElementById('username').value || 'guest';
      const res = await fetch('/api/login', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ username }),
      });
      ({ token } = await res.json());
      statusEl.textContent = 'Got token: ' + token.slice(0, 24) + '…';
    };
    document.getElementById('me').onclick = async () => {
      const res = await fetch('/api/me', { headers: authHeaders() });
      const body = await res.json();
      statusEl.textContent = res.ok ? ('Hello, ' + body.sub) : ('Rejected (' + res.status + ')');
    };
    document.getElementById('admin').onclick = async () => {
      const res = await fetch('/api/admin', { headers: authHeaders() });
      statusEl.textContent = res.ok
        ? 'Admin access granted ✅'
        : (res.status === 403 ? 'Forbidden — missing admin scope (403)' : 'Rejected (' + res.status + ')');
    };
    document.getElementById('logout').onclick = () => {
      token = null;
      statusEl.textContent = 'Token forgotten — protected routes will now 401.';
    };
  </script>
</body>
</html>"#;

#[derive(Deserialize)]
struct LoginBody {
    username: String,
}

#[derive(Serialize)]
struct Claims {
    sub: String,
    scope: String,
    exp: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // In a real app, load the secret from the environment — never hardcode it.
    let jwt = Jwt::hs256(b"demo-secret-change-me");

    let mut app = Ultimo::new_without_defaults();

    // Mount the verifier in *optional* mode so `/` and `/api/login` stay public;
    // the protected handlers enforce auth (and scopes) themselves.
    app.use_middleware(jwt.clone().optional().build());

    app.get("/", |ctx: Context| async move { ctx.html(PAGE).await });

    // Issue a 15-minute token. "admin" gets the admin scope; everyone gets "user".
    let signer = jwt.clone();
    app.post("/api/login", move |ctx: Context| {
        let signer = signer.clone();
        async move {
            let body: LoginBody = ctx.req.json().await?;
            let scope = if body.username == "admin" {
                "user admin"
            } else {
                "user"
            };
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| UltimoError::Internal(e.to_string()))?
                .as_secs() as usize;
            let token = signer.sign(&Claims {
                sub: body.username.clone(),
                scope: scope.to_string(),
                exp: now + 900,
            })?;
            ctx.json(json!({ "token": token })).await
        }
    });

    // Requires any valid token.
    app.get("/api/me", |ctx: Context| async move {
        let principal = ctx.require_auth().await?;
        ctx.json(json!({ "sub": principal.id, "scopes": principal.scopes }))
            .await
    });

    // Requires the `admin` scope (401 if unauthenticated, 403 if missing it).
    app.get("/api/admin", |ctx: Context| async move {
        ctx.require_scope("admin").await?;
        ctx.json(json!({ "ok": true, "area": "admin" })).await
    });

    println!("🔑 JWT auth + guards demo: http://127.0.0.1:3000");
    app.listen("127.0.0.1:3000").await
}
