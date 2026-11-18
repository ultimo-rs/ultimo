//! Request guards and geo-blocking. Run with `cargo run -p request-guard-example`,
//! then open http://localhost:3000.
//!
//! The app pretends to sit behind Cloudflare (`trust_proxy(true)`), so the page
//! can play the CDN and send `CF-IPCountry` itself. In production, enable
//! `trust_proxy` only when a real proxy sets these headers.
use ultimo::guard::{Decision, GeoFilter, Guard};
use ultimo::prelude::*;

const PAGE: &str = r#"<!doctype html>
<meta charset="utf-8"><title>Ultimo request guard demo</title>
<h1>Request guard demo</h1>
<p>Rules: scanners (user agent containing <code>sqlmap</code>) are blocked,
<code>/admin</code> answers 404 without <code>X-Internal</code>, and only
US / CA / DE visitors (from <code>CF-IPCountry</code>) get in.</p>
<p>
  <button onclick="send('/api/hello', { 'CF-IPCountry': 'US' })">Visitor from US</button>
  <button onclick="send('/api/hello', { 'CF-IPCountry': 'de' })">Visitor from Germany</button>
  <button onclick="send('/api/hello', { 'CF-IPCountry': 'FR' })">Visitor from France</button>
  <button onclick="send('/api/hello', {})">Unknown country</button>
</p>
<p>
  <button onclick="send('/api/hello', { 'CF-IPCountry': 'US', 'X-Scanner': 'sqlmap/1.8' })">Scanner from US</button>
  <button onclick="send('/admin', { 'CF-IPCountry': 'US' })">/admin</button>
  <button onclick="send('/admin', { 'CF-IPCountry': 'US', 'X-Internal': '1' })">/admin + X-Internal</button>
</p>
<pre id="log"></pre>
<script>
async function send(path, headers) {
  const res = await fetch(path, { headers });
  const body = await res.text();
  document.getElementById('log').textContent +=
    `${res.status}  ${path}  ${JSON.stringify(headers)}  → ${body}\n`;
}
</script>"#;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = Ultimo::new();
    app.trust_proxy(true);

    // The page itself is reachable from anywhere.
    app.get("/", |ctx: Context| async move { ctx.html(PAGE).await });

    // Rule 1: block scanners. Browsers won't let a page change User-Agent, so
    // the demo also checks an `X-Scanner` header.
    app.use_middleware(
        Guard::new(|ctx: &Context| {
            let ua = ctx.req.header("user-agent").unwrap_or_default()
                + &ctx.req.header("x-scanner").unwrap_or_default();
            Decision::deny_if(ua.contains("sqlmap") || ua.contains("nikto"))
        })
        .build(),
    );

    // Rule 2: hide /admin unless the request carries the internal header.
    app.use_middleware(
        Guard::new(|ctx: &Context| {
            if ctx.req.path().starts_with("/admin") && ctx.req.header("x-internal").is_none() {
                Decision::respond(404, "Not Found")
            } else {
                Decision::Allow
            }
        })
        .build(),
    );

    // Rule 3: geo allow-list on everything except the demo page.
    let geo = GeoFilter::allow(&["US", "CA", "DE"])
        .header("CF-IPCountry")
        .build();
    app.use_middleware(std::sync::Arc::new(move |ctx: Context, next| {
        if ctx.req.path() == "/" {
            return next(ctx);
        }
        geo(ctx, next)
    }));

    app.get("/api/hello", |ctx: Context| async move {
        let country = ctx.country().await;
        ctx.json(json!({ "hello": "world", "country": country }))
            .await
    });
    app.get("/admin", |ctx: Context| async move {
        ctx.text("admin area").await
    });

    println!("→ http://localhost:3000");
    app.listen("127.0.0.1:3000").await
}
