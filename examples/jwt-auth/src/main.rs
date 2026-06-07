//! JWT auth example — a tiny full-stack demo of Ultimo's `jwt` feature.
//!
//! `/api/login` issues a short-lived HS256 token; the page stores it and sends it
//! as `Authorization: Bearer <token>` on calls to the protected `/api/me`.
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
    body { font-family: system-ui, sans-serif; max-width: 28rem; margin: 4rem auto; padding: 0 1rem; }
    input, button { font: inherit; padding: 0.5rem 0.75rem; border-radius: 0.5rem; border: 1px solid #ccc; }
    button { cursor: pointer; border-color: #4f46e5; background: #4f46e5; color: white; }
    #status { margin-top: 1rem; padding: 0.75rem; border-radius: 0.5rem; background: #f4f4f5; }
  </style>
</head>
<body>
  <h1>Ultimo JWT Auth</h1>
  <p>Log in to receive a bearer token, then call the protected endpoint.</p>
  <div>
    <input id="username" placeholder="username" value="ada" />
    <button id="login">Log in</button>
    <button id="me">Call /api/me</button>
    <button id="logout">Forget token</button>
  </div>
  <div id="status">…</div>

  <script>
    const statusEl = document.getElementById('status');
    let token = null;
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
      const res = await fetch('/api/me', {
        headers: token ? { authorization: 'Bearer ' + token } : {},
      });
      const body = await res.json();
      statusEl.textContent = res.ok ? ('Hello, ' + body.sub) : ('Rejected (' + res.status + ')');
    };
    document.getElementById('logout').onclick = () => {
      token = null;
      statusEl.textContent = 'Token forgotten — /api/me will now 401.';
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
    exp: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // In a real app, load the secret from the environment — never hardcode it.
    let jwt = Jwt::hs256(b"demo-secret-change-me");

    let mut app = Ultimo::new_without_defaults();

    // Mount the verifier in *optional* mode so `/` and `/api/login` stay public;
    // the protected handler (`/api/me`) enforces that claims are present itself.
    app.use_middleware(jwt.clone().optional().build());

    app.get("/", |ctx: Context| async move { ctx.html(PAGE).await });

    // Issue a token valid for 15 minutes (secure-by-default: short-lived).
    let signer = jwt.clone();
    app.post("/api/login", move |ctx: Context| {
        let signer = signer.clone();
        async move {
            let body: LoginBody = ctx.req.json().await?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| UltimoError::Internal(e.to_string()))?
                .as_secs() as usize;
            let token = signer.sign(&Claims {
                sub: body.username.clone(),
                exp: now + 900,
            })?;
            ctx.json(json!({ "token": token })).await
        }
    });

    // Protected: the optional middleware attached claims iff a valid token was sent.
    app.get("/api/me", |ctx: Context| async move {
        match ctx.jwt_claims().await {
            Some(c) => ctx.json(json!({ "sub": c.get("sub") })).await,
            None => Err(UltimoError::Unauthorized("login required".into())),
        }
    });

    println!("🔑 JWT auth demo: http://127.0.0.1:3000");
    app.listen("127.0.0.1:3000").await
}
