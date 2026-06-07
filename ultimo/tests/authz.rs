//! Integration tests for authorization guards (scopes via the JWT principal).
#![cfg(feature = "jwt")]

use http_body_util::Full;
use hyper::Request as HyperRequest;
use serde::Serialize;
use ultimo::auth::jwt::Jwt;
use ultimo::{Context, Ultimo};

#[derive(Serialize)]
struct Claims {
    sub: String,
    scope: String,
    exp: usize,
}

fn far_future() -> usize {
    4_102_444_800
}

fn jwt() -> Jwt {
    Jwt::hs256(b"secret")
}

fn token(scope: &str) -> String {
    jwt()
        .sign(&Claims {
            sub: "ada".into(),
            scope: scope.into(),
            exp: far_future(),
        })
        .unwrap()
}

fn app() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(jwt().build());
    app.get("/admin", |ctx: Context| async move {
        ctx.require_scope("admin").await?;
        ctx.json(serde_json::json!({ "ok": true })).await
    });
    app.get("/all", |ctx: Context| async move {
        ctx.require_all_scopes(&["read", "write"]).await?;
        ctx.json(serde_json::json!({ "ok": true })).await
    });
    app.get("/any", |ctx: Context| async move {
        ctx.require_any_scope(&["read", "x"]).await?;
        ctx.json(serde_json::json!({ "ok": true })).await
    });
    app
}

fn get(path: &str, bearer: Option<&str>) -> HyperRequest<Full<bytes::Bytes>> {
    let mut b = HyperRequest::builder().uri(path);
    if let Some(t) = bearer {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    b.body(Full::new(bytes::Bytes::new())).unwrap()
}

#[tokio::test]
async fn unauthenticated_is_401() {
    let res = app().oneshot(get("/admin", None)).await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn authenticated_without_scope_is_403() {
    let res = app().oneshot(get("/admin", Some(&token("read")))).await;
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn authenticated_with_scope_is_200() {
    let res = app()
        .oneshot(get("/admin", Some(&token("read write admin"))))
        .await;
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn require_all_scopes_enforced() {
    // Has both → 200.
    let res = app().oneshot(get("/all", Some(&token("read write")))).await;
    assert_eq!(res.status(), 200);
    // Missing one → 403.
    let res = app().oneshot(get("/all", Some(&token("read")))).await;
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn require_any_scope_enforced() {
    // Has one of {read, x} → 200.
    let res = app().oneshot(get("/any", Some(&token("read")))).await;
    assert_eq!(res.status(), 200);
    // Has neither → 403.
    let res = app().oneshot(get("/any", Some(&token("write")))).await;
    assert_eq!(res.status(), 403);
}
