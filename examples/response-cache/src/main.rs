//! A slow endpoint behind the response cache. Run with
//! `cargo run -p response-cache-example`, then open http://localhost:3000.
//!
//! Entries live in memory; set `REDIS_URL` (e.g. `redis://127.0.0.1:6379`) to
//! keep them in Redis instead, shared by every instance of the app.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use ultimo::cache::Cache;
use ultimo::prelude::*;
use ultimo::redis::{connect, RedisCacheStore};

const PAGE: &str = r#"<!doctype html>
<meta charset="utf-8"><title>Ultimo response cache demo</title>
<h1>Response cache demo</h1>
<p><code>GET /report</code> takes a second to build and is cached for 30 s.</p>
<p>
  <button onclick="load()">Fetch /report</button>
  <button onclick="update()">Change the data (POST /report)</button>
</p>
<pre id="log"></pre>
<script>
const log = (line) => (document.getElementById('log').textContent += line + '\n');
async function load() {
  const start = performance.now();
  const res = await fetch('/report');
  const body = await res.json();
  const ms = Math.round(performance.now() - start);
  log(`${res.headers.get('x-cache')}  ${ms} ms  version ${body.version}, built ${body.built_at}` +
      (res.headers.get('age') ? `, age ${res.headers.get('age')} s` : ''));
}
async function update() {
  const body = await (await fetch('/report', { method: 'POST' })).json();
  log(`-- data changed to version ${body.version}; cache invalidated`);
}
</script>"#;

#[tokio::main]
async fn main() -> Result<()> {
    let mut cache = Cache::new(Duration::from_secs(30));
    let backend = match std::env::var("REDIS_URL") {
        Ok(url) => {
            cache = cache.store(RedisCacheStore::new(connect(&url).await?));
            format!("Redis at {url}")
        }
        Err(_) => "memory".to_string(),
    };

    let version = Arc::new(AtomicU64::new(1));
    let mut app = Ultimo::new();
    app.use_middleware(cache.middleware());

    app.get("/", |ctx: Context| async move { ctx.html(PAGE).await });

    let v = version.clone();
    app.get("/report", move |ctx: Context| {
        let v = v.clone();
        async move {
            // Stand-in for an expensive query.
            tokio::time::sleep(Duration::from_secs(1)).await;
            let built_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            ctx.json(json!({ "version": v.load(Ordering::SeqCst), "built_at": built_at }))
                .await
        }
    });

    app.post("/report", move |ctx: Context| {
        let (version, cache) = (version.clone(), cache.clone());
        async move {
            let new = version.fetch_add(1, Ordering::SeqCst) + 1;
            cache.invalidate("/report").await?;
            ctx.json(json!({ "version": new })).await
        }
    });

    println!("→ http://localhost:3000   (cache in {backend})");
    app.listen("127.0.0.1:3000").await
}
