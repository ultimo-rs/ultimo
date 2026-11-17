#![cfg(all(feature = "redis", feature = "session", feature = "testing"))]

//! Needs a real Redis: set `REDIS_URL` (e.g. `redis://127.0.0.1:6379`). Without
//! it every test returns early, so a plain `cargo test` stays green offline.
//! Each test namespaces its keys with a fresh UUID so tests can run in parallel.

use std::time::Duration;
use ultimo::cache::{Cache, CacheStore, CachedResponse};
use ultimo::middleware::builtin::{RateLimitKey, RateLimitStore, RateLimiter};
use ultimo::redis::{connect, RedisCacheStore, RedisRateLimitStore, RedisSessionStore};
use ultimo::session::{session, SessionConfig, SessionData, SessionStore};
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

async fn conn() -> Option<ultimo::redis::client::aio::ConnectionManager> {
    let url = std::env::var("REDIS_URL").ok()?;
    Some(connect(&url).await.expect("connect to REDIS_URL"))
}

fn prefix() -> String {
    format!("ultimo-test:{}:", uuid::Uuid::new_v4())
}

#[tokio::test]
async fn session_store_roundtrip_and_destroy() {
    let Some(conn) = conn().await else { return };
    let store = RedisSessionStore::new(conn).prefix(prefix());

    let mut data = SessionData::new();
    data.insert("uid".into(), serde_json::json!(42));
    store.store("abc", &data, Duration::from_secs(60)).await;
    assert_eq!(store.load("abc").await, Some(data));

    store.destroy("abc").await;
    assert_eq!(store.load("abc").await, None);
    assert_eq!(store.load("never-stored").await, None);
}

#[tokio::test]
async fn session_store_expires_with_ttl() {
    let Some(conn) = conn().await else { return };
    let store = RedisSessionStore::new(conn).prefix(prefix());

    let mut data = SessionData::new();
    data.insert("k".into(), serde_json::json!("v"));
    store
        .store("short", &data, Duration::from_millis(150))
        .await;
    assert!(store.load("short").await.is_some());
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(store.load("short").await, None);
}

#[tokio::test]
async fn sessions_are_shared_between_app_instances() {
    let Some(conn) = conn().await else { return };
    let prefix = prefix();
    let make_app = |conn| {
        let mut app = Ultimo::new_without_defaults();
        app.use_middleware(session(
            RedisSessionStore::new(conn).prefix(prefix.clone()),
            SessionConfig::default().secure(false),
        ));
        app.get("/login", |ctx: Context| async move {
            ctx.session().await.set("uid", &7u64).await?;
            ctx.text("ok").await
        });
        app.get("/me", |ctx: Context| async move {
            let uid: Option<u64> = ctx.session().await.get("uid").await?;
            ctx.json(serde_json::json!({ "uid": uid })).await
        });
        app
    };
    // Two independent apps, as if behind a load balancer.
    let a = TestClient::new(make_app(conn.clone()));
    let b = TestClient::new(make_app(conn));

    let login = a.get("/login").send().await;
    let cookie = login
        .header("set-cookie")
        .expect("login sets a cookie")
        .split(';')
        .next()
        .unwrap()
        .to_string();

    let me = b.get("/me").header("cookie", &cookie).send().await;
    assert_eq!(
        me.json::<serde_json::Value>(),
        serde_json::json!({ "uid": 7 })
    );
}

#[tokio::test]
async fn rate_limit_store_enforces_the_bucket() {
    let Some(conn) = conn().await else { return };
    let store = RedisRateLimitStore::new(conn).prefix(prefix());
    let window = Duration::from_secs(60);

    for _ in 0..3 {
        assert!(store.try_acquire("ip:1", 3, window).await);
    }
    assert!(!store.try_acquire("ip:1", 3, window).await);
    // Other keys have their own bucket.
    assert!(store.try_acquire("ip:2", 3, window).await);
}

#[tokio::test]
async fn rate_limit_store_refills_over_time() {
    let Some(conn) = conn().await else { return };
    let store = RedisRateLimitStore::new(conn).prefix(prefix());
    // 2 tokens per second → one token back every 500 ms.
    let window = Duration::from_secs(1);

    assert!(store.try_acquire("k", 2, window).await);
    assert!(store.try_acquire("k", 2, window).await);
    assert!(!store.try_acquire("k", 2, window).await);
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert!(store.try_acquire("k", 2, window).await);
    assert!(!store.try_acquire("k", 2, window).await);
}

