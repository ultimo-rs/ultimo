//! Integration tests for response compression middleware.
//! Run with: cargo test -p ultimo --features "compression" --test compression

#![cfg(feature = "compression")]

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request as HyperRequest;
use ultimo::middleware::builtin::{compression, Compression};
use ultimo::prelude::*;

fn empty() -> Full<Bytes> {
    Full::new(Bytes::new())
}

/// A body large enough to exceed the default 1024-byte min_size floor.
fn large_text() -> String {
    "Hello, compressed world! ".repeat(100)
}

fn app_with_text_route() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/text", |ctx: Context| async move {
        ctx.json(serde_json::json!({ "data": large_text() })).await
    });
    app
}

#[tokio::test]
async fn gzip_accept_returns_gzip_encoding() {
    let app = app_with_text_route();
    let req = HyperRequest::builder()
        .uri("/text")
        .header("accept-encoding", "gzip")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get("content-encoding")
            .and_then(|v| v.to_str().ok()),
        Some("gzip")
    );
}

#[tokio::test]
async fn brotli_accept_returns_brotli_encoding() {
    let app = app_with_text_route();
    let req = HyperRequest::builder()
        .uri("/text")
        .header("accept-encoding", "br")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get("content-encoding")
            .and_then(|v| v.to_str().ok()),
        Some("br")
    );
}

#[tokio::test]
async fn brotli_preferred_over_gzip_when_both_listed() {
    let app = app_with_text_route();
    let req = HyperRequest::builder()
        .uri("/text")
        .header("accept-encoding", "gzip, br")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(
        res.headers()
            .get("content-encoding")
            .and_then(|v| v.to_str().ok()),
        Some("br")
    );
}

#[tokio::test]
async fn no_accept_encoding_no_compression() {
    let app = app_with_text_route();
    let req = HyperRequest::builder().uri("/text").body(empty()).unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    assert!(res.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn image_content_type_not_compressed() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/img", |_ctx: Context| async move {
        Ok(hyper::Response::builder()
            .status(200)
            .header("content-type", "image/png")
            .body(Full::new(Bytes::from(vec![0u8; 2048])))
            .unwrap())
    });

    let req = HyperRequest::builder()
        .uri("/img")
        .header("accept-encoding", "gzip")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    assert!(res.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn small_body_not_compressed() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/tiny", |ctx: Context| async move {
        ctx.json(serde_json::json!({ "ok": true })).await // < 1024 bytes
    });

    let req = HyperRequest::builder()
        .uri("/tiny")
        .header("accept-encoding", "gzip")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert!(res.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn vary_header_always_set() {
    let app = app_with_text_route();
    let req = HyperRequest::builder()
        .uri("/text")
        .body(empty()) // no Accept-Encoding
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(
        res.headers().get("vary").and_then(|v| v.to_str().ok()),
        Some("Accept-Encoding")
    );
}

#[tokio::test]
async fn already_encoded_response_not_double_compressed() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/pre-encoded", |_ctx: Context| async move {
        Ok(hyper::Response::builder()
            .status(200)
            .header("content-type", "text/plain")
            .header("content-encoding", "gzip") // already encoded
            .body(Full::new(Bytes::from(vec![0u8; 2048])))
            .unwrap())
    });

    let req = HyperRequest::builder()
        .uri("/pre-encoded")
        .header("accept-encoding", "gzip")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    // Must not double-compress — content-encoding stays as the original "gzip"
    assert_eq!(
        res.headers()
            .get("content-encoding")
            .and_then(|v| v.to_str().ok()),
        Some("gzip")
    );
}

#[tokio::test]
async fn gzip_body_is_decodable() {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let app = app_with_text_route();
    let req = HyperRequest::builder()
        .uri("/text")
        .header("accept-encoding", "gzip")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);

    let compressed = res.into_body().collect().await.unwrap().to_bytes();
    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut decoded = String::new();
    decoder
        .read_to_string(&mut decoded)
        .expect("gzip decode failed");
    let json: serde_json::Value = serde_json::from_str(&decoded).unwrap();
    assert!(json["data"].as_str().unwrap().starts_with("Hello"));
}

#[tokio::test]
async fn compression_builder_respects_min_size_override() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(Compression::new().gzip().min_size(99999).build());
    app.get("/big", |ctx: Context| async move {
        // Even a "large" body is below min_size=99999
        ctx.json(serde_json::json!({ "data": large_text() })).await
    });

    let req = HyperRequest::builder()
        .uri("/big")
        .header("accept-encoding", "gzip")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    // min_size=99999 means nothing is compressed
    assert!(res.headers().get("content-encoding").is_none());
}
