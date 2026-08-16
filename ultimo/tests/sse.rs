//! Integration tests for Server-Sent Events.
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::Request as HyperRequest;
use ultimo::prelude::*;
use ultimo::SseEvent;

fn get(uri: &str) -> HyperRequest<Full<Bytes>> {
    HyperRequest::builder()
        .uri(uri)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

async fn body(resp: ultimo::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn ctx_sse_sets_headers_and_encodes_events() {
    let mut app = Ultimo::new_without_defaults();
    app.get("/events", |ctx: Context| async move {
        let evs = vec![
            SseEvent::new(&json!({ "n": 1 })).unwrap(),
            SseEvent::new(&json!({ "n": 2 })).unwrap().event("tick"),
        ];
        ctx.sse(futures_util::stream::iter(evs)).await
    });

    let resp = app.oneshot(get("/events")).await;
    assert_eq!(
        resp.headers()
            .get(hyper::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );
    assert_eq!(
        resp.headers()
            .get(hyper::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok()),
        Some("no-cache")
    );
    assert!(resp.headers().get(hyper::header::CONTENT_LENGTH).is_none());
    assert_eq!(
        body(resp).await,
        "data: {\"n\":1}\n\nevent: tick\ndata: {\"n\":2}\n\n"
    );
}

#[tokio::test]
async fn ctx_last_event_id_reads_header() {
    let mut app = Ultimo::new_without_defaults();
    app.get("/resume", |ctx: Context| async move {
        ctx.text(ctx.last_event_id().unwrap_or_else(|| "none".into()))
            .await
    });

    let req = HyperRequest::builder()
        .uri("/resume")
        .header("Last-Event-ID", "42")
        .body(Full::new(Bytes::new()))
        .unwrap();
    assert_eq!(body(app.oneshot(req).await).await, "42");
}

#[tokio::test]
async fn keep_alive_passes_events_through_and_terminates() {
    use std::time::Duration;
    let mut app = Ultimo::new_without_defaults();
    app.get("/ka", |ctx: Context| async move {
        let evs = vec![SseEvent::data("a"), SseEvent::data("b")];
        // Long interval → no ping fires; stream still ends when events end.
        ctx.sse_keep_alive(futures_util::stream::iter(evs), Duration::from_secs(60))
            .await
    });
    assert_eq!(
        body(app.oneshot(get("/ka")).await).await,
        "data: a\n\ndata: b\n\n"
    );
}

#[tokio::test]
async fn keep_alive_injects_ping_when_idle() {
    use std::time::Duration;
    let mut app = Ultimo::new_without_defaults();
    app.get("/ka", |ctx: Context| async move {
        use futures_util::StreamExt;
        // One event, then a gap longer than the ping interval, then end.
        let s = futures_util::stream::once(async { SseEvent::data("x") }).chain(
            futures_util::stream::once(async {
                tokio::time::sleep(Duration::from_millis(60)).await;
                SseEvent::data("y")
            }),
        );
        ctx.sse_keep_alive(s, Duration::from_millis(10)).await
    });
    let out = body(app.oneshot(get("/ka")).await).await;
    assert!(out.starts_with("data: x\n\n"), "got: {out:?}");
    assert!(
        out.contains(": ping\n\n"),
        "expected a ping comment, got: {out:?}"
    );
    assert!(out.ends_with("data: y\n\n"), "got: {out:?}");
}

#[tokio::test]
async fn sse_channel_delivers_pushed_events() {
    use ultimo::sse_channel;
    let mut app = Ultimo::new_without_defaults();
    app.get("/push", |ctx: Context| async move {
        let (tx, rx) = sse_channel();
        tx.send(SseEvent::data("a")).unwrap();
        tx.send(SseEvent::data("b")).unwrap();
        drop(tx); // end the stream so the response completes
        ctx.sse(rx).await
    });
    assert_eq!(
        body(app.oneshot(get("/push")).await).await,
        "data: a\n\ndata: b\n\n"
    );
}

#[tokio::test]
async fn app_sse_registers_a_get_route() {
    let mut app = Ultimo::new_without_defaults();
    app.sse("/events", |ctx: Context| async move {
        ctx.sse(futures_util::stream::iter(vec![SseEvent::data("hi")]))
            .await
    });
    let resp = app.oneshot(get("/events")).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(body(resp).await, "data: hi\n\n");
}
