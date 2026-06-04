#![cfg(all(feature = "session", feature = "testing"))]

use ultimo::session::{session, MemoryStore, SessionConfig};
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

fn app() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    // secure(false) so the test cookie isn't HTTPS-only (TestClient is in-process).
    app.use_middleware(session(
        MemoryStore::new(),
        SessionConfig::default().secure(false),
    ));
    app.get("/login", |ctx: Context| async move {
        ctx.session().await.set("uid", &42u64).await?;
        ctx.text("ok").await
    });
    app.get("/me", |ctx: Context| async move {
        let uid: Option<u64> = ctx.session().await.get("uid").await?;
        ctx.json(serde_json::json!({ "uid": uid })).await
    });
    app.get("/anon", |ctx: Context| async move { ctx.text("hi").await });
    app
}

/// Extract the "name=value" pair from a Set-Cookie header.
fn sid(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_string()
}

#[tokio::test]
async fn session_persists_across_requests() {
    let client = TestClient::new(app());
    let login = client.get("/login").send().await;
    let cookie = sid(login.header("set-cookie").expect("login sets a cookie"));

    let me = client.get("/me").header("cookie", &cookie).send().await;
    assert_eq!(
        me.json::<serde_json::Value>(),
        serde_json::json!({ "uid": 42 })
    );
}

#[tokio::test]
async fn empty_session_sets_no_cookie() {
    // anti-DoS: a request that never touches the session gets no Set-Cookie.
    let client = TestClient::new(app());
    let res = client.get("/anon").send().await;
    assert!(res.header("set-cookie").is_none());
}

#[tokio::test]
async fn unknown_client_id_is_not_adopted() {
    // anti-fixation: a forged cookie id must not become the session id.
    let client = TestClient::new(app());
    let res = client
        .get("/login")
        .header("cookie", "ultimo_sid=attacker_fixed_id")
        .send()
        .await;
    let issued = sid(res.header("set-cookie").expect("login sets a cookie"));
    assert_ne!(issued, "ultimo_sid=attacker_fixed_id");
}

#[tokio::test]
async fn default_cookie_is_httponly_lax() {
    let client = TestClient::new(app());
    let res = client.get("/login").send().await;
    let sc = res.header("set-cookie").expect("login sets a cookie");
    assert!(sc.contains("HttpOnly"));
    assert!(sc.contains("SameSite=Lax"));
}
