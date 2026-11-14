//! OpenTelemetry tracing for HTTP traffic.
//!
//! Requires the `otel` Cargo feature. Depends only on the
//! [`opentelemetry`](https://docs.rs/opentelemetry) **API** crate: your app
//! chooses the SDK, sampler and exporter (OTLP, Jaeger, stdout, …) and installs
//! them globally. Until it does, every span is a no-op.
//!
//! [`Tracing::middleware`] starts one `SERVER` span per request, following the
//! [HTTP semantic conventions](https://opentelemetry.io/docs/specs/semconv/http/http-spans/):
//!
//! | | |
//! |---|---|
//! | Span name | `{method} {route}`, e.g. `GET /users/:id` (just `{method}` when no route matched) |
//! | `http.request.method` | `GET`, `POST`, … (`_OTHER` for non-standard methods, with `http.request.method_original`) |
//! | `http.route` | the **registered pattern**, never the raw path |
//! | `url.path` | the raw request path |
//! | `user_agent.original` | the `User-Agent` header, if sent |
//! | `http.response.status_code` | the response status |
//! | `error.type` | the status code for 5xx responses, or `cancelled` if the request was dropped (e.g. by a request timeout) |
//! | `ultimo.request_id` | the id assigned by the [`request_id`](crate::middleware::builtin::request_id) middleware, if registered |
//!
//! 5xx responses also set the span status to `Error`. 4xx responses don't: on
//! a server span they're the client's fault, not the server's.
//!
//! **Incoming trace context** (e.g. a W3C `traceparent` header) is extracted
//! with the global text-map propagator, so the server span continues the
//! caller's trace. The default global propagator is a no-op — install
//! `TraceContextPropagator` from `opentelemetry_sdk` to honour `traceparent`.
//!
//! While the rest of the middleware chain and the handler run, the server span
//! is the **current** OpenTelemetry context, so spans you start inside a
//! handler (or via `tracing-opentelemetry`) become its children. Work you
//! `tokio::spawn` doesn't inherit it automatically — pass
//! `opentelemetry::Context::current()` along and wrap the task with
//! `FutureExt::with_context`.
//!
//! ```rust
//! use ultimo::prelude::*;
//! use ultimo::otel::Tracing;
//!
//! // At startup, install your SDK first, e.g.:
//! //   global::set_tracer_provider(provider);
//! //   global::set_text_map_propagator(TraceContextPropagator::new());
//! let mut app = Ultimo::new();
//! app.use_middleware(Tracing::new().middleware());
//! ```
//!
//! Use the same major/minor version of `opentelemetry` as Ultimo (0.33):
//! OpenTelemetry's globals are per crate version, so a provider installed via a
//! different version isn't seen here and spans stay no-ops.

use crate::middleware::BoxedMiddleware;
use opentelemetry::context::FutureExt as _;
use opentelemetry::propagation::Extractor;
use opentelemetry::trace::{SpanKind, Status, TraceContextExt, Tracer};
use opentelemetry::{global, InstrumentationScope, KeyValue};
use std::borrow::Cow;
use std::sync::Arc;

/// Middleware factory for OpenTelemetry server spans.
///
/// See the [module docs](self) for the span shape, propagation and an example.
#[derive(Clone, Debug)]
pub struct Tracing {
    scope: InstrumentationScope,
}

impl Default for Tracing {
    fn default() -> Self {
        Self::new()
    }
}

impl Tracing {
    /// Spans are reported under the instrumentation scope `ultimo`, versioned
    /// with the crate version.
    pub fn new() -> Self {
        Self::with_scope_name("ultimo")
    }

