//! Integration tests for streaming response bodies.
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::Request as HyperRequest;
use ultimo::prelude::*;
use ultimo::response::BoxError;

fn empty_req(uri: &str) -> HyperRequest<Full<Bytes>> {
    HyperRequest::builder()
        .uri(uri)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

#[tokio::test]
async fn ctx_stream_sends_chunks_without_content_length() {
    let mut app = Ultimo::new_without_defaults();
    app.get("/stream", |ctx: Context| async move {
        let chunks: Vec<std::result::Result<Bytes, BoxError>> = vec![
            Ok(Bytes::from("chunk-1;")),
            Ok(Bytes::from("chunk-2;")),
            Ok(Bytes::from("chunk-3")),
        ];
        ctx.stream(futures_util::stream::iter(chunks)).await
    });

    let resp = app.oneshot(empty_req("/stream")).await;
    assert_eq!(resp.status(), 200);
    assert!(
        resp.headers().get(hyper::header::CONTENT_LENGTH).is_none(),
        "streamed responses must not carry Content-Length"
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"chunk-1;chunk-2;chunk-3");
}

#[tokio::test]
async fn ctx_stream_drops_queued_content_length() {
    let mut app = Ultimo::new_without_defaults();
    app.get("/stream", |ctx: Context| async move {
        // A stale/incorrect length that must not survive onto a chunked body.
        ctx.header("Content-Length", "1024").await;
        let chunks: Vec<std::result::Result<Bytes, BoxError>> = vec![Ok(Bytes::from("abc"))];
        ctx.stream(futures_util::stream::iter(chunks)).await
    });

    let resp = app.oneshot(empty_req("/stream")).await;
    assert!(
        resp.headers().get(hyper::header::CONTENT_LENGTH).is_none(),
        "a queued Content-Length must be dropped for a chunked streaming body"
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"abc");
}

#[tokio::test]
async fn ctx_stream_defaults_content_type_to_octet_stream() {
    let mut app = Ultimo::new_without_defaults();
    app.get("/stream", |ctx: Context| async move {
        let chunks: Vec<std::result::Result<Bytes, BoxError>> = vec![Ok(Bytes::from("x"))];
        ctx.stream(futures_util::stream::iter(chunks)).await
    });

    let resp = app.oneshot(empty_req("/stream")).await;
    assert_eq!(
        resp.headers()
            .get(hyper::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/octet-stream"),
        "a streamed body with no explicit type should default to octet-stream"
    );
}

#[tokio::test]
async fn ctx_stream_preserves_explicit_content_type() {
    let mut app = Ultimo::new_without_defaults();
    app.get("/stream", |ctx: Context| async move {
        ctx.header("Content-Type", "text/csv").await;
        let chunks: Vec<std::result::Result<Bytes, BoxError>> = vec![Ok(Bytes::from("a,b\n1,2\n"))];
        ctx.stream(futures_util::stream::iter(chunks)).await
    });

    let resp = app.oneshot(empty_req("/stream")).await;
    assert_eq!(
        resp.headers()
            .get(hyper::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("text/csv"),
        "an explicit Content-Type must not be overridden by the default"
    );
}

#[cfg(feature = "compression")]
#[tokio::test]
async fn compression_passes_streaming_bodies_through() {
    use ultimo::middleware::builtin::compression;
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/stream", |ctx: Context| async move {
        // A large streamed body that would otherwise be well over min_size.
        let chunks: Vec<std::result::Result<Bytes, BoxError>> =
            (0..100).map(|_| Ok(Bytes::from("x".repeat(64)))).collect();
        ctx.stream(futures_util::stream::iter(chunks)).await
    });

    let req = HyperRequest::builder()
        .uri("/stream")
        .header("Accept-Encoding", "br, gzip")
        .body(Full::new(Bytes::new()))
        .unwrap();
    let resp = app.oneshot(req).await;
    assert!(
        resp.headers()
            .get(hyper::header::CONTENT_ENCODING)
            .is_none(),
        "streaming bodies must not be compressed"
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.len(), 100 * 64);
}

#[cfg(feature = "compression")]
#[tokio::test]
async fn compression_still_compresses_buffered_bodies() {
    use ultimo::middleware::builtin::compression;
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/buffered", |ctx: Context| async move {
        ctx.text("y".repeat(4096)).await
    });

    let req = HyperRequest::builder()
        .uri("/buffered")
        .header("Accept-Encoding", "gzip")
        .body(Full::new(Bytes::new()))
        .unwrap();
    let resp = app.oneshot(req).await;
    assert_eq!(
        resp.headers()
            .get(hyper::header::CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok()),
        Some("gzip"),
        "buffered bodies over min_size must still be compressed"
    );
}
