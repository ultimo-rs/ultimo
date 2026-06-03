//! Helpers for unit-testing middleware in isolation.

use crate::context::Context;
use crate::error::Result;
use crate::middleware::{BoxedMiddleware, MiddlewareChain};
use crate::response::Response;
use bytes::Bytes;
use hyper::Request as HyperRequest;
use std::future::Future;

/// Build a [`Context`] for tests without a live request.
pub fn test_context() -> TestContextBuilder {
    TestContextBuilder {
        method: "GET".to_string(),
        path: "/".to_string(),
        headers: Vec::new(),
        body: Bytes::new(),
    }
}

/// Builder produced by [`test_context`].
pub struct TestContextBuilder {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Bytes,
}

impl TestContextBuilder {
    /// Set the HTTP method (default `GET`).
    pub fn method(mut self, m: &str) -> Self {
        self.method = m.to_string();
        self
    }

    /// Set the request path (default `/`).
    pub fn path(mut self, p: &str) -> Self {
        self.path = p.to_string();
        self
    }

    /// Add a request header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    /// Set the request body.
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    /// Build the [`Context`].
    pub fn build(self) -> Context {
        let mut builder = HyperRequest::builder()
            .method(self.method.as_str())
            .uri(&self.path);
        for (n, v) in &self.headers {
            builder = builder.header(n, v);
        }
        let req = builder.body(()).expect("valid test request parts");
        let (parts, ()) = req.into_parts();
        Context::from_parts(parts, self.body, crate::router::Params::new())
    }
}

/// Run a single middleware against a context and a terminal handler.
///
/// Lets you assert a middleware's behavior (short-circuit, header injection,
/// state mutation) in isolation, using the real [`MiddlewareChain`] machinery.
/// Construct `middleware` the same way the built-ins do — `Arc::new(|ctx, next|
/// Box::pin(async move { … }))` — which yields a [`BoxedMiddleware`].
pub async fn run_middleware<F, Fut>(
    middleware: BoxedMiddleware,
    ctx: Context,
    handler: F,
) -> Result<Response>
where
    F: FnOnce(Context) -> Fut + Send + 'static,
    Fut: Future<Output = Result<Response>> + Send + 'static,
{
    let mut chain = MiddlewareChain::new();
    chain.push(middleware);
    chain.execute(ctx, handler).await
}