    /// Report spans under a custom instrumentation scope name (the
    /// "tracer name" shown by most backends).
    ///
    /// ```rust
    /// let tracing = ultimo::otel::Tracing::with_scope_name("my-api");
    /// ```
    pub fn with_scope_name(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            scope: InstrumentationScope::builder(name)
                .with_version(env!("CARGO_PKG_VERSION"))
                .build(),
        }
    }

    /// Middleware that wraps every request in a server span. Register it first
    /// (outermost) so the span covers the rest of the chain.
    pub fn middleware(&self) -> BoxedMiddleware {
        let scope = self.scope.clone();
        Arc::new(move |ctx, next| {
            let scope = scope.clone();
            Box::pin(async move {
                let parent = global::get_text_map_propagator(|p| {
                    p.extract(&HeaderExtractor(ctx.req.headers_map()))
                });

                let (method, original) = normalize_method(ctx.req.method().as_str());
                let route = ctx.req.route_pattern().map(str::to_string);
                let name = match &route {
                    Some(route) => format!("{method} {route}"),
                    None => method.to_string(),
                };

                let mut attributes = vec![
                    KeyValue::new("http.request.method", method),
                    KeyValue::new("url.path", ctx.req.path().to_string()),
                ];
                if let Some(original) = original {
                    attributes.push(KeyValue::new("http.request.method_original", original));
                }
                if let Some(route) = route {
                    attributes.push(KeyValue::new("http.route", route));
                }
                if let Some(ua) = ctx.req.header("user-agent") {
                    attributes.push(KeyValue::new("user_agent.original", ua));
                }

                // Fetched per request so a provider installed after the
                // middleware was built is still picked up.
                let tracer = global::tracer_with_scope(scope);
                let span = tracer
                    .span_builder(name)
                    .with_kind(SpanKind::Server)
                    .with_attributes(attributes)
                    .start_with_context(&tracer, &parent);
                let cx = parent.with_span(span);
                // Ends the span on every path, including the future being
                // dropped mid-flight (request timeout, client disconnect).
                let mut guard = SpanGuard {
                    cx: cx.clone(),
                    finished: false,
                };

                let state = ctx.clone();
                let result = next(ctx).with_context(cx).await;

                let status = match &result {
                    Ok(res) => res.status().as_u16(),
                    Err(err) => err.status_code(),
                };
                let span = guard.cx.span();
                span.set_attribute(KeyValue::new(
                    "http.response.status_code",
                    i64::from(status),
                ));
                if status >= 500 {
                    span.set_attribute(KeyValue::new("error.type", status.to_string()));
                    let description = match &result {
                        Err(err) => err.to_string(),
                        Ok(_) => String::new(),
                    };
                    span.set_status(Status::error(description));
                }
                if let Some(id) = state.request_id().await {
                    span.set_attribute(KeyValue::new("ultimo.request_id", id));
                }
                guard.finished = true;
                result
            })
        })
    }
}

struct SpanGuard {
    cx: opentelemetry::Context,
    finished: bool,
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        let span = self.cx.span();
        if !self.finished {
            span.set_attribute(KeyValue::new("error.type", "cancelled"));
        }
        span.end();
    }
}

/// `(method, original)`: non-standard methods are reported as `_OTHER`, with
/// the original kept separately, to bound cardinality.
fn normalize_method(method: &str) -> (&'static str, Option<String>) {
    const KNOWN: &[&str] = &[
        "GET", "HEAD", "POST", "PUT", "DELETE", "CONNECT", "OPTIONS", "TRACE", "PATCH",
    ];
    match KNOWN.iter().find(|m| **m == method) {
        Some(m) => (m, None),
        None => ("_OTHER", Some(method.to_string())),
    }
}

struct HeaderExtractor<'a>(&'a hyper::HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }

    fn get_all(&self, key: &str) -> Option<Vec<&str>> {
        let values: Vec<&str> = self
            .0
            .get_all(key)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect();
        (!values.is_empty()).then_some(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_methods_pass_through() {
        assert_eq!(normalize_method("GET"), ("GET", None));
        assert_eq!(normalize_method("PATCH"), ("PATCH", None));
    }

    #[test]
    fn unknown_methods_become_other() {
        assert_eq!(
            normalize_method("PURGE"),
            ("_OTHER", Some("PURGE".to_string()))
        );
    }

    #[test]
    fn extractor_reads_all_values_of_a_header() {
        let mut headers = hyper::HeaderMap::new();
        headers.append("tracestate", "a=1".parse().unwrap());
        headers.append("tracestate", "b=2".parse().unwrap());
        let ex = HeaderExtractor(&headers);
        assert_eq!(ex.get_all("tracestate"), Some(vec!["a=1", "b=2"]));
        assert_eq!(ex.get_all("missing"), None);
        assert_eq!(ex.keys(), vec!["tracestate"]);
    }
}
