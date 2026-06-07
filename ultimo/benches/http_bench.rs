//! Framework-overhead benchmarks — measure what Ultimo itself costs, isolated
//! from network/OS noise by driving the app in-process via `Ultimo::oneshot`.
//!
//! These are deliberately *relative* measurements (routing vs dispatch vs JSON
//! vs middleware overhead), low-variance enough to base a regression gate on.
//! Absolute numbers are only comparable within the same machine/run. Real req/s
//! comparisons live in the end-to-end load tests — see `BENCHMARKS.md`.
//!
//! Run with: `cargo bench -p ultimo --bench http_bench`

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use http_body_util::Full;
use hyper::Request as HyperRequest;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use ultimo::middleware::{BoxedMiddleware, Next};
use ultimo::response::Response;
use ultimo::{Context, Result, Ultimo};

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn request(uri: &str) -> HyperRequest<Full<Bytes>> {
    HyperRequest::builder()
        .uri(uri)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// A pass-through middleware that does nothing but call the next handler — used
/// to measure per-layer chain overhead.
fn passthrough() -> BoxedMiddleware {
    Arc::new(|ctx: Context, next: Next| {
        Box::pin(async move { next(ctx).await })
            as Pin<Box<dyn Future<Output = Result<Response>> + Send>>
    })
}

/// Full request → minimal text handler.
fn bench_dispatch_text(c: &mut Criterion) {
    let rt = runtime();
    let mut app = Ultimo::new_without_defaults();
    app.get("/", |ctx: Context| async move { ctx.text("ok").await });

    c.bench_function("dispatch_text", |b| {
        b.to_async(&rt).iter(|| async {
            let res = app.oneshot(black_box(request("/"))).await;
            black_box(res.status());
        });
    });
}

/// Full request → JSON-serialized response.
fn bench_dispatch_json(c: &mut Criterion) {
    let rt = runtime();
    let mut app = Ultimo::new_without_defaults();
    app.get("/", |ctx: Context| async move {
        ctx.json(serde_json::json!({ "message": "hello", "ok": true, "n": 42 }))
            .await
    });

    c.bench_function("dispatch_json", |b| {
        b.to_async(&rt).iter(|| async {
            let res = app.oneshot(black_box(request("/"))).await;
            black_box(res.status());
        });
    });
}

/// Radix-tree route lookup as the routing table grows. Dispatches to a route in
/// the middle of the table plus a parameterized route.
fn bench_routing(c: &mut Criterion) {
    let rt = runtime();
    let mut group = c.benchmark_group("routing");

    for n in [10usize, 100, 500] {
        let mut app = Ultimo::new_without_defaults();
        for i in 0..n {
            let path = format!("/route/{i}");
            app.get(&path, |ctx: Context| async move { ctx.text("ok").await });
        }
        app.get("/users/:id", |ctx: Context| async move {
            let id = ctx.req.param("id")?;
            ctx.text(id.to_string()).await
        });

        // Look up a static route in the middle of the table.
        let mid = format!("/route/{}", n / 2);
        group.bench_with_input(BenchmarkId::new("static", n), &mid, |b, mid| {
            b.to_async(&rt).iter(|| async {
                let res = app.oneshot(black_box(request(mid))).await;
                black_box(res.status());
            });
        });

        // Look up the parameterized route.
        group.bench_with_input(BenchmarkId::new("param", n), &n, |b, _| {
            b.to_async(&rt).iter(|| async {
                let res = app.oneshot(black_box(request("/users/123"))).await;
                black_box(res.status());
            });
        });
    }

    group.finish();
}

/// Per-layer middleware overhead: dispatch through a chain of pass-through layers.
fn bench_middleware_chain(c: &mut Criterion) {
    let rt = runtime();
    let mut group = c.benchmark_group("middleware_chain");

    for layers in [0usize, 1, 5, 10] {
        let mut app = Ultimo::new_without_defaults();
        for _ in 0..layers {
            app.use_middleware(passthrough());
        }
        app.get("/", |ctx: Context| async move { ctx.text("ok").await });

        group.bench_with_input(BenchmarkId::from_parameter(layers), &layers, |b, _| {
            b.to_async(&rt).iter(|| async {
                let res = app.oneshot(black_box(request("/"))).await;
                black_box(res.status());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_dispatch_text,
    bench_dispatch_json,
    bench_routing,
    bench_middleware_chain
);
criterion_main!(benches);