#[tokio::test]
async fn rate_limit_is_shared_between_app_instances() {
    let Some(conn) = conn().await else { return };
    let prefix = prefix();
    let make_app = |conn| {
        let mut app = Ultimo::new_without_defaults();
        app.use_middleware(
            RateLimiter::new(3, 60)
                .key(RateLimitKey::Global)
                .store(RedisRateLimitStore::new(conn).prefix(prefix.clone()))
                .build(),
        );
        app.get("/", |ctx: Context| async move { ctx.text("ok").await });
        app
    };
    let a = TestClient::new(make_app(conn.clone()));
    let b = TestClient::new(make_app(conn));

    // 3 requests split across two instances use up one shared budget.
    assert_eq!(a.get("/").send().await.status(), 200);
    assert_eq!(b.get("/").send().await.status(), 200);
    assert_eq!(a.get("/").send().await.status(), 200);
    assert_eq!(b.get("/").send().await.status(), 429);
}

#[tokio::test]
async fn rate_limit_store_fails_open_unless_fail_closed() {
    let Some(mut conn) = conn().await else { return };
    let prefix = prefix();
    // A plain string where the bucket hash should be makes the script error
    // (WRONGTYPE) — the same path as Redis being unreachable.
    let _: () = ultimo::redis::client::AsyncCommands::set(&mut conn, format!("{prefix}k"), "x")
        .await
        .unwrap();
    let window = Duration::from_secs(60);

    let open = RedisRateLimitStore::new(conn.clone()).prefix(prefix.clone());
    assert!(
        open.try_acquire("k", 1, window).await,
        "fails open by default"
    );

    let closed = RedisRateLimitStore::new(conn).prefix(prefix).fail_closed();
    assert!(
        !closed.try_acquire("k", 1, window).await,
        "fail_closed rejects"
    );
}

// ---------------------------------------------------------------- cache

#[tokio::test]
async fn cache_store_roundtrip_expiry_and_prefix_removal() {
    let Some(conn) = conn().await else { return };
    let store = RedisCacheStore::new(conn).prefix(prefix());
    let entry = CachedResponse::new(
        200,
        vec![("content-type".into(), b"text/plain".to_vec())],
        bytes::Bytes::from_static(b"hello"),
    );

    store
        .set("s:/a?x=1", entry.clone(), Duration::from_secs(60))
        .await;
    store
        .set("s:/a?x=2", entry.clone(), Duration::from_secs(60))
        .await;
    // `?` and `*` in keys must match literally, not as globs.
    store
        .set("s:/ab?", entry.clone(), Duration::from_secs(60))
        .await;
    store
        .set("s:/b?", entry.clone(), Duration::from_millis(150))
        .await;
    assert_eq!(store.get("s:/a?x=1").await, Some(entry.clone()));

    store.remove_prefix("s:/a?").await.unwrap();
    assert_eq!(store.get("s:/a?x=1").await, None);
    assert_eq!(store.get("s:/a?x=2").await, None);
    assert!(store.get("s:/ab?").await.is_some());

    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(store.get("s:/b?").await, None);
}

#[tokio::test]
async fn cache_is_shared_and_invalidated_across_instances() {
    let Some(conn) = conn().await else { return };
    let prefix = prefix();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let make = |conn| {
        let cache = Cache::new(Duration::from_secs(60))
            .store(RedisCacheStore::new(conn).prefix(prefix.clone()));
        let mut app = Ultimo::new_without_defaults();
        app.use_middleware(cache.middleware());
        let calls = calls.clone();
        app.get("/n", move |ctx: Context| {
            let calls = calls.clone();
            async move {
                let n = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                ctx.text(n.to_string()).await
            }
        });
        (cache, TestClient::new(app))
    };
    let (cache_a, a) = make(conn.clone());
    let (_cache_b, b) = make(conn);

    assert_eq!(a.get("/n").send().await.text(), "1");
    let hit = b.get("/n").send().await;
    assert_eq!(hit.header("x-cache"), Some("HIT"));
    assert_eq!(hit.text(), "1");

    cache_a.invalidate("/n").await.unwrap();
    assert_eq!(b.get("/n").send().await.text(), "2");
}
