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
