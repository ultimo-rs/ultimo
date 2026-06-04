#![cfg(feature = "testing")]

use ultimo::middleware::builtin::{security_headers, SecurityHeaders};
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

#[tokio::test]
async fn default_security_headers_are_set() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(security_headers());
    app.get("/", |ctx: Context| async move { ctx.text("ok").await });

    let res = TestClient::new(app).get("/").send().await;
    assert_eq!(res.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(res.header("x-frame-options"), Some("DENY"));
    assert_eq!(
        res.header("referrer-policy"),
        Some("strict-origin-when-cross-origin")
    );
    assert!(res.header("strict-transport-security").is_some());
    assert!(res.header("permissions-policy").is_some());
    // CSP is opt-in — not set by default.
    assert!(res.header("content-security-policy").is_none());
}

#[tokio::test]
async fn csp_opt_in_and_handler_override_wins() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(
        SecurityHeaders::new()
            .csp("default-src 'self'")
            .frame_options("SAMEORIGIN")
            .build(),
    );
    // Handler sets its own frame-options → middleware must not override it.
    app.get("/", |ctx: Context| async move {
        ctx.header("x-frame-options", "DENY").await;
        ctx.text("ok").await
    });

    let res = TestClient::new(app).get("/").send().await;
    assert_eq!(
        res.header("content-security-policy"),
        Some("default-src 'self'")
    );
    assert_eq!(res.header("x-frame-options"), Some("DENY")); // handler's value kept
}

#[tokio::test]
async fn no_hsts_disables_it() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(SecurityHeaders::new().no_hsts().build());
    app.get("/", |ctx: Context| async move { ctx.text("ok").await });

    let res = TestClient::new(app).get("/").send().await;
    assert!(res.header("strict-transport-security").is_none());
}
