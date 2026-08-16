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
