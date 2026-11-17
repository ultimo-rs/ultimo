#![cfg(feature = "testing")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use ultimo::cache::{Cache, CacheStore, CachedResponse, MemoryCacheStore};
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

/// An app whose handlers count how often they run, so a cache hit is visible
/// as "the handler didn't run".
fn counting_app(cache: &Cache) -> (Ultimo, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(cache.middleware());

    let c = calls.clone();
    app.get("/items", move |ctx: Context| {
        let c = c.clone();
        async move {
            let n = c.fetch_add(1, Ordering::SeqCst) + 1;
            let q = ctx.req.query("q").unwrap_or_default();
            let lang = ctx.req.header("accept-language").unwrap_or_default();
            ctx.header("x-custom", "kept").await;
            ctx.text(format!("call {n} q={q} lang={lang}")).await
        }
    });
    let c = calls.clone();
    app.get("/items/:id", move |ctx: Context| {
        let c = c.clone();
        async move {
            let n = c.fetch_add(1, Ordering::SeqCst) + 1;
            ctx.text(format!("item {} call {n}", ctx.req.param("id")?))
                .await
        }
    });
    let c = calls.clone();
    app.post("/items", move |ctx: Context| {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            ctx.text("created").await
        }
    });
    let c = calls.clone();
    app.get("/missing", move |_ctx: Context| {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            Err(ultimo::UltimoError::NotFound("nope".into()))
        }
    });
    let c = calls.clone();
    app.get("/private", move |ctx: Context| {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            ctx.header("cache-control", "private, max-age=60").await;
            ctx.text("secret").await
        }
    });
    let c = calls.clone();
    app.get("/cookie", move |ctx: Context| {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            ctx.header("set-cookie", "sid=abc").await;
            ctx.text("with cookie").await
        }
    });
    let c = calls.clone();
    app.get("/varies", move |ctx: Context| {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            ctx.header("vary", "X-Unknown").await;
            ctx.text("varies").await
        }
    });
    let c = calls.clone();
    app.get("/big", move |ctx: Context| {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            ctx.text("x".repeat(2048)).await
        }
    });
    let c = calls.clone();
    app.get("/stream", move |ctx: Context| {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            let chunks: Vec<Result<bytes::Bytes, ultimo::response::BoxError>> =
                vec![Ok(bytes::Bytes::from("streamed"))];
            let chunks = futures_util::stream::iter(chunks);
            ctx.stream(chunks).await
        }
    });
    (app, calls)
}

fn calls(c: &AtomicUsize) -> usize {
    c.load(Ordering::SeqCst)
}

#[tokio::test]
async fn second_get_is_served_from_cache() {
    let cache = Cache::new(Duration::from_secs(60));
    let (app, n) = counting_app(&cache);
    let client = TestClient::new(app);

    let a = client.get("/items").send().await;
    assert_eq!(a.status(), 200);
    assert_eq!(a.header("x-cache"), Some("MISS"));
    assert_eq!(a.text(), "call 1 q= lang=");

    let b = client.get("/items").send().await;
    assert_eq!(b.status(), 200);
    assert_eq!(b.header("x-cache"), Some("HIT"));
    assert_eq!(b.text(), "call 1 q= lang=");
    assert_eq!(b.header("x-custom"), Some("kept"));
    assert_eq!(b.header("content-type"), a.header("content-type"));
    assert!(b.header("age").is_some());
    assert_eq!(calls(&n), 1);
}

#[tokio::test]
async fn query_string_is_part_of_the_key() {
    let cache = Cache::new(Duration::from_secs(60));
    let (app, n) = counting_app(&cache);
    let client = TestClient::new(app);

    assert_eq!(
        client.get("/items?q=a").send().await.text(),
        "call 1 q=a lang="
    );
    assert_eq!(
        client.get("/items?q=b").send().await.text(),
        "call 2 q=b lang="
    );
    assert_eq!(
        client.get("/items?q=a").send().await.text(),
        "call 1 q=a lang="
    );
    assert_eq!(calls(&n), 2);
}

