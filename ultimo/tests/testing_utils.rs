#![cfg(feature = "testing")]

use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

fn app() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.get("/hello", |ctx: Context| async move { ctx.text("hi").await });
    app.post("/echo", |ctx: Context| async move {
        let body: serde_json::Value = ctx.req.json().await.unwrap_or_default();
        ctx.json(body).await
    });
    app
}

#[tokio::test]
async fn get_returns_handler_body() {
    let client = TestClient::new(app());
    let res = client.get("/hello").send().await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.text(), "hi");
}

#[tokio::test]
async fn post_json_round_trips() {
    let client = TestClient::new(app());
    let res = client
        .post("/echo")
        .json(&serde_json::json!({ "n": 1 }))
        .send()
        .await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.json::<serde_json::Value>(),
        serde_json::json!({ "n": 1 })
    );
}

#[tokio::test]
async fn assertions_pass_for_ok_text() {
    let client = TestClient::new(app());
    let res = client.get("/hello").send().await;
    res.assert_ok().assert_status(200).assert_text("hi");
}

#[tokio::test]
#[should_panic(expected = "expected status 404")]
async fn assert_status_panics_on_mismatch() {
    let client = TestClient::new(app());
    client.get("/hello").send().await.assert_status(404);
}

// ---- Task 6: macros ----
#[tokio::test]
async fn macros_work() {
    let client = TestClient::new(app());
    let res = client
        .post("/echo")
        .json(&serde_json::json!({ "ok": true }))
        .send()
        .await;
    ultimo::assert_status!(res, 200);
    ultimo::assert_json_eq!(
        res.json::<serde_json::Value>(),
        serde_json::json!({ "ok": true })
    );
}

// ---- Task 7: middleware helpers ----
use std::sync::Arc;
use ultimo::middleware::{BoxedMiddleware, Next};
use ultimo::testing::{run_middleware, test_context};

/// An auth middleware that 401s when the Authorization header is absent.
/// Constructed via `Arc::new(...)` exactly like the built-in middleware.
fn auth_mw() -> BoxedMiddleware {
    Arc::new(|ctx: Context, next: Next| {
        Box::pin(async move {
            if ctx.req.header("authorization").is_none() {
                return ultimo::response::ResponseBuilder::new()
                    .status(401)
                    .text("unauthorized")
                    .build();
            }
            next(ctx).await
        })
    })
}

#[tokio::test]
async fn middleware_can_short_circuit() {
    let ctx = test_context().method("GET").path("/private").build();
    let res = run_middleware(auth_mw(), ctx, |ctx: Context| async move {
        ctx.text("secret").await
    })
    .await
    .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn middleware_passes_through_when_authorized() {
    let ctx = test_context()
        .path("/private")
        .header("authorization", "Bearer t")
        .build();
    let res = run_middleware(auth_mw(), ctx, |ctx: Context| async move {
        ctx.text("secret").await
    })
    .await
    .unwrap();
    assert_eq!(res.status(), 200);
}

// ---- Task 9: fixtures ----
#[derive(serde::Deserialize, PartialEq, Debug)]
struct UserFixture {
    id: u32,
    name: String,
}

#[test]
fn load_fixture_parses_json() {
    let u: UserFixture = ultimo::testing::load_fixture("tests/fixtures/user.json");
    assert_eq!(
        u,
        UserFixture {
            id: 1,
            name: "Ada".into()
        }
    );
}
