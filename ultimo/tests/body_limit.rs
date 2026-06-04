#![cfg(feature = "testing")]

use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

fn app() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.max_body_size(16); // 16 bytes
    app.post("/", |ctx: Context| async move { ctx.text("ok").await });
    app
}

#[tokio::test]
async fn rejects_oversized_body() {
    let big = "x".repeat(100);
    let res = TestClient::new(app()).post("/").text(&big).send().await;
    assert_eq!(res.status(), 413);
}

#[tokio::test]
async fn allows_body_within_limit() {
    let res = TestClient::new(app()).post("/").text("small").send().await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.text(), "ok");
}

#[tokio::test]
async fn no_limit_by_default() {
    let mut app = Ultimo::new_without_defaults();
    app.post("/", |ctx: Context| async move { ctx.text("ok").await });
    let big = "x".repeat(10_000);
    let res = TestClient::new(app).post("/").text(&big).send().await;
    assert_eq!(res.status(), 200);
}
