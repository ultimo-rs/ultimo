#![cfg(all(feature = "csrf", feature = "testing"))]

use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

fn app() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(ultimo::csrf::Csrf::new().secure(false).build());
    app.get("/", |ctx: Context| async move { ctx.text("ok").await });
    app.post(
        "/submit",
        |ctx: Context| async move { ctx.text("done").await },
    );
    app
}

#[tokio::test]
async fn get_issues_token_cookie() {
    let res = TestClient::new(app()).get("/").send().await;
    let sc = res.header("set-cookie").expect("GET issues a csrf cookie");
    assert!(sc.contains("csrf_token="));
    assert!(!sc.contains("HttpOnly")); // frontend must read it
}

#[tokio::test]
async fn post_without_token_is_forbidden() {
    let res = TestClient::new(app()).post("/submit").send().await;
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn post_with_matching_token_passes() {
    let res = TestClient::new(app())
        .post("/submit")
        .header("cookie", "csrf_token=tok123")
        .header("x-csrf-token", "tok123")
        .send()
        .await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.text(), "done");
}

#[tokio::test]
async fn post_with_mismatched_token_is_forbidden() {
    let res = TestClient::new(app())
        .post("/submit")
        .header("cookie", "csrf_token=tok123")
        .header("x-csrf-token", "different")
        .send()
        .await;
    assert_eq!(res.status(), 403);
}
