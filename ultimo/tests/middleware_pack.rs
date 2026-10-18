#![cfg(feature = "testing")]

use base64::Engine;
use ultimo::middleware::builtin::{cache_control, etag, server_timing, BasicAuth};
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

fn hello_app() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.get("/", |ctx: Context| async move { ctx.text("hello").await });
    app.post("/", |ctx: Context| async move { ctx.text("posted").await });
    app.get("/missing", |_ctx: Context| async move {
        Err(ultimo::UltimoError::NotFound("nope".into()))
    });
    app
}

// ---------------------------------------------------------------- etag

#[tokio::test]
async fn etag_is_set_and_stable() {
    let mut app = hello_app();
    app.use_middleware(etag());
    let client = TestClient::new(app);

    let a = client.get("/").send().await;
    let b = client.get("/").send().await;
    let tag = a.header("etag").expect("etag header").to_string();
    assert!(tag.starts_with('"') && tag.ends_with('"'), "{tag}");
    assert_eq!(b.header("etag"), Some(tag.as_str()));
}

#[tokio::test]
async fn etag_match_returns_304_without_body() {
    let mut app = hello_app();
    app.use_middleware(etag());
    let client = TestClient::new(app);

    let tag = client
        .get("/")
        .send()
        .await
        .header("etag")
        .unwrap()
        .to_string();

    let res = client.get("/").header("if-none-match", &tag).send().await;
    assert_eq!(res.status(), 304);
    assert!(res.bytes().is_empty());
    assert_eq!(res.header("etag"), Some(tag.as_str()));

    // Weak comparison and lists are accepted, per RFC 9110.
    let weak_list = format!("\"other\", W/{tag}");
    let res = client
        .get("/")
        .header("if-none-match", &weak_list)
        .send()
        .await;
    assert_eq!(res.status(), 304);

    let res = client.get("/").header("if-none-match", "*").send().await;
    assert_eq!(res.status(), 304);
}

#[tokio::test]
async fn etag_mismatch_returns_full_response() {
    let mut app = hello_app();
    app.use_middleware(etag());
    let res = TestClient::new(app)
        .get("/")
        .header("if-none-match", "\"stale\"")
        .send()
        .await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.text(), "hello");
}

#[tokio::test]
async fn etag_skips_non_get_and_errors() {
    let mut app = hello_app();
    app.use_middleware(etag());
    let client = TestClient::new(app);

    assert!(client.post("/").send().await.header("etag").is_none());
    assert!(client.get("/missing").send().await.header("etag").is_none());
}

#[tokio::test]
async fn etag_keeps_handler_supplied_etag() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(etag());
    app.get("/", |ctx: Context| async move {
        ctx.header("ETag", "\"v1\"").await;
        ctx.text("hello").await
    });
    let client = TestClient::new(app);

    assert_eq!(client.get("/").send().await.header("etag"), Some("\"v1\""));
    let res = client
        .get("/")
        .header("if-none-match", "\"v1\"")
        .send()
        .await;
    assert_eq!(res.status(), 304);
}

// ------------------------------------------------------- cache_control

#[tokio::test]
async fn cache_control_applies_to_success_only() {
    let mut app = hello_app();
    app.use_middleware(cache_control("public, max-age=60"));
    let client = TestClient::new(app);

    assert_eq!(
        client.get("/").send().await.header("cache-control"),
        Some("public, max-age=60")
    );
    assert!(client
        .get("/missing")
        .send()
        .await
        .header("cache-control")
        .is_none());
}

#[tokio::test]
async fn cache_control_does_not_override_handler() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(cache_control("public, max-age=60"));
    app.get("/", |ctx: Context| async move {
        ctx.header("Cache-Control", "no-store").await;
        ctx.text("secret").await
    });
    let res = TestClient::new(app).get("/").send().await;
    assert_eq!(res.header("cache-control"), Some("no-store"));
}

// ------------------------------------------------------- server_timing

#[tokio::test]
async fn server_timing_reports_duration() {
    let mut app = hello_app();
    app.use_middleware(server_timing());
    let res = TestClient::new(app).get("/").send().await;

    let value = res.header("server-timing").expect("server-timing header");
    let dur = value
        .strip_prefix("app;dur=")
        .unwrap_or_else(|| panic!("unexpected format: {value}"));
    assert!(dur.parse::<f64>().is_ok(), "{value}");
}

// ---------------------------------------------------------- basic auth

fn basic(user: &str, pass: &str) -> String {
    let token = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"));
    format!("Basic {token}")
}

fn protected_app() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(
        BasicAuth::new("Admin")
            .user("alice", "s3cret")
            .user("bob", "hunter2")
            .build(),
    );
    app.get("/", |ctx: Context| async move {
        let user = ctx.basic_auth_user().await.unwrap_or_default();
        ctx.text(format!("hi {user}")).await
    });
    app
}

#[tokio::test]
async fn basic_auth_rejects_missing_credentials() {
    let res = TestClient::new(protected_app()).get("/").send().await;
    assert_eq!(res.status(), 401);
    assert_eq!(
        res.header("www-authenticate"),
        Some("Basic realm=\"Admin\", charset=\"UTF-8\"")
    );
}

#[tokio::test]
async fn basic_auth_rejects_wrong_password_and_garbage() {
    let client = TestClient::new(protected_app());
    for value in [
        basic("alice", "wrong"),
        basic("mallory", "s3cret"),
        "Basic !!!not-base64".to_string(),
        "Bearer abc".to_string(),
    ] {
        let res = client.get("/").header("authorization", &value).send().await;
        assert_eq!(res.status(), 401, "{value}");
    }
}

#[tokio::test]
async fn basic_auth_accepts_valid_users() {
    let client = TestClient::new(protected_app());
    let res = client
        .get("/")
        .header("authorization", &basic("alice", "s3cret"))
        .send()
        .await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.text(), "hi alice");

    // Scheme is case-insensitive; passwords may contain ':'.
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(BasicAuth::new("x").user("carol", "a:b").build());
    app.get("/", |ctx: Context| async move { ctx.text("ok").await });
    let token = basic("carol", "a:b").replacen("Basic", "basic", 1);
    let res = TestClient::new(app)
        .get("/")
        .header("authorization", &token)
        .send()
        .await;
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn basic_auth_custom_validator() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(
        BasicAuth::with_validator("x", |user, pass| user == "dave" && pass.len() > 3).build(),
    );
    app.get("/", |ctx: Context| async move { ctx.text("ok").await });
    let client = TestClient::new(app);

    let ok = client
        .get("/")
        .header("authorization", &basic("dave", "long-pass"))
        .send()
        .await;
    assert_eq!(ok.status(), 200);
    let bad = client
        .get("/")
        .header("authorization", &basic("dave", "no"))
        .send()
        .await;
    assert_eq!(bad.status(), 401);
}