#[tokio::test]
async fn entries_expire_after_ttl() {
    let cache = Cache::new(Duration::from_millis(100));
    let (app, n) = counting_app(&cache);
    let client = TestClient::new(app);

    client.get("/items").send().await;
    client.get("/items").send().await;
    assert_eq!(calls(&n), 1);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let res = client.get("/items").send().await;
    assert_eq!(res.header("x-cache"), Some("MISS"));
    assert_eq!(calls(&n), 2);
}

#[tokio::test]
async fn only_successful_buffered_cacheable_gets_are_stored() {
    let cache = Cache::new(Duration::from_secs(60)).max_body_size(1024);
    let (app, n) = counting_app(&cache);
    let client = TestClient::new(app);

    for path in [
        "/missing", "/private", "/cookie", "/varies", "/big", "/stream",
    ] {
        let before = calls(&n);
        let first = client.get(path).send().await;
        let second = client.get(path).send().await;
        assert_eq!(calls(&n), before + 2, "{path} must not be cached");
        assert_ne!(second.header("x-cache"), Some("HIT"), "{path}");
        assert_eq!(first.status(), second.status(), "{path}");
    }
    assert_eq!(
        client.get("/stream").send().await.text(),
        "streamed",
        "streams pass through intact"
    );

    let before = calls(&n);
    client.post("/items").send().await;
    client.post("/items").send().await;
    assert_eq!(calls(&n), before + 2, "POST is never cached");
}

#[tokio::test]
async fn credentialed_requests_bypass_unless_public() {
    let cache = Cache::new(Duration::from_secs(60));
    let (app, n) = counting_app(&cache);
    let client = TestClient::new(app);

    // A cached anonymous response must not be served to a logged-in user, and
    // a logged-in user's response must not be stored.
    client.get("/items").send().await;
    let res = client.get("/items").header("cookie", "sid=1").send().await;
    assert_eq!(res.header("x-cache"), None);
    client.get("/items").bearer("t").send().await;
    assert_eq!(calls(&n), 3);

    let cache = Cache::new(Duration::from_secs(60)).public();
    let (app, n) = counting_app(&cache);
    let client = TestClient::new(app);
    client.get("/items").header("cookie", "sid=1").send().await;
    client.get("/items").header("cookie", "sid=2").send().await;
    assert_eq!(calls(&n), 1);
}

#[tokio::test]
async fn varied_headers_are_part_of_the_key() {
    let cache = Cache::new(Duration::from_secs(60)).vary(["Accept-Language"]);
    let (app, n) = counting_app(&cache);
    let client = TestClient::new(app);

    let en = client
        .get("/items")
        .header("accept-language", "en")
        .send()
        .await;
    let de = client
        .get("/items")
        .header("accept-language", "de")
        .send()
        .await;
    let en2 = client
        .get("/items")
        .header("accept-language", "en")
        .send()
        .await;
    assert_eq!(en.text(), "call 1 q= lang=en");
    assert_eq!(de.text(), "call 2 q= lang=de");
    assert_eq!(en2.text(), "call 1 q= lang=en");
    assert_eq!(calls(&n), 2);
}

#[tokio::test]
async fn vary_on_cookie_caches_per_user() {
    let cache = Cache::new(Duration::from_secs(60)).vary(["cookie"]);
    let (app, n) = counting_app(&cache);
    let client = TestClient::new(app);

    client.get("/items").header("cookie", "sid=1").send().await;
    client.get("/items").header("cookie", "sid=2").send().await;
    let again = client.get("/items").header("cookie", "sid=1").send().await;
    assert_eq!(again.header("x-cache"), Some("HIT"));
    assert_eq!(calls(&n), 2);
}

