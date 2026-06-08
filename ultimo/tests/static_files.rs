//! Integration tests for static file serving.
//! Run with: cargo test -p ultimo --features "static-files" --test static_files

#![cfg(feature = "static-files")]

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request as HyperRequest;
use tempfile::TempDir;
use ultimo::prelude::*;

fn empty() -> Full<Bytes> {
    Full::new(Bytes::new())
}

/// Write a file (possibly in a subdirectory) into `dir`.
async fn write_fixture(dir: &TempDir, name: &str, content: &[u8]) {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.unwrap();
    }
    tokio::fs::write(&path, content).await.unwrap();
}

#[tokio::test]
async fn existing_file_returns_200_with_correct_mime() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "hello.txt", b"hello world").await;

    let mut app = Ultimo::new_without_defaults();
    app.serve_static("/assets", dir.path());

    let req = HyperRequest::builder()
        .uri("/assets/hello.txt")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;

    assert_eq!(res.status(), 200);
    let ct = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    // mime_guess may return "text/plain" or "text/plain; charset=utf-8"
    assert!(ct.starts_with("text/plain"), "content-type was: {ct}");

    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), b"hello world");
}

#[tokio::test]
async fn missing_file_returns_404() {
    let dir = TempDir::new().unwrap();

    let mut app = Ultimo::new_without_defaults();
    app.serve_static("/assets", dir.path());

    let req = HyperRequest::builder()
        .uri("/assets/nope.txt")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn path_traversal_is_blocked() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "secret.txt", b"secret").await;

    let mut app = Ultimo::new_without_defaults();
    app.serve_static("/assets", dir.path());

    // URL-encoded `..` — hyper doesn't decode this for us, but the path
    // resolution in serve_file will normalize it.
    let req = HyperRequest::builder()
        .uri("/assets/%2E%2E%2Fsecret.txt")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn etag_is_set_and_304_on_match() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "file.txt", b"content").await;

    let mut app = Ultimo::new_without_defaults();
    app.serve_static("/assets", dir.path());

    // First request — grab ETag
    let req = HyperRequest::builder()
        .uri("/assets/file.txt")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    let etag = res
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .expect("ETag header missing")
        .to_string();

    // Second request with If-None-Match → 304
    let mut app2 = Ultimo::new_without_defaults();
    app2.serve_static("/assets", dir.path());

    let req2 = HyperRequest::builder()
        .uri("/assets/file.txt")
        .header("if-none-match", &etag)
        .body(empty())
        .unwrap();
    let res2 = app2.oneshot(req2).await;
    assert_eq!(res2.status(), 304);
}

#[tokio::test]
async fn nested_path_is_served() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "css/main.css", b"body { color: red; }").await;

    let mut app = Ultimo::new_without_defaults();
    app.serve_static("/assets", dir.path());

    let req = HyperRequest::builder()
        .uri("/assets/css/main.css")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    let ct = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.starts_with("text/css"), "content-type was: {ct}");
}

#[tokio::test]
async fn spa_fallback_serves_index_for_unknown_routes() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "index.html", b"<!DOCTYPE html><html></html>").await;

    let mut app = Ultimo::new_without_defaults();
    app.get("/api/hello", |ctx: Context| async move {
        ctx.json(serde_json::json!({ "ok": true })).await
    });
    app.serve_spa(dir.path(), "index.html");

    let req = HyperRequest::builder()
        .uri("/unknown-route")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(body.starts_with(b"<!DOCTYPE html>"));
}

#[tokio::test]
async fn spa_fallback_does_not_intercept_post_404() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "index.html", b"<!DOCTYPE html>").await;

    let mut app = Ultimo::new_without_defaults();
    app.serve_spa(dir.path(), "index.html");

    let req = HyperRequest::builder()
        .method("POST")
        .uri("/unknown")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 404);
}
