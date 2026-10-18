#![cfg(feature = "testing")]

use ultimo::middleware::builtin::{append_trailing_slash, trim_trailing_slash};
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

fn app_with(mw: ultimo::middleware::BoxedMiddleware) -> TestClient {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(mw);
    app.get("/", |ctx: Context| async move { ctx.text("root").await });
    app.get(
        "/users",
        |ctx: Context| async move { ctx.text("users").await },
    );
    app.post(
        "/users",
        |ctx: Context| async move { ctx.text("created").await },
    );
    app.get(
        "/app.js",
        |ctx: Context| async move { ctx.text("js").await },
    );
    TestClient::new(app)
}

#[tokio::test]
async fn trim_redirects_to_canonical_path_keeping_query() {
    let client = app_with(trim_trailing_slash());

    let res = client.get("/users/?page=2").send().await;
    assert_eq!(res.status(), 308);
    assert_eq!(res.header("location"), Some("/users?page=2"));

    let res = client.get("/users").send().await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.text(), "users");

    assert_eq!(client.get("/").send().await.status(), 200);
}

#[tokio::test]
async fn trim_collapses_duplicate_slashes_without_open_redirect() {
    let client = app_with(trim_trailing_slash());

    let res = client.get("//users/").send().await;
    assert_eq!(res.status(), 308);
    // `//host` would be a protocol-relative URL pointing off-site.
    assert_eq!(res.header("location"), Some("/users"));
}

#[tokio::test]
async fn trim_leaves_non_get_requests_alone() {
    let client = app_with(trim_trailing_slash());
    let res = client.post("/users/").send().await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.text(), "created");
}

#[tokio::test]
async fn append_redirects_to_slash_form() {
    let client = app_with(append_trailing_slash());

    let res = client.get("/users?x=1").send().await;
    assert_eq!(res.status(), 308);
    assert_eq!(res.header("location"), Some("/users/?x=1"));

    let res = client.get("/users/").send().await;
    assert_eq!(res.status(), 200);

    // File-like paths are left alone.
    assert_eq!(client.get("/app.js").send().await.status(), 200);
    assert_eq!(client.get("/").send().await.status(), 200);
}

#[tokio::test]
async fn unmatched_paths_still_404() {
    let client = app_with(trim_trailing_slash());
    assert_eq!(client.get("/nope/").send().await.status(), 404);
}