#[tokio::test]
async fn invalidate_drops_every_variant_of_a_path_only() {
    let cache = Cache::new(Duration::from_secs(60)).vary(["accept-language"]);
    let (app, n) = counting_app(&cache);
    let client = TestClient::new(app);

    client.get("/items?q=a").send().await;
    client
        .get("/items")
        .header("accept-language", "de")
        .send()
        .await;
    client.get("/items/1").send().await;
    assert_eq!(calls(&n), 3);

    cache.invalidate("/items").await.unwrap();

    let a = client.get("/items?q=a").send().await;
    assert_eq!(a.header("x-cache"), Some("MISS"));
    let de = client
        .get("/items")
        .header("accept-language", "de")
        .send()
        .await;
    assert_eq!(de.header("x-cache"), Some("MISS"));
    let item = client.get("/items/1").send().await;
    assert_eq!(
        item.header("x-cache"),
        Some("HIT"),
        "other paths stay cached"
    );
    assert_eq!(calls(&n), 5);

    cache.clear().await.unwrap();
    let item = client.get("/items/1").send().await;
    assert_eq!(item.header("x-cache"), Some("MISS"));
}

#[tokio::test]
async fn scopes_keep_caches_apart_in_one_store() {
    let store = MemoryCacheStore::new();
    let a = Cache::new(Duration::from_secs(60))
        .store(store.clone())
        .scope("a");
    let b = Cache::new(Duration::from_secs(60))
        .store(store.clone())
        .scope("b");
    let (app_a, na) = counting_app(&a);
    let (app_b, nb) = counting_app(&b);
    let (ca, cb) = (TestClient::new(app_a), TestClient::new(app_b));

    ca.get("/items").send().await;
    cb.get("/items").send().await;
    assert_eq!((calls(&na), calls(&nb)), (1, 1));

    a.clear().await.unwrap();
    assert_eq!(cb.get("/items").send().await.header("x-cache"), Some("HIT"));
    assert_eq!(
        ca.get("/items").send().await.header("x-cache"),
        Some("MISS")
    );
}

#[tokio::test]
async fn shared_store_serves_another_instance() {
    let store = MemoryCacheStore::new();
    let a = Cache::new(Duration::from_secs(60)).store(store.clone());
    let b = Cache::new(Duration::from_secs(60)).store(store);
    let (app_a, na) = counting_app(&a);
    let (app_b, nb) = counting_app(&b);

    TestClient::new(app_a).get("/items").send().await;
    let res = TestClient::new(app_b).get("/items").send().await;
    assert_eq!(res.header("x-cache"), Some("HIT"));
    assert_eq!((calls(&na), calls(&nb)), (1, 0));
}

#[tokio::test]
async fn memory_store_is_bounded() {
    let store = MemoryCacheStore::new().max_entries(2);
    let entry = || CachedResponse::new(200, vec![], bytes::Bytes::from_static(b"x"));
    let ttl = Duration::from_secs(60);
    store.set("k1", entry(), Duration::from_secs(10)).await;
    store.set("k2", entry(), ttl).await;
    store.set("k3", entry(), ttl).await;
    assert_eq!(store.len(), 2);
    // The entry closest to expiry is evicted first.
    assert!(store.get("k1").await.is_none());
    assert!(store.get("k2").await.is_some());
    assert!(store.get("k3").await.is_some());
}

#[test]
fn cached_response_encoding_roundtrips() {
    let res = CachedResponse::new(
        200,
        vec![
            ("content-type".into(), b"text/plain".to_vec()),
            ("x-bin".into(), vec![0, 255, 10]),
        ],
        bytes::Bytes::from_static(b"hello\0world"),
    );
    let decoded = CachedResponse::decode(&res.encode()).expect("decodes");
    assert_eq!(decoded, res);
    assert_eq!(decoded.status(), 200);
    assert_eq!(decoded.body().as_ref(), b"hello\0world");

    assert!(CachedResponse::decode(b"").is_none());
    assert!(CachedResponse::decode(b"garbage that is not an entry").is_none());
    let mut truncated = res.encode();
    truncated.pop();
    assert!(CachedResponse::decode(&truncated).is_none());
}
