//! Middleware system with composable chain execution
//!
//! Middleware can execute before and after handlers, modify context,
//! and short-circuit request handling.

use crate::{context::Context, error::Result, response::Response};
use http_body_util::Full;
use hyper::{body::Bytes, Response as HyperResponse};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for the next() function in middleware
pub type Next<'a> = Box<
    dyn FnOnce(Context) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>> + Send + 'a,
>;

/// Type alias for boxed middleware functions
pub type BoxedMiddleware = Arc<
    dyn for<'a> Fn(Context, Next<'a>) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>>
        + Send
        + Sync,
>;

/// Trait for types that can be converted into middleware
pub trait IntoMiddleware {
    fn into_middleware(self) -> BoxedMiddleware;
}

/// Implement IntoMiddleware for async functions
impl<F> IntoMiddleware for F
where
    F: for<'a> Fn(Context, Next<'a>) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>>
        + Send
        + Sync
        + 'static,
{
    fn into_middleware(self) -> BoxedMiddleware {
        Arc::new(self)
    }
}

/// Middleware chain executor
pub struct MiddlewareChain {
    middleware: Vec<BoxedMiddleware>,
}

impl MiddlewareChain {
    /// Create a new empty middleware chain
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }

    /// Add middleware to the chain
    pub fn push(&mut self, middleware: BoxedMiddleware) {
        self.middleware.push(middleware);
    }

    /// Execute the middleware chain with a final handler
    pub async fn execute<F, Fut>(self, ctx: Context, handler: F) -> Result<Response>
    where
        F: FnOnce(Context) -> Fut + Send + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        self.execute_at(ctx, 0, Box::new(handler)).await
    }

    /// Execute starting at a specific middleware index
    fn execute_at<F, Fut>(
        self,
        ctx: Context,
        index: usize,
        handler: Box<F>,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>>
    where
        F: FnOnce(Context) -> Fut + Send + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        Box::pin(async move {
            if index >= self.middleware.len() {
                // No more middleware, call final handler
                return handler(ctx).await;
            }

            let current_middleware = self.middleware[index].clone();
            let next_index = index + 1;

            // Create next() closure that captures remaining chain
            let next: Next = Box::new(move |ctx| self.execute_at(ctx, next_index, handler));

            current_middleware(ctx, next).await
        })
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in middleware constructors
pub mod builtin {
    use super::*;
    use std::time::Instant;
    use tracing::{error, info};

    /// Logger middleware that logs request/response details
    pub fn logger() -> BoxedMiddleware {
        Arc::new(|ctx, next| {
            Box::pin(async move {
                let method = ctx.req.method().clone();
                let path = ctx.req.path().to_string();
                let start = Instant::now();

                info!("--> {} {}", method, path);

                let result = next(ctx).await;

                let duration = start.elapsed();
                match &result {
                    Ok(response) => {
                        info!(
                            "<-- {} {} {} ({:?})",
                            method,
                            path,
                            response.status().as_u16(),
                            duration
                        );
                    }
                    Err(err) => {
                        error!("<-- {} {} ERROR: {} ({:?})", method, path, err, duration);
                    }
                }

                result
            })
        })
    }

    /// CORS middleware with configurable options
    pub struct Cors {
        allow_origin: String,
        allow_methods: Vec<String>,
        allow_headers: Vec<String>,
    }

    impl Cors {
        pub fn new() -> Self {
            Self {
                allow_origin: "*".to_string(),
                allow_methods: vec!["GET".to_string(), "POST".to_string()],
                allow_headers: vec!["Content-Type".to_string()],
            }
        }

        pub fn allow_origin(mut self, origin: impl Into<String>) -> Self {
            self.allow_origin = origin.into();
            self
        }

        pub fn allow_methods(mut self, methods: Vec<impl Into<String>>) -> Self {
            self.allow_methods = methods.into_iter().map(|m| m.into()).collect();
            self
        }

        pub fn allow_headers(mut self, headers: Vec<impl Into<String>>) -> Self {
            self.allow_headers = headers.into_iter().map(|h| h.into()).collect();
            self
        }

        pub fn build(self) -> BoxedMiddleware {
            let origin = self.allow_origin;
            let methods = self.allow_methods.join(", ");
            let headers = self.allow_headers.join(", ");

            Arc::new(move |ctx, next| {
                let origin = origin.clone();
                let methods = methods.clone();
                let headers = headers.clone();

                Box::pin(async move {
                    // Handle preflight OPTIONS requests
                    if ctx.req.method() == "OPTIONS" {
                        let response = HyperResponse::builder()
                            .status(204)
                            .header("Access-Control-Allow-Origin", origin)
                            .header("Access-Control-Allow-Methods", methods)
                            .header("Access-Control-Allow-Headers", headers)
                            .body(Full::new(Bytes::new()))
                            .unwrap();
                        return Ok(response);
                    }

                    // Set CORS headers on context before calling next
                    ctx.header("Access-Control-Allow-Origin", origin).await;
                    ctx.header("Access-Control-Allow-Methods", methods).await;
                    ctx.header("Access-Control-Allow-Headers", headers).await;

                    // Call next with the modified context
                    next(ctx).await
                })
            })
        }
    }

    impl Default for Cors {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Convenience function to create CORS middleware
    pub fn cors() -> BoxedMiddleware {
        Cors::new().build()
    }

    /// Powered-by header middleware that adds framework identification
    ///
    /// Adds `X-Powered-By: Ultimo` header to all responses.
    /// Similar to Express.js's X-Powered-By header.
    ///
    /// # Security Note
    /// Some security guides recommend disabling this header in production
    /// as it reveals your framework version to potential attackers.
    ///
    /// # Example
    /// ```rust,no_run
    /// use ultimo::prelude::*;
    ///
    /// let mut app = Ultimo::new();
    /// app.use_middleware(ultimo::middleware::builtin::powered_by());
    /// ```
    pub fn powered_by() -> BoxedMiddleware {
        Arc::new(|ctx, next| {
            Box::pin(async move {
                ctx.header("X-Powered-By", "Ultimo").await;
                next(ctx).await
            })
        })
    }

    /// Server identification middleware with configurable name
    ///
    /// Adds custom server identification headers to all responses.
    ///
    /// # Arguments
    /// * `name` - Server name (default: "Ultimo")
    /// * `version` - Include version in X-Powered-By header (default: false)
    ///
    /// # Example
    /// ```rust,no_run
    /// use ultimo::prelude::*;
    ///
    /// let mut app = Ultimo::new();
    /// // Add simple identification
    /// app.use_middleware(ultimo::middleware::builtin::server_headers("Ultimo", false));
    ///
    /// // Or with version
    /// app.use_middleware(ultimo::middleware::builtin::server_headers("Ultimo", true));
    /// ```
    pub fn server_headers(name: impl Into<String>, include_version: bool) -> BoxedMiddleware {
        let name = name.into();
        let version = if include_version {
            format!("{}/{}", name, env!("CARGO_PKG_VERSION"))
        } else {
            name.clone()
        };

        Arc::new(move |ctx, next| {
            let powered_by = version.clone();
            Box::pin(async move {
                ctx.header("X-Powered-By", powered_by).await;
                next(ctx).await
            })
        })
    }

    /// Secure-by-default HTTP security headers.
    ///
    /// Sets HSTS, X-Content-Type-Options, X-Frame-Options, Referrer-Policy and a
    /// restrictive Permissions-Policy. Content-Security-Policy is opt-in (a wrong
    /// CSP breaks more than it protects) — set it with [`SecurityHeaders::csp`].
    /// Headers are applied to the response **only if the handler didn't already
    /// set them**, so per-route overrides win.
    ///
    /// ```
    /// # use ultimo::Ultimo;
    /// let mut app = Ultimo::new_without_defaults();
    /// app.use_middleware(ultimo::middleware::builtin::security_headers());
    /// // or customized:
    /// app.use_middleware(
    ///     ultimo::middleware::builtin::SecurityHeaders::new()
    ///         .csp("default-src 'self'")
    ///         .frame_options("SAMEORIGIN")
    ///         .build(),
    /// );
    /// ```
    #[derive(Debug, Clone)]
    pub struct SecurityHeaders {
        hsts: Option<String>,
        csp: Option<String>,
        frame_options: Option<String>,
        content_type_options: bool,
        referrer_policy: Option<String>,
        permissions_policy: Option<String>,
    }

    impl Default for SecurityHeaders {
        fn default() -> Self {
            Self {
                hsts: Some("max-age=31536000; includeSubDomains".to_string()),
                csp: None,
                frame_options: Some("DENY".to_string()),
                content_type_options: true,
                referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
                permissions_policy: Some("geolocation=(), microphone=(), camera=()".to_string()),
            }
        }
    }

    impl SecurityHeaders {
        /// Secure defaults.
        pub fn new() -> Self {
            Self::default()
        }
        /// Set the `Strict-Transport-Security` value.
        pub fn hsts(mut self, value: impl Into<String>) -> Self {
            self.hsts = Some(value.into());
            self
        }
        /// Disable HSTS (e.g. for non-HTTPS environments).
        pub fn no_hsts(mut self) -> Self {
            self.hsts = None;
            self
        }
        /// Set the `Content-Security-Policy` (off by default).
        pub fn csp(mut self, value: impl Into<String>) -> Self {
            self.csp = Some(value.into());
            self
        }
        /// Set the `X-Frame-Options` value (default `DENY`).
        pub fn frame_options(mut self, value: impl Into<String>) -> Self {
            self.frame_options = Some(value.into());
            self
        }
        /// Set the `Referrer-Policy` value.
        pub fn referrer_policy(mut self, value: impl Into<String>) -> Self {
            self.referrer_policy = Some(value.into());
            self
        }
        /// Set the `Permissions-Policy` value.
        pub fn permissions_policy(mut self, value: impl Into<String>) -> Self {
            self.permissions_policy = Some(value.into());
            self
        }
        /// Disable the `X-Content-Type-Options: nosniff` header.
        pub fn no_content_type_options(mut self) -> Self {
            self.content_type_options = false;
            self
        }

        fn pairs(&self) -> Vec<(&'static str, String)> {
            let mut out = Vec::new();
            if let Some(v) = &self.hsts {
                out.push(("strict-transport-security", v.clone()));
            }
            if let Some(v) = &self.csp {
                out.push(("content-security-policy", v.clone()));
            }
            if let Some(v) = &self.frame_options {
                out.push(("x-frame-options", v.clone()));
            }
            if self.content_type_options {
                out.push(("x-content-type-options", "nosniff".to_string()));
            }
            if let Some(v) = &self.referrer_policy {
                out.push(("referrer-policy", v.clone()));
            }
            if let Some(v) = &self.permissions_policy {
                out.push(("permissions-policy", v.clone()));
            }
            out
        }

        /// Build the middleware.
        pub fn build(self) -> BoxedMiddleware {
            let pairs = Arc::new(self.pairs());
            Arc::new(move |ctx, next| {
                let pairs = pairs.clone();
                Box::pin(async move {
                    let mut response = next(ctx).await?;
                    let headers = response.headers_mut();
                    for (name, value) in pairs.iter() {
                        let header_name = hyper::header::HeaderName::from_static(name);
                        if !headers.contains_key(&header_name) {
                            if let Ok(hv) = hyper::header::HeaderValue::from_str(value) {
                                headers.insert(header_name, hv);
                            }
                        }
                    }
                    Ok(response)
                })
            })
        }
    }

    /// Security headers middleware with secure defaults.
    pub fn security_headers() -> BoxedMiddleware {
        SecurityHeaders::new().build()
    }

    // -------------------------------------------------------------------------
    // Response compression
    // -------------------------------------------------------------------------

    /// Response compression middleware (gzip + brotli).
    ///
    /// Negotiates the best encoding from the request's `Accept-Encoding` header.
    /// Brotli is preferred over gzip when both are accepted.
    ///
    /// Skips compression when:
    /// - The response body is smaller than `min_size` bytes (default: 1024).
    /// - The `Content-Type` is a binary format (images, audio, video, zip, …).
    /// - The response already carries a `Content-Encoding` header.
    ///
    /// Always sets `Vary: Accept-Encoding` (required by RFC 7231 so caches
    /// serve the correct version to each client).
    ///
    /// ```
    /// # use ultimo::Ultimo;
    /// let mut app = Ultimo::new_without_defaults();
    /// app.use_middleware(ultimo::middleware::builtin::compression());
    /// // or configured:
    /// app.use_middleware(
    ///     ultimo::middleware::builtin::Compression::new()
    ///         .gzip()
    ///         .brotli()
    ///         .min_size(512)
    ///         .build(),
    /// );
    /// ```
    #[cfg(feature = "compression")]
    #[derive(Debug, Clone)]
    pub struct Compression {
        gzip: bool,
        brotli: bool,
        min_size: usize,
    }

    #[cfg(feature = "compression")]
    impl Default for Compression {
        fn default() -> Self {
            Self {
                gzip: true,
                brotli: true,
                min_size: 1024,
            }
        }
    }

    #[cfg(feature = "compression")]
    impl Compression {
        /// Create with defaults (gzip + brotli enabled, min_size = 1024 bytes).
        pub fn new() -> Self {
            Self::default()
        }

        /// Enable gzip compression.
        pub fn gzip(mut self) -> Self {
            self.gzip = true;
            self
        }

        /// Enable brotli compression.
        pub fn brotli(mut self) -> Self {
            self.brotli = true;
            self
        }

        /// Minimum response body size in bytes before compression is applied.
        /// Responses smaller than this are passed through unchanged (default: 1024).
        pub fn min_size(mut self, bytes: usize) -> Self {
            self.min_size = bytes;
            self
        }

        /// Build the [`BoxedMiddleware`].
        pub fn build(self) -> BoxedMiddleware {
            use brotli::CompressorWriter;
            use flate2::{write::GzEncoder, Compression as GzLevel};
            use http_body_util::BodyExt;
            use hyper::header::{CONTENT_ENCODING, CONTENT_LENGTH, VARY};
            use std::io::Write;

            let gzip_enabled = self.gzip;
            let brotli_enabled = self.brotli;
            let min_size = self.min_size;

            Arc::new(move |ctx, next| {
                Box::pin(async move {
                    // Capture Accept-Encoding BEFORE consuming ctx with next().
                    let accept_enc = ctx
                        .req
                        .header("accept-encoding")
                        .unwrap_or_default()
                        .to_lowercase();

                    let mut res = next(ctx).await?;

                    // Always set Vary (RFC 7231 §7.1.4).
                    res.headers_mut().insert(
                        VARY,
                        hyper::header::HeaderValue::from_static("Accept-Encoding"),
                    );

                    // Skip if already encoded.
                    if res.headers().contains_key(CONTENT_ENCODING) {
                        return Ok(res);
                    }

                    // Decompose response so we can inspect and replace the body.
                    let (parts, body) = res.into_parts();
                    // Full<Bytes> is infallible — unwrap is safe.
                    let body_bytes = body.collect().await.unwrap().to_bytes();

                    // Skip below min_size.
                    if body_bytes.len() < min_size {
                        return Ok(hyper::Response::from_parts(parts, Full::new(body_bytes)));
                    }

                    // Skip binary content types.
                    let ct = parts
                        .headers
                        .get(hyper::header::CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_lowercase();

                    const SKIP_PREFIXES: &[&str] = &["image/", "audio/", "video/", "font/woff"];
                    const SKIP_EXACT: &[&str] = &[
                        "application/zip",
                        "application/gzip",
                        "application/x-gzip",
                        "application/octet-stream",
                    ];
                    let skip = SKIP_PREFIXES.iter().any(|p| ct.starts_with(p))
                        || SKIP_EXACT.iter().any(|e| ct.starts_with(e));

                    if skip {
                        return Ok(hyper::Response::from_parts(parts, Full::new(body_bytes)));
                    }

                    // Choose algorithm: prefer brotli > gzip > identity.
                    let use_brotli =
                        brotli_enabled && accept_enc.split(',').any(|t| t.trim() == "br");
                    let use_gzip = !use_brotli
                        && gzip_enabled
                        && accept_enc.split(',').any(|t| t.trim().starts_with("gzip"));

                    if use_brotli {
                        let mut compressed = Vec::new();
                        {
                            let mut writer = CompressorWriter::new(&mut compressed, 4096, 5, 22);
                            writer.write_all(&body_bytes).unwrap();
                        }
                        let len = compressed.len();
                        let mut res =
                            hyper::Response::from_parts(parts, Full::new(Bytes::from(compressed)));
                        res.headers_mut().insert(
                            CONTENT_ENCODING,
                            hyper::header::HeaderValue::from_static("br"),
                        );
                        res.headers_mut().insert(
                            CONTENT_LENGTH,
                            hyper::header::HeaderValue::from_str(&len.to_string()).unwrap(),
                        );
                        Ok(res)
                    } else if use_gzip {
                        let mut compressed = Vec::new();
                        {
                            let mut encoder = GzEncoder::new(&mut compressed, GzLevel::default());
                            encoder.write_all(&body_bytes).unwrap();
                            encoder.finish().unwrap();
                        }
                        let len = compressed.len();
                        let mut res =
                            hyper::Response::from_parts(parts, Full::new(Bytes::from(compressed)));
                        res.headers_mut().insert(
                            CONTENT_ENCODING,
                            hyper::header::HeaderValue::from_static("gzip"),
                        );
                        res.headers_mut().insert(
                            CONTENT_LENGTH,
                            hyper::header::HeaderValue::from_str(&len.to_string()).unwrap(),
                        );
                        Ok(res)
                    } else {
                        // No matching encoding — pass through unmodified.
                        Ok(hyper::Response::from_parts(parts, Full::new(body_bytes)))
                    }
                })
            })
        }
    }

    /// Compression middleware with defaults (gzip + brotli, min 1 KB).
    ///
    /// Convenience alias for `Compression::new().build()`.
    ///
    /// Requires the `compression` Cargo feature.
    ///
    /// ```
    /// # use ultimo::Ultimo;
    /// let mut app = Ultimo::new_without_defaults();
    /// app.use_middleware(ultimo::middleware::builtin::compression());
    /// ```
    #[cfg(feature = "compression")]
    pub fn compression() -> BoxedMiddleware {
        Compression::new().build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_chain_creation() {
        let chain = MiddlewareChain::new();
        assert_eq!(chain.middleware.len(), 0);
    }

    #[test]
    fn test_middleware_chain_push() {
        let mut chain = MiddlewareChain::new();
        let middleware: BoxedMiddleware =
            Arc::new(|ctx, next| Box::pin(async move { next(ctx).await }));

        chain.push(middleware.clone());
        assert_eq!(chain.middleware.len(), 1);

        chain.push(middleware);
        assert_eq!(chain.middleware.len(), 2);
    }

    #[test]
    fn test_cors_builder_creation() {
        let _cors = builtin::Cors::default();
        let _cors2 = builtin::Cors::new();
        // Just verify they compile
    }

    #[test]
    fn test_cors_builder_chaining() {
        let cors = builtin::Cors::new()
            .allow_origin("https://example.com")
            .allow_methods(vec!["GET", "POST"])
            .allow_headers(vec!["Authorization"]);

        // Build to verify it works
        let _middleware = cors.build();
    }

    #[test]
    fn test_cors_convenience_function() {
        let _cors = builtin::cors();
        // Just verify it compiles and returns middleware
    }

    #[test]
    fn test_logger_convenience_function() {
        let _logger = builtin::logger();
        // Just verify it compiles and returns middleware
    }

    #[test]
    fn test_powered_by_convenience_function() {
        let _powered_by = builtin::powered_by();
        // Just verify it compiles and returns middleware
    }

    #[test]
    fn test_middleware_chain_default() {
        let chain1 = MiddlewareChain::default();
        let chain2 = MiddlewareChain::new();
        assert_eq!(chain1.middleware.len(), chain2.middleware.len());
    }

    #[test]
    fn test_boxed_middleware_creation() {
        // Test that we can create BoxedMiddleware from a closure
        let _middleware: BoxedMiddleware = Arc::new(|ctx, next| {
            Box::pin(async move {
                // Do something before
                let result = next(ctx).await;
                // Do something after
                result
            })
        });
    }

    #[test]
    fn test_middleware_passthrough() {
        // Test creating a simple passthrough middleware
        let _passthrough: BoxedMiddleware =
            Arc::new(|ctx, next| Box::pin(async move { next(ctx).await }));
    }

    #[test]
    fn test_cors_multiple_methods() {
        let cors =
            builtin::Cors::new().allow_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"]);

        let _middleware = cors.build();
    }

    #[test]
    fn test_cors_multiple_headers() {
        let cors = builtin::Cors::new().allow_headers(vec![
            "Content-Type",
            "Authorization",
            "X-Custom-Header",
        ]);

        let _middleware = cors.build();
    }

    #[test]
    fn test_cors_custom_origin() {
        let cors = builtin::Cors::new().allow_origin("https://app.example.com");

        let _middleware = cors.build();
    }

    #[test]
    fn test_cors_builder_defaults() {
        let cors = builtin::Cors::default();
        // Verify defaults are set - just build to ensure no panics
        let _middleware = cors.build();
    }

    #[test]
    fn test_server_headers_builder() {
        let _middleware = builtin::server_headers("CustomServer", false);
        let _middleware_with_version = builtin::server_headers("Ultimo", true);
        // Just verify compilation and creation
    }

    #[test]
    fn test_cors_origin_string_conversion() {
        // Test that Into<String> works for various types
        let cors1 = builtin::Cors::new().allow_origin("https://example.com");
        let cors2 = builtin::Cors::new().allow_origin(String::from("https://test.com"));

        let _m1 = cors1.build();
        let _m2 = cors2.build();
    }

    #[test]
    fn test_cors_methods_string_conversion() {
        let cors = builtin::Cors::new().allow_methods(vec!["GET", "POST"]);

        let _middleware = cors.build();
    }

    #[test]
    fn test_cors_headers_string_conversion() {
        let cors = builtin::Cors::new().allow_headers(vec!["Content-Type"]);

        let _middleware = cors.build();
    }

    #[test]
    fn test_middleware_arc_clone() {
        let middleware: BoxedMiddleware =
            Arc::new(|ctx, next| Box::pin(async move { next(ctx).await }));

        let cloned = middleware.clone();
        // Verify Arc::clone works
        assert_eq!(Arc::strong_count(&middleware), Arc::strong_count(&cloned));
    }

    #[test]
    fn test_middleware_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<BoxedMiddleware>();
        assert_sync::<BoxedMiddleware>();
    }

    #[test]
    fn test_middleware_chain_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<MiddlewareChain>();
    }

    #[test]
    fn test_cors_struct_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<builtin::Cors>();
        assert_sync::<builtin::Cors>();
    }
}
