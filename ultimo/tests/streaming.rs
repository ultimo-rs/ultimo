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
