//! Lives in its own test binary on purpose: `tracing` caches callsite interest
//! process-wide, so a subscriber-less test running concurrently in the same
//! binary can disable the request span and make this assertion flaky.

use bytes::Bytes;
use http_body_util::Full;
use hyper::Request as HyperRequest;
use std::sync::{Arc, Mutex};
use ultimo::middleware::builtin::{logger, request_id};
use ultimo::prelude::*;

#[derive(Clone, Default)]
struct Buf(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Buf {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Buf {
    type Writer = Buf;
    fn make_writer(&'a self) -> Buf {
        self.clone()
    }
}

#[tokio::test]
async fn every_log_line_in_the_request_carries_the_id() {
    let buf = Buf::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buf.clone())
        .with_ansi(false)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(request_id()); // outermost, so the span wraps logger()
    app.use_middleware(logger());
    app.get("/", |ctx: Context| async move {
        tracing::info!("inside the handler");
        ctx.text("ok").await
    });

    let req = HyperRequest::builder()
        .uri("/")
        .header("x-request-id", "corr-42")
        .body(Full::new(Bytes::new()))
        .unwrap();
    app.oneshot(req).await;

    let out = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.len() >= 3, "expected logger + handler lines: {out}");
    for line in lines {
        assert!(line.contains("corr-42"), "line missing request id: {line}");
    }
}
