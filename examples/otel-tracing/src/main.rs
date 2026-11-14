//! OpenTelemetry tracing demo: the browser sends a W3C `traceparent`, the
//! server continues that trace, and a handler adds a child span. Spans are kept
//! in memory and listed at `/spans`. Run with `cargo run -p otel-tracing-example`.
//!
//! A real app would export with `opentelemetry-otlp` to Jaeger, Tempo, Honeycomb
//! or any OTLP collector instead of the in-memory exporter used here.
use opentelemetry::trace::{Span, TraceContextExt, Tracer};
use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider};
use std::time::Duration;
use ultimo::middleware::builtin;
use ultimo::otel::Tracing;
use ultimo::prelude::*;

const PAGE: &str = r#"<!doctype html>
<meta charset="utf-8"><title>Ultimo OpenTelemetry demo</title>
<h1>OpenTelemetry tracing demo</h1>
<p>
  <button onclick="call(true)">GET /users/:id with a new traceparent</button>
  <button onclick="call(false)">GET /users/:id without one</button>
  <button onclick="fail()">GET /fail (500)</button>
</p>
<p id="result"></p>
<h2>Recent spans</h2>
<pre id="spans"></pre>
<script>
const hex = (bytes) => [...crypto.getRandomValues(new Uint8Array(bytes))]
  .map((b) => b.toString(16).padStart(2, '0')).join('');

async function call(withParent) {
  const headers = {};
  let sent = '';
  if (withParent) {
    const traceId = hex(16);
    headers.traceparent = `00-${traceId}-${hex(8)}-01`;
    sent = `sent trace ${traceId} · `;
  }
  const id = Math.ceil(Math.random() * 100);
  const body = await (await fetch(`/users/${id}`, { headers })).json();
  document.getElementById('result').textContent =
    `${sent}server span ran in trace ${body.trace_id}`;
  refresh();
}
async function fail() { await fetch('/fail'); refresh(); }
async function refresh() {
  const spans = await (await fetch('/spans')).json();
  document.getElementById('spans').textContent = JSON.stringify(spans, null, 2);
}
refresh();
</script>"#;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Install an SDK + W3C propagator once, at startup.
    let exporter = InMemorySpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    global::set_tracer_provider(provider);
    global::set_text_map_propagator(TraceContextPropagator::new());

    // 2. Register the tracing middleware first so the span covers the chain.
    let mut app = Ultimo::new();
    app.use_middleware(Tracing::with_scope_name("otel-demo").middleware());
    app.use_middleware(builtin::request_id());

    app.get("/", |ctx: Context| async move { ctx.html(PAGE).await });

    app.get("/users/:id", |ctx: Context| async move {
        let id = ctx.req.param("id")?.to_string();
        // The server span is current here, so this span becomes its child.
        let mut span = global::tracer("otel-demo").start("load user");
        span.set_attribute(KeyValue::new("user.id", id));
        tokio::time::sleep(Duration::from_millis(20)).await;
        span.end();
        let trace_id = opentelemetry::Context::current()
            .span()
            .span_context()
            .trace_id()
            .to_string();
        ctx.json(json!({ "trace_id": trace_id })).await
    });

    app.get("/fail", |_ctx: Context| async move {
        Err::<ultimo::response::Response, _>(UltimoError::Internal("boom".into()))
    });

    // 3. Demo only: read finished spans back from the in-memory exporter.
    app.get("/spans", move |ctx: Context| {
        let exporter = exporter.clone();
        async move {
            let spans = exporter.get_finished_spans().unwrap_or_default();
            let recent: Vec<_> = spans
                .iter()
                .rev()
                // Hide the demo's own page and polling requests.
                .filter(|s| s.name != "GET /" && s.name != "GET /spans")
                .take(12)
                .map(|s| {
                    let attributes: serde_json::Map<_, _> = s
                        .attributes
                        .iter()
                        .map(|kv| (kv.key.to_string(), json!(kv.value.to_string())))
                        .collect();
                    json!({
                        "name": s.name,
                        "trace_id": s.span_context.trace_id().to_string(),
                        "span_id": s.span_context.span_id().to_string(),
                        "parent_span_id": s.parent_span_id.to_string(),
                        "status": format!("{:?}", s.status),
                        "attributes": attributes,
                    })
                })
                .collect();
            ctx.json(recent).await
        }
    });

    println!("→ http://127.0.0.1:3000");
    app.listen("127.0.0.1:3000").await
}
