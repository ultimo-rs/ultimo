#![cfg(feature = "testing")]

use ultimo::cookie::{Cookie, SameSite};
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

#[tokio::test]
async fn set_cookie_appears_on_response() {
    let mut app = Ultimo::new_without_defaults();
    app.get("/set", |ctx: Context| async move {
        ctx.set_cookie(
            Cookie::new("sid", "xyz")
                .http_only(true)
                .same_site(SameSite::Lax),
        )
        .await?;
        ctx.text("ok").await
    });

    let client = TestClient::new(app);
    let res = client.get("/set").send().await;
    let sc = res.header("set-cookie").unwrap();
    assert!(sc.contains("sid=xyz"));
    assert!(sc.contains("HttpOnly"));
    assert!(sc.contains("SameSite=Lax"));
}

#[tokio::test]
async fn reads_request_cookie() {
    let mut app = Ultimo::new_without_defaults();
    app.get("/read", |ctx: Context| async move {
        let v = ctx.cookie("token").unwrap_or_else(|| "none".into());
        ctx.text(v).await
    });

    let client = TestClient::new(app);
    let res = client
        .get("/read")
        .header("cookie", "token=secret123")
        .send()
        .await;
    assert_eq!(res.text(), "secret123");
}
