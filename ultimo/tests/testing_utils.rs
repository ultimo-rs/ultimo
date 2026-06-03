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
