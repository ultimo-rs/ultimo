//! Prometheus metrics for HTTP traffic.
//!
//! Requires the `metrics` Cargo feature. No extra dependencies: the
//! [text exposition format](https://prometheus.io/docs/instrumenting/exposition_formats/)
//! is written directly.
//!
//! Three metrics are recorded:
//!
//! | Metric | Type | Labels |
//! |---|---|---|
//! | `http_requests_total` | counter | `method`, `route`, `status` |
//! | `http_request_duration_seconds` | histogram | `method`, `route` |
//! | `http_requests_in_flight` | gauge | — |
//!
//! `route` is the **registered pattern** (`/users/:id`), never the raw path, so
//! label cardinality stays bounded by the number of routes you define. Requests
//! that match no route never reach middleware, so 404s for unknown paths are
//! not recorded.
//!
//! ```rust
//! use ultimo::prelude::*;
//! use ultimo::metrics::Metrics;
//!
//! let metrics = Metrics::new();
//! let mut app = Ultimo::new();
//! app.use_middleware(metrics.middleware());
//! app.get("/metrics", metrics.handler());
//! ```
//!
//! The `/metrics` route is public as written; put it behind
//! [`BasicAuth`](crate::middleware::builtin::BasicAuth), an IP allow-list, or a
//! separate internal listener if your scrape target shouldn't be reachable from
//! the internet.

use crate::context::Context;
use crate::error::Result;
use crate::middleware::BoxedMiddleware;
use crate::response::{Response, ResponseBuilder};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::future::Future;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Content type of the Prometheus text exposition format.
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Prometheus' default histogram buckets, in seconds.
pub const DEFAULT_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Label used for `route` when no route matched (e.g. `OPTIONS` preflights
/// answered by middleware).
const UNMATCHED_ROUTE: &str = "unmatched";

#[derive(Default)]
struct Histogram {
    /// Non-cumulative per-bucket counts; cumulated when rendered.
    buckets: Vec<u64>,
    sum: f64,
    count: u64,
}

#[derive(Default)]
struct Series {
    /// (method, route, status) → count
    requests: BTreeMap<(String, Arc<str>, u16), u64>,
    /// (method, route) → latency histogram
    durations: BTreeMap<(String, Arc<str>), Histogram>,
}

struct Inner {
    buckets: Vec<f64>,
    series: Mutex<Series>,
    in_flight: AtomicI64,
}

/// HTTP metrics registry. Cheap to clone; all clones share the same data.
///
/// See the [module docs](self) for the metrics recorded and an example.
#[derive(Clone)]
pub struct Metrics {
    inner: Arc<Inner>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    /// A registry using Prometheus' [default buckets](DEFAULT_BUCKETS).
    pub fn new() -> Self {
        Self::with_buckets(DEFAULT_BUCKETS)
    }

    /// A registry with custom latency histogram bucket upper bounds, in
    /// seconds. Bounds are sorted and deduplicated; `+Inf` is always added.
    ///
    /// ```rust
    /// let metrics = ultimo::metrics::Metrics::with_buckets(&[0.05, 0.25, 1.0]);
    /// ```
    pub fn with_buckets(buckets: &[f64]) -> Self {
        let mut buckets: Vec<f64> = buckets.iter().copied().filter(|b| b.is_finite()).collect();
        buckets.sort_by(f64::total_cmp);
        buckets.dedup();
        Self {
            inner: Arc::new(Inner {
                buckets,
                series: Mutex::new(Series::default()),
                in_flight: AtomicI64::new(0),
            }),
        }
    }

    /// Middleware that records every request it sees. Register it first
    /// (outermost) so the latency covers the rest of the chain.
    pub fn middleware(&self) -> BoxedMiddleware {
        let metrics = self.clone();
        Arc::new(move |ctx, next| {
            let metrics = metrics.clone();
            Box::pin(async move {
                let method = ctx.req.method().as_str().to_string();
                let route: Arc<str> = Arc::from(ctx.req.route_pattern().unwrap_or(UNMATCHED_ROUTE));
                // Decrements on drop, so a request cancelled mid-flight (e.g. by
                // `request_timeout`) doesn't leave the gauge stuck high.
                let _guard = InFlight::enter(&metrics.inner.in_flight);
                let start = Instant::now();

                let result = next(ctx).await;

                let status = match &result {
                    Ok(res) => res.status().as_u16(),
                    Err(err) => err.status_code(),
                };
                metrics.observe(method, route, status, start.elapsed().as_secs_f64());
                result
            })
        })
    }

    /// A route handler serving [`render`](Self::render) with the Prometheus
    /// content type. Mount it with `app.get("/metrics", metrics.handler())`.
    pub fn handler(
        &self,
    ) -> impl Fn(Context) -> std::pin::Pin<Box<dyn Future<Output = Result<Response>> + Send>>
           + Clone
           + Send
           + Sync
           + 'static {
        let metrics = self.clone();
        move |_ctx: Context| {
            let body = metrics.render();
            Box::pin(async move {
                ResponseBuilder::new()
                    .header("Content-Type", CONTENT_TYPE)
                    .body(body.into_bytes())
                    .build()
            })
        }
    }

    /// Render all metrics in the Prometheus text exposition format.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let series = self.inner.series.lock().unwrap_or_else(|e| e.into_inner());

        out.push_str("# HELP http_requests_total Total number of HTTP requests.\n");
        out.push_str("# TYPE http_requests_total counter\n");
        for ((method, route, status), count) in &series.requests {
            let _ = writeln!(
                out,
                "http_requests_total{{method=\"{}\",route=\"{}\",status=\"{}\"}} {}",
                escape(method),
                escape(route),
                status,
                count
            );
        }

        out.push_str("# HELP http_request_duration_seconds HTTP request latency in seconds.\n");
        out.push_str("# TYPE http_request_duration_seconds histogram\n");
        for ((method, route), hist) in &series.durations {
            let labels = format!("method=\"{}\",route=\"{}\"", escape(method), escape(route));
            let mut cumulative = 0;
            for (bound, n) in self.inner.buckets.iter().zip(&hist.buckets) {
                cumulative += n;
                let _ = writeln!(
                    out,
                    "http_request_duration_seconds_bucket{{{labels},le=\"{bound}\"}} {cumulative}"
                );
            }
            let _ = writeln!(
                out,
                "http_request_duration_seconds_bucket{{{labels},le=\"+Inf\"}} {}",
                hist.count
            );
            let _ = writeln!(
                out,
                "http_request_duration_seconds_sum{{{labels}}} {}",
                hist.sum
            );
            let _ = writeln!(
                out,
                "http_request_duration_seconds_count{{{labels}}} {}",
                hist.count
            );
        }
        drop(series);

        out.push_str("# HELP http_requests_in_flight HTTP requests currently being served.\n");
        out.push_str("# TYPE http_requests_in_flight gauge\n");
        let _ = writeln!(
            out,
            "http_requests_in_flight {}",
            self.inner.in_flight.load(Ordering::Relaxed)
        );
        out
    }

    fn observe(&self, method: String, route: Arc<str>, status: u16, seconds: f64) {
        let bucket = self.inner.buckets.iter().position(|b| seconds <= *b);
        let mut series = self.inner.series.lock().unwrap_or_else(|e| e.into_inner());
        *series
            .requests
            .entry((method.clone(), route.clone(), status))
            .or_insert(0) += 1;
        let hist = series.durations.entry((method, route)).or_default();
        if hist.buckets.is_empty() {
            hist.buckets = vec![0; self.inner.buckets.len()];
        }
        if let Some(i) = bucket {
            hist.buckets[i] += 1;
        }
        hist.sum += seconds;
        hist.count += 1;
    }
}

