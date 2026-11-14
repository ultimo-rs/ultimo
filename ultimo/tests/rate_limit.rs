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

/// Records the bucket keys it's asked about and allows the first `allow` calls.
#[derive(Clone, Default)]
struct RecordingStore {
    keys: std::sync::Arc<std::sync::Mutex<Vec<(String, u64, std::time::Duration)>>>,
    allow: usize,
}

#[async_trait::async_trait]
impl ultimo::middleware::builtin::RateLimitStore for RecordingStore {
    async fn try_acquire(&self, key: &str, max: u64, window: std::time::Duration) -> bool {
        let mut keys = self.keys.lock().unwrap();
        keys.push((key.to_string(), max, window));
        keys.len() <= self.allow
    }
}

#[tokio::test]
async fn rate_limiter_uses_a_custom_store_with_scoped_keys() {
    let store = RecordingStore {
        allow: 1,
        ..Default::default()
    };
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(
        RateLimiter::new(5, 60)
            .key(RateLimitKey::Global)
            .store(store.clone())
            .build(),
    );
    app.use_middleware(
        RateLimiter::new(5, 60)
            .key(RateLimitKey::Header("x-user".into()))
            .scope("per-user")
            .store(store.clone())
            .build(),
    );
    app.get("/", |ctx: Context| async move { ctx.text("ok").await });
    let client = TestClient::new(app);

    assert_eq!(
        client
            .get("/")
            .header("x-user", "ann")
            .send()
            .await
            .status(),
        429
    );
    assert_eq!(client.get("/").send().await.status(), 429);

    let keys = store.keys.lock().unwrap().clone();
    let window = std::time::Duration::from_secs(60);
    assert_eq!(
        keys,
        vec![
            // default scope is derived from the limit
            ("5/60s:__global__".to_string(), 5, window),
            ("per-user:ann".to_string(), 5, window),
            ("5/60s:__global__".to_string(), 5, window),
        ]
    );
}
