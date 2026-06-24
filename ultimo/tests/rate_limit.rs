#![cfg(feature = "testing")]

use ultimo::middleware::builtin::{rate_limiter, RateLimitKey, RateLimiter};
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

#[tokio::test]
async fn rate_limiter_allows_within_limit() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(RateLimiter::new(5, 60).key(RateLimitKey::Global).build());
    app.get("/", |ctx: Context| async move { ctx.text("ok").await });

    let client = TestClient::new(app);

    // 5 requests should all succeed
    for _ in 0..5 {
        let res = client.get("/").send().await;
        assert_eq!(res.status(), 200);
    }
}

#[tokio::test]
async fn rate_limiter_returns_429_when_exceeded() {
    let mut app = Ultimo::new_without_defaults();
    // Allow only 3 requests globally
    app.use_middleware(RateLimiter::new(3, 60).key(RateLimitKey::Global).build());
    app.get("/", |ctx: Context| async move { ctx.text("ok").await });

    let client = TestClient::new(app);

    // First 3 should pass
    for _ in 0..3 {
        let res = client.get("/").send().await;
        assert_eq!(res.status(), 200);
    }

    // 4th should be rate limited
    let res = client.get("/").send().await;
    assert_eq!(res.status(), 429);
    assert!(res.header("retry-after").is_some());
}

#[tokio::test]
async fn rate_limiter_default_convenience_function() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(rate_limiter());
    app.get("/", |ctx: Context| async move { ctx.text("ok").await });

    let client = TestClient::new(app);

    // Default is 100 req/min — first request should succeed
    let res = client.get("/").send().await;
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn rate_limiter_per_header_key() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(
        RateLimiter::new(2, 60)
            .key(RateLimitKey::Header("X-API-Key".into()))
            .build(),
    );
    app.get("/", |ctx: Context| async move { ctx.text("ok").await });

    let client = TestClient::new(app);

    // Client A: 2 requests OK, 3rd blocked
    let res = client.get("/").header("X-API-Key", "client-a").send().await;
    assert_eq!(res.status(), 200);
    let res = client.get("/").header("X-API-Key", "client-a").send().await;
    assert_eq!(res.status(), 200);
    let res = client.get("/").header("X-API-Key", "client-a").send().await;
    assert_eq!(res.status(), 429);

    // Client B: still has its own quota
    let res = client.get("/").header("X-API-Key", "client-b").send().await;
    assert_eq!(res.status(), 200);
}
