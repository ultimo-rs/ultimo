#![cfg(all(feature = "otel", feature = "testing"))]

//! The OpenTelemetry globals (tracer provider, propagator) are process-wide, so
//! every test shares one in-memory exporter and finds its own span by a route
//! that no other test uses.

use opentelemetry::trace::{SpanKind, Status, TraceContextExt};
use opentelemetry::{global, Value};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider, SpanData};
use std::sync::OnceLock;
use std::time::Duration;
use ultimo::middleware::builtin;
use ultimo::otel::Tracing;
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo, UltimoError};

fn exporter() -> &'static InMemorySpanExporter {
    static EXPORTER: OnceLock<InMemorySpanExporter> = OnceLock::new();
    EXPORTER.get_or_init(|| {
        let exporter = InMemorySpanExporter::default();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter.clone())
            .build();
        global::set_tracer_provider(provider);
        global::set_text_map_propagator(TraceContextPropagator::new());
        exporter
    })
}

/// The finished server span whose `http.route` is `route`.
fn span_for(route: &str) -> SpanData {
    let spans = exporter().get_finished_spans().expect("finished spans");
    spans
        .into_iter()
        .find(|s| attr(s, "http.route") == Some(Value::from(route.to_string())))
        .unwrap_or_else(|| panic!("no span with http.route={route}"))
}

fn attr(span: &SpanData, key: &str) -> Option<Value> {
    span.attributes
        .iter()
        .find(|kv| kv.key.as_str() == key)
        .map(|kv| kv.value.clone())
}

fn app() -> Ultimo {
    exporter();
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(Tracing::new().middleware());
    app
}

#[tokio::test]
async fn records_a_server_span_named_by_route_pattern() {
    let mut app = app();
    app.get("/otel/users/:id", |ctx: Context| async move {
        ctx.text(ctx.req.param("id")?.to_string()).await
    });
    let client = TestClient::new(app);

    let res = client
        .get("/otel/users/42")
        .header("user-agent", "otel-test")
        .send()
        .await;
    assert_eq!(res.status(), 200);

    let span = span_for("/otel/users/:id");
    assert_eq!(span.name, "GET /otel/users/:id");
    assert_eq!(span.span_kind, SpanKind::Server);
    assert_eq!(attr(&span, "http.request.method"), Some("GET".into()));
    assert_eq!(attr(&span, "url.path"), Some("/otel/users/42".into()));
    assert_eq!(attr(&span, "user_agent.original"), Some("otel-test".into()));
    assert_eq!(
        attr(&span, "http.response.status_code"),
        Some(200_i64.into())
    );
    assert_eq!(span.status, Status::Unset);
}

#[tokio::test]
async fn continues_an_incoming_w3c_trace() {
    let mut app = app();
    app.get("/otel/traced", |ctx: Context| async move {
        ctx.text("ok").await
    });
    let client = TestClient::new(app);

    let trace_id = "4bf92f3577b34da6a3ce929d0e0e4736";
    let parent_id = "00f067aa0ba902b7";
    client
        .get("/otel/traced")
        .header("traceparent", &format!("00-{trace_id}-{parent_id}-01"))
        .send()
        .await;

    let span = span_for("/otel/traced");
    assert_eq!(span.span_context.trace_id().to_string(), trace_id);
    assert_eq!(span.parent_span_id.to_string(), parent_id);
}

#[tokio::test]
async fn handler_sees_the_server_span_as_current() {
    let mut app = app();
    app.get("/otel/current", |ctx: Context| async move {
        let cx = opentelemetry::Context::current();
        let id = cx.span().span_context().span_id().to_string();
        ctx.text(id).await
    });
    let client = TestClient::new(app);

    let res = client.get("/otel/current").send().await;
    let span = span_for("/otel/current");
    assert_eq!(res.text(), span.span_context.span_id().to_string());
}

#[tokio::test]
async fn server_errors_mark_the_span_as_error() {
    let mut app = app();
    app.get("/otel/boom", |_ctx: Context| async move {
        Err::<ultimo::response::Response, _>(UltimoError::Internal("boom".into()))
    });
    app.get("/otel/missing", |_ctx: Context| async move {
        Err::<ultimo::response::Response, _>(UltimoError::NotFound("nope".into()))
    });
    let client = TestClient::new(app);

    client.get("/otel/boom").send().await;
    client.get("/otel/missing").send().await;

    let boom = span_for("/otel/boom");
    assert_eq!(
        attr(&boom, "http.response.status_code"),
        Some(500_i64.into())
    );
    assert_eq!(attr(&boom, "error.type"), Some("500".into()));
    assert!(
        matches!(boom.status, Status::Error { .. }),
        "{:?}",
        boom.status
    );

    // 4xx is the client's fault: not an error on a server span.
    let missing = span_for("/otel/missing");
    assert_eq!(
        attr(&missing, "http.response.status_code"),
        Some(404_i64.into())
    );
    assert_eq!(attr(&missing, "error.type"), None);
    assert_eq!(missing.status, Status::Unset);
}

#[tokio::test]
async fn records_the_request_id() {
    let mut app = app();
    app.use_middleware(builtin::request_id());
    app.get(
        "/otel/rid",
        |ctx: Context| async move { ctx.text("ok").await },
    );
    let client = TestClient::new(app);

    client
        .get("/otel/rid")
        .header("x-request-id", "req-abc-123")
        .send()
        .await;

    let span = span_for("/otel/rid");
    assert_eq!(attr(&span, "ultimo.request_id"), Some("req-abc-123".into()));
}

#[tokio::test]
async fn timed_out_requests_still_end_their_span() {
    let mut app = app();
    app.request_timeout(Duration::from_millis(20));
    app.get("/otel/slow", |ctx: Context| async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        ctx.text("late").await
    });
    let client = TestClient::new(app);

    let res = client.get("/otel/slow").send().await;
    assert_eq!(res.status(), 408);

    // The timeout drops the middleware future; the span still ends and is
    // marked as cancelled (no status code was ever produced inside it).
    let span = span_for("/otel/slow");
    assert_eq!(attr(&span, "error.type"), Some("cancelled".into()));
    assert_eq!(attr(&span, "http.response.status_code"), None);
}
