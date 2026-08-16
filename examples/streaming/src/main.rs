//! Streaming response demo: a route that streams numbered lines, served with a
//! tiny HTML page that reads the stream via `fetch`.
use hyper::body::Bytes;
use std::time::Duration;
use ultimo::prelude::*;
use ultimo::response::BoxError;

const PAGE: &str = r#"<!doctype html>
<meta charset="utf-8"><title>Ultimo streaming demo</title>
<h1>Streaming demo</h1>
<button onclick="go()">Stream</button>
<pre id="out"></pre>
<script>
async function go() {
  const out = document.getElementById('out');
  out.textContent = '';
  const res = await fetch('/numbers');
  const reader = res.body.getReader();
  const dec = new TextDecoder();
  for (;;) {
    const { value, done } = await reader.read();
    if (done) break;
    out.textContent += dec.decode(value);
  }
}
</script>"#;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = Ultimo::new();

    app.get("/", |ctx: Context| async move { ctx.html(PAGE).await });

    app.get("/numbers", |ctx: Context| async move {
        // A stream that yields a line every 300ms, ten times.
        let s = futures_util::stream::unfold(0u32, |n| async move {
            if n >= 10 {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
            let chunk: std::result::Result<Bytes, BoxError> =
                Ok(Bytes::from(format!("line {n}\n")));
            Some((chunk, n + 1))
        });
        ctx.stream(s).await
    });

    println!("→ http://127.0.0.1:3000");
    app.listen("127.0.0.1:3000").await
}
