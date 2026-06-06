#![cfg(feature = "testing")]

use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

fn ip_app(trust: bool) -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.trust_proxy(trust);
    app.get("/ip", |ctx: Context| async move {
        let ip = ctx
            .client_ip()
            .map(|i| i.to_string())
            .unwrap_or_else(|| "none".into());
        ctx.text(ip).await
    });
    app
}

#[tokio::test]
async fn xff_used_when_proxy_trusted() {
    let res = TestClient::new(ip_app(true))
        .get("/ip")
        .header("x-forwarded-for", "203.0.113.7, 10.0.0.1")
        .send()
        .await;
    assert_eq!(res.text(), "203.0.113.7");
}

#[tokio::test]
async fn xff_ignored_when_not_trusted() {
    // Untrusted + no real peer (in-process) → no client IP, header ignored.
    let res = TestClient::new(ip_app(false))
        .get("/ip")
        .header("x-forwarded-for", "203.0.113.7")
        .send()
        .await;
    assert_eq!(res.text(), "none");
}

#[tokio::test]
async fn forwarded_header_used_when_trusted() {
    let res = TestClient::new(ip_app(true))
        .get("/ip")
        .header("forwarded", "for=198.51.100.5;proto=https")
        .send()
        .await;
    assert_eq!(res.text(), "198.51.100.5");
}