struct InFlight<'a>(&'a AtomicI64);

impl<'a> InFlight<'a> {
    fn enter(gauge: &'a AtomicI64) -> Self {
        gauge.fetch_add(1, Ordering::Relaxed);
        Self(gauge)
    }
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Escape a label value per the exposition format (`\`, `"`, newline).
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_label_values() {
        assert_eq!(escape(r#"a"b\c"#), r#"a\"b\\c"#);
        assert_eq!(escape("a\nb"), r"a\nb");
    }

    #[test]
    fn buckets_are_sorted_deduped_and_finite() {
        let m = Metrics::with_buckets(&[1.0, 0.1, 1.0, f64::INFINITY, f64::NAN]);
        assert_eq!(m.inner.buckets, vec![0.1, 1.0]);
    }

    #[test]
    fn observation_lands_in_first_fitting_bucket_and_renders_cumulatively() {
        let m = Metrics::with_buckets(&[0.1, 1.0]);
        m.observe("GET".into(), Arc::from("/a"), 200, 0.05);
        m.observe("GET".into(), Arc::from("/a"), 200, 0.5);
        m.observe("GET".into(), Arc::from("/a"), 200, 5.0);
        let body = m.render();
        assert!(
            body.contains(r#"_bucket{method="GET",route="/a",le="0.1"} 1"#),
            "{body}"
        );
        assert!(
            body.contains(r#"_bucket{method="GET",route="/a",le="1"} 2"#),
            "{body}"
        );
        assert!(
            body.contains(r#"_bucket{method="GET",route="/a",le="+Inf"} 3"#),
            "{body}"
        );
        assert!(
            body.contains(r#"_count{method="GET",route="/a"} 3"#),
            "{body}"
        );
    }

    #[test]
    fn in_flight_guard_decrements_on_drop() {
        let gauge = AtomicI64::new(0);
        {
            let _g = InFlight::enter(&gauge);
            assert_eq!(gauge.load(Ordering::Relaxed), 1);
        }
        assert_eq!(gauge.load(Ordering::Relaxed), 0);
    }
}
