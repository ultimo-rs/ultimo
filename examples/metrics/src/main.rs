//! Prometheus metrics demo: fire requests from the browser and watch the
//! `/metrics` output update. Run with `cargo run -p metrics-example`.
use std::time::Duration;
use ultimo::metrics::Metrics;
use ultimo::prelude::*;

const PAGE: &str = r#"<!doctype html>
<meta charset="utf-8"><title>Ultimo metrics demo</title>
<h1>Prometheus metrics demo</h1>
<p>
  <button onclick="hit('/users/' + Math.ceil(Math.random() * 100))">GET /users/:id</button>
  <button onclick="hit('/slow')">GET /slow (~300 ms)</button>
  <button onclick="hit('/fail')">GET /fail (500)</button>
</p>
<p>Note how many different user ids all land on one <code>route="/users/:id"</code> series.</p>
<pre id="out"></pre>
<script>
async function refresh() {
  const text = await (await fetch('/metrics')).text();
  document.getElementById('out').textContent = text
    .split('\n')
    .filter((l) => !l.startsWith('http_request_duration_seconds_bucket'))
    .join('\n');
}
async function hit(path) { await fetch(path); refresh(); }
refresh();
setInterval(refresh, 2000);
</script>"#;

#[tokio::main]
async fn main() -> Result<()> {
    let metrics = Metrics::new();
    let mut app = Ultimo::new();

    // Register first so latency covers the rest of the chain.
    app.use_middleware(metrics.middleware());
    app.get("/metrics", metrics.handler());

    app.get("/", |ctx: Context| async move { ctx.html(PAGE).await });
    app.get("/users/:id", |ctx: Context| async move {
        let id = ctx.req.param("id")?.to_string();
        ctx.json(json!({ "id": id })).await
    });
    app.get("/slow", |ctx: Context| async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        ctx.text("done").await
    });
    app.get("/fail", |_ctx: Context| async move {
        Err::<ultimo::response::Response, _>(UltimoError::Internal("boom".into()))
    });

    println!("→ http://127.0.0.1:3000   (scrape target: /metrics)");
    app.listen("127.0.0.1:3000").await
}
