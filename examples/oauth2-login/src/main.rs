//! "Sign in with …" demo. Run with `cargo run -p oauth2-login-example`, then
//! open http://localhost:3000.
//!
//! Picks a provider from the environment:
//!
//! - `GITHUB_CLIENT_ID` + `GITHUB_CLIENT_SECRET` → GitHub
//!   (callback URL to register: `http://localhost:3000/auth/callback`)
//! - `OIDC_ISSUER` + `OIDC_CLIENT_ID` + `OIDC_CLIENT_SECRET` → any OpenID
//!   Connect provider (Google, Entra ID, Auth0, Okta, Keycloak, …)
//! - nothing set → a tiny built-in **mock provider** on :3001, so the whole
//!   flow runs with no accounts or setup
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use ultimo::auth::oauth2::{OAuth2, Provider};
use ultimo::prelude::*;
use ultimo::session::{session, MemoryStore, SessionConfig};

const CALLBACK: &str = "http://localhost:3000/auth/callback";

#[tokio::main]
async fn main() -> Result<()> {
    let env = |k: &str| std::env::var(k).ok();
    let (label, oauth) =
        if let (Some(id), Some(secret)) = (env("GITHUB_CLIENT_ID"), env("GITHUB_CLIENT_SECRET")) {
            ("GitHub", OAuth2::github(id, secret, CALLBACK))
        } else if let (Some(issuer), Some(id), Some(secret)) = (
            env("OIDC_ISSUER"),
            env("OIDC_CLIENT_ID"),
            env("OIDC_CLIENT_SECRET"),
        ) {
            (
                "OpenID Connect",
                OAuth2::discover(&issuer, id, secret, CALLBACK).await?,
            )
        } else {
            tokio::spawn(mock_provider());
            let provider = Provider {
                authorization_url: "http://localhost:3001/authorize".into(),
                token_url: "http://localhost:3001/token".into(),
                userinfo_url: Some("http://localhost:3001/userinfo".into()),
            };
            (
                "the mock provider",
                OAuth2::new(provider, "demo", "demo-secret", CALLBACK),
            )
        };

    let mut app = Ultimo::new();
    // secure(false): the demo runs over plain http://localhost.
    app.use_middleware(session(
        MemoryStore::new(),
        SessionConfig::default().secure(false),
    ));

    app.get("/auth/login", oauth.login_handler());
    app.get("/auth/callback", oauth.callback_handler());
    app.post("/logout", |ctx: Context| async move {
        ctx.session().await.destroy();
        ctx.redirect("/").await
    });

    app.get("/", move |ctx: Context| async move {
        let body = match OAuth2::current_user(&ctx).await? {
            Some(user) => format!(
                "<p>Signed in as <b>{}</b> ({})</p>\
                 <form method=post action=/logout><button>Sign out</button></form>\
                 <h2>Claims</h2><pre>{}</pre>",
                escape(user.name.as_deref().unwrap_or(&user.subject)),
                escape(user.email.as_deref().unwrap_or("no email")),
                escape(&serde_json::to_string_pretty(&user.claims).unwrap_or_default()),
            ),
            None => format!("<p><a href=\"/auth/login?return_to=/\">Sign in with {label}</a></p>"),
        };
        ctx.html(format!(
            "<!doctype html><meta charset=utf-8><title>Ultimo OAuth2 demo</title>\
             <h1>OAuth2 login demo</h1>{body}"
        ))
        .await
    });

    println!("→ http://localhost:3000   (signing in with {label})");
    app.listen("127.0.0.1:3000").await
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A deliberately minimal OAuth2 provider for the demo: a consent page that
/// issues a code, a token endpoint that checks it, and a userinfo endpoint.
/// It skips almost everything a real provider must do — don't copy it.
async fn mock_provider() -> Result<()> {
    // code → redirect_uri it was issued for
    let codes: Arc<Mutex<HashMap<String, String>>> = Arc::default();
    let mut app = Ultimo::new_without_defaults();

    app.get("/authorize", |ctx: Context| async move {
        let q = ctx.req.query_string().unwrap_or_default().to_string();
        let params: HashMap<String, String> = serde_urlencoded::from_str(&q).unwrap_or_default();
        let approve = format!("/approve?{q}");
        ctx.html(format!(
            "<!doctype html><meta charset=utf-8><title>Mock provider</title>\
             <h1>Mock provider</h1>\
             <p><b>{}</b> wants to know who you are.</p>\
             <p><a href=\"{}\">Continue as Ada Lovelace</a></p>",
            escape(params.get("client_id").map(String::as_str).unwrap_or("?")),
            escape(&approve),
        ))
        .await
    });

    let issued = codes.clone();
    app.get("/approve", move |ctx: Context| {
        let issued = issued.clone();
        async move {
            let q = ctx.req.query_string().unwrap_or_default();
            let params: HashMap<String, String> = serde_urlencoded::from_str(q).unwrap_or_default();
            let (Some(redirect), Some(state)) = (params.get("redirect_uri"), params.get("state"))
            else {
                return Err(UltimoError::BadRequest("missing redirect_uri/state".into()));
            };
            let code = format!("code-{}", issued.lock().unwrap().len() + 1);
            issued
                .lock()
                .unwrap()
                .insert(code.clone(), redirect.clone());
            let back = serde_urlencoded::to_string([("code", &code), ("state", state)])
                .map_err(|e| UltimoError::Internal(e.to_string()))?;
            ctx.redirect(&format!("{redirect}?{back}")).await
        }
    });

    app.post("/token", move |ctx: Context| {
        let codes = codes.clone();
        async move {
            let form: HashMap<String, String> =
                serde_urlencoded::from_str(&ctx.req.text().await?).unwrap_or_default();
            let code = form.get("code").cloned().unwrap_or_default();
            let valid = codes.lock().unwrap().remove(&code).as_ref() == form.get("redirect_uri");
            if !valid {
                ctx.status(400).await;
                return ctx.json(json!({ "error": "invalid_grant" })).await;
            }
            ctx.json(json!({ "access_token": "mock-token", "token_type": "Bearer" }))
                .await
        }
    });

    app.get("/userinfo", |ctx: Context| async move {
        if ctx.req.header("authorization").as_deref() != Some("Bearer mock-token") {
            return Err(UltimoError::Unauthorized("bad token".into()));
        }
        ctx.json(json!({
            "sub": "ada-1815",
            "name": "Ada Lovelace",
            "email": "ada@example.com",
        }))
        .await
    });

    app.listen("127.0.0.1:3001").await
}
