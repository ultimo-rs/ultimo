//! SSE demo: a server-pushed counter feed consumed by a browser EventSource.
use std::time::Duration;
use ultimo::prelude::*;
use ultimo::{sse_channel, SseEvent};

const PAGE: &str = r#"<!doctype html>
<meta charset="utf-8"><title>Ultimo SSE demo</title>
<h1>SSE demo</h1>
<pre id="out"></pre>
<script>
const es = new EventSource('/events');
es.addEventListener('tick', (e) => {
  const { n } = JSON.parse(e.data);
  document.getElementById('out').textContent += `tick ${n}\n`;
});
</script>"#;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = Ultimo::new();

    app.get("/", |ctx: Context| async move { ctx.html(PAGE).await });

    app.sse("/events", |ctx: Context| async move {
        let (tx, rx) = sse_channel();
        tokio::spawn(async move {
            let mut n = 0u32;
            loop {
                let ev = SseEvent::new(&serde_json::json!({ "n": n }))
                    .unwrap()
                    .event("tick")
                    .id(n.to_string());
                if tx.send(ev).is_err() {
                    break; // client disconnected
                }
                n += 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
        ctx.sse_keep_alive(rx, Duration::from_secs(15)).await
    });

    println!("→ http://127.0.0.1:3000");
    app.listen("127.0.0.1:3000").await
}
