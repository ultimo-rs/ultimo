//! Main Ultimo application
//!
//! Ties together routing, middleware, handlers, and HTTP server.

use crate::{
    context::Context,
    error::{Result, UltimoError},
    handler::{BoxedHandler, IntoHandler},
    middleware::{BoxedMiddleware, MiddlewareChain},
    response::{self, Response},
    router::{Method, Params, Router},
};
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request as HyperRequest;
use hyper_util::rt::TokioIo;
#[cfg(feature = "websocket")]
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

#[cfg(feature = "database")]
use crate::database::Database;

#[cfg(feature = "websocket")]
use crate::websocket::{ChannelManager, WebSocketConfig, WebSocketHandler, WebSocketUpgrade};

/// WebSocket handler function type
#[cfg(feature = "websocket")]
type BoxedWebSocketHandler = Arc<
    dyn Fn(WebSocketUpgrade<()>) -> hyper::Response<http_body_util::Full<bytes::Bytes>>
        + Send
        + Sync,
>;

/// Main Ultimo application
pub struct Ultimo {
    router: Router,
    handlers: Vec<BoxedHandler>,
    middleware: Vec<BoxedMiddleware>,
    max_body_size: Option<usize>,
    trust_proxy: bool,

    #[cfg(feature = "database")]
    database: Option<Database>,

    #[cfg(feature = "websocket")]
    websocket_routes: HashMap<String, BoxedWebSocketHandler>,

    #[cfg(feature = "websocket")]
    channel_manager: Arc<ChannelManager>,

    /// SPA fallback: `(root_dir, fallback_filename)`. When set, any `GET`
    /// request that returns 404 is answered with this file instead.
    #[cfg(feature = "static-files")]
    spa_fallback: Option<(std::path::PathBuf, String)>,
}

impl Ultimo {
    /// Create a new Ultimo application
    ///
    /// By default, adds `X-Powered-By: Ultimo` header to all responses.
    /// To disable this, use `new_without_defaults()` instead.
    pub fn new() -> Self {
        let mut app = Self {
            router: Router::new(),
            handlers: Vec::new(),
            middleware: Vec::new(),
            max_body_size: None,
            trust_proxy: false,
            #[cfg(feature = "database")]
            database: None,
            #[cfg(feature = "websocket")]
            websocket_routes: HashMap::new(),
            #[cfg(feature = "websocket")]
            channel_manager: Arc::new(ChannelManager::new()),
            #[cfg(feature = "static-files")]
            spa_fallback: None,
        };

        // Add X-Powered-By header by default (like Express.js)
        app.middleware
            .push(crate::middleware::builtin::powered_by());

        app
    }

    /// Create a new Ultimo application without default middleware
    ///
    /// Use this if you don't want the `X-Powered-By: Ultimo` header
    /// or want full control over middleware configuration.
    pub fn new_without_defaults() -> Self {
        Self {
            router: Router::new(),
            handlers: Vec::new(),
            middleware: Vec::new(),
            max_body_size: None,
            trust_proxy: false,
            #[cfg(feature = "database")]
            database: None,
            #[cfg(feature = "websocket")]
            websocket_routes: HashMap::new(),
            #[cfg(feature = "websocket")]
            channel_manager: Arc::new(ChannelManager::new()),
            #[cfg(feature = "static-files")]
            spa_fallback: None,
        }
    }

    /// Set the maximum request body size in bytes.
    ///
    /// Requests whose body exceeds this are rejected with **413 Payload Too
    /// Large** (and, on the live server, the oversized body is not buffered).
    /// Defaults to no limit — setting one is recommended for production.
    pub fn max_body_size(&mut self, bytes: usize) -> &mut Self {
        self.max_body_size = Some(bytes);
        self
    }

    /// Trust `X-Forwarded-For` / `Forwarded` headers for [`Context::client_ip`].
    ///
    /// **Only enable when the app sits behind a trusted proxy/load balancer** —
    /// these headers are client-spoofable, so trusting them on a directly-exposed
    /// server lets clients forge their IP. Defaults to `false`.
    pub fn trust_proxy(&mut self, trust: bool) -> &mut Self {
        self.trust_proxy = trust;
        self
    }

    /// Attach a SQLx database pool to the application
    #[cfg(feature = "sqlx")]
    pub fn with_sqlx<DB>(&mut self, pool: crate::database::sqlx::SqlxPool<DB>) -> &mut Self
    where
        DB: sqlx::Database + 'static,
    {
        self.database = Some(Database::from_sqlx(pool));
        self
    }

    /// Attach a Diesel database pool to the application
    #[cfg(feature = "diesel")]
    pub fn with_diesel<Conn>(
        &mut self,
        pool: crate::database::diesel::DieselPool<Conn>,
    ) -> &mut Self
    where
        Conn: diesel::Connection + diesel::r2d2::R2D2Connection + 'static,
    {
        self.database = Some(Database::from_diesel(pool));
        self
    }

    /// Add a GET route
    pub fn get(&mut self, path: &str, handler: impl IntoHandler + 'static) -> &mut Self {
        self.add_route(Method::GET, path, handler)
    }

    /// Add a POST route
    pub fn post(&mut self, path: &str, handler: impl IntoHandler + 'static) -> &mut Self {
        self.add_route(Method::POST, path, handler)
    }

    /// Add a PUT route
    pub fn put(&mut self, path: &str, handler: impl IntoHandler + 'static) -> &mut Self {
        self.add_route(Method::PUT, path, handler)
    }

    /// Add a DELETE route
    pub fn delete(&mut self, path: &str, handler: impl IntoHandler + 'static) -> &mut Self {
        self.add_route(Method::DELETE, path, handler)
    }

    /// Add a PATCH route
    pub fn patch(&mut self, path: &str, handler: impl IntoHandler + 'static) -> &mut Self {
        self.add_route(Method::PATCH, path, handler)
    }

    /// Add an OPTIONS route
    pub fn options(&mut self, path: &str, handler: impl IntoHandler + 'static) -> &mut Self {
        self.add_route(Method::OPTIONS, path, handler)
    }

    /// Add a WebSocket route
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ultimo::prelude::*;
    /// use ultimo::websocket::{WebSocketHandler, WebSocket, Message};
    ///
    /// struct ChatHandler;
    ///
    /// #[async_trait::async_trait]
    /// impl WebSocketHandler for ChatHandler {
    ///     type Data = ();
    ///
    ///     async fn on_open(&self, ws: &WebSocket<Self::Data>) {
    ///         println!("Client connected!");
    ///     }
    ///
    ///     async fn on_message(&self, ws: &WebSocket<Self::Data>, msg: Message) {
    ///         if let Message::Text(text) = msg {
    ///             ws.send(&text).await.ok();
    ///         }
    ///     }
    /// }
    ///
    /// # async {
    /// let mut app = Ultimo::new();
    /// app.websocket("/ws", ChatHandler);
    /// # };
    /// ```
    #[cfg(feature = "websocket")]
    pub fn websocket<H>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: WebSocketHandler<Data = ()> + 'static,
    {
        self.websocket_with_config(path, handler, WebSocketConfig::default())
    }

    /// Register a WebSocket handler with custom configuration
    #[cfg(feature = "websocket")]
    pub fn websocket_with_config<H>(
        &mut self,
        path: &str,
        handler: H,
        config: WebSocketConfig,
    ) -> &mut Self
    where
        H: WebSocketHandler<Data = ()> + 'static,
    {
        let handler = Arc::new(handler);
        let channel_manager = self.channel_manager.clone();

        let ws_handler = move |upgrade: WebSocketUpgrade<()>| {
            let handler = handler.clone();
            let upgrade = upgrade
                .with_data(())
                .with_channel_manager(channel_manager.clone())
                .with_config(config.clone());

            upgrade.on_upgrade_with_receiver(move |ws, mut incoming_rx, mut drain_rx| {
                let handler = handler.clone();
                async move {
                    // Call on_open
                    handler.on_open(&ws).await;

                    // Handle incoming messages and drain notifications
                    loop {
                        tokio::select! {
                            Some(msg) = incoming_rx.recv() => {
                                handler.on_message(&ws, msg).await;
                            }
                            Some(_) = drain_rx.recv() => {
                                handler.on_drain(&ws).await;
                            }
                            else => break,
                        }
                    }

                    // Call on_close when connection ends
                    handler.on_close(&ws, 1000, "Connection closed").await;
                }
            })
        };

        self.websocket_routes
            .insert(path.to_string(), Arc::new(ws_handler));
        self
    }

    /// Add a route with any method
    fn add_route(
        &mut self,
        method: Method,
        path: &str,
        handler: impl IntoHandler + 'static,
    ) -> &mut Self {
        let handler_id = self.handlers.len();
        self.handlers.push(handler.into_handler());
        self.router.add_route(method, path, handler_id);
        self
    }

    /// Add global middleware
    pub fn use_middleware(&mut self, middleware: BoxedMiddleware) -> &mut Self {
        self.middleware.push(middleware);
        self
    }

    /// Serve static files from `dir` under the URL prefix `prefix`.
    ///
    /// Registers a `GET {prefix}/*path` route. Responds with the correct
    /// `Content-Type`, sets an `ETag`, and handles `If-None-Match` → 304.
    /// Path traversal attempts return 404.
    ///
    /// Requires the `static-files` Cargo feature.
    ///
    /// ```rust,no_run
    /// use ultimo::prelude::*;
    ///
    /// let mut app = Ultimo::new();
    /// app.serve_static("/assets", "./public");
    /// ```
    #[cfg(feature = "static-files")]
    pub fn serve_static(&mut self, prefix: &str, dir: impl Into<std::path::PathBuf>) -> &mut Self {
        let root = dir.into();
        let pattern = format!("{}/*path", prefix.trim_end_matches('/'));
        self.get(&pattern, move |ctx: Context| {
            let root = root.clone();
            async move {
                let rel = ctx.req.param("path")?.to_string();
                let inm = ctx.req.header("if-none-match");
                crate::static_files::serve_file(&root, &rel, inm).await
            }
        });
        self
    }

    /// Serve a Single Page Application from `dir`.
    ///
    /// Any `GET` request that returns 404 (no matching route) is answered
    /// with `dir/fallback` instead, enabling client-side routing.
    ///
    /// Mount API routes **before** calling `serve_spa` so they take
    /// precedence.
    ///
    /// Requires the `static-files` Cargo feature.
    ///
    /// ```rust,no_run
    /// use ultimo::prelude::*;
    ///
    /// let mut app = Ultimo::new();
    /// app.get("/api/hello", |ctx: Context| async move {
    ///     ctx.json(serde_json::json!({ "ok": true })).await
    /// });
    /// app.serve_spa("./dist", "index.html");
    /// ```
    #[cfg(feature = "static-files")]
    pub fn serve_spa(&mut self, dir: impl Into<std::path::PathBuf>, fallback: &str) -> &mut Self {
        self.spa_fallback = Some((dir.into(), fallback.to_string()));
        self
    }

    /// Handle an incoming HTTP request
    async fn handle_request(&self, req: HyperRequest<Incoming>, peer_addr: SocketAddr) -> Response {
        // Check for WebSocket upgrade request (needs the live `Incoming` body)
        #[cfg(feature = "websocket")]
        {
            let path = req.uri().path().to_string();
            if let Some(ws_handler) = self.websocket_routes.get(&path) {
                // Check if this is a WebSocket upgrade request
                if req
                    .headers()
                    .get(hyper::header::UPGRADE)
                    .and_then(|v| v.to_str().ok())
                    .map(|v| v.eq_ignore_ascii_case("websocket"))
                    .unwrap_or(false)
                {
                    let upgrade = WebSocketUpgrade::new(req);
                    return ws_handler(upgrade);
                }
            }
        }

        // Buffer the body (capped if a max is configured, so an oversized body
        // is never fully buffered), then dispatch through the body-agnostic core.
        let (parts, body) = req.into_parts();
        let bytes = match self.max_body_size {
            Some(max) => match http_body_util::Limited::new(body, max).collect().await {
                Ok(c) => c.to_bytes(),
                Err(e) => {
                    if e.downcast_ref::<http_body_util::LengthLimitError>()
                        .is_some()
                    {
                        return body_too_large();
                    }
                    error!("Failed to read body: {}", e);
                    return internal_error();
                }
            },
            None => match body.collect().await {
                Ok(c) => c.to_bytes(),
                Err(e) => {
                    error!("Failed to read body: {}", e);
                    return internal_error();
                }
            },
        };
        self.dispatch_parts(parts, bytes, Some(peer_addr)).await
    }

    /// Run routing + middleware + handler against an already-buffered request.
    async fn dispatch_parts(
        &self,
        parts: hyper::http::request::Parts,
        body: Bytes,
        client_addr: Option<SocketAddr>,
    ) -> Response {
        let method_str = parts.method.clone();
        let path = parts.uri.path().to_string();

        // Enforce the body-size limit (covers in-process dispatch + a backstop
        // for the live path).
        if let Some(max) = self.max_body_size {
            if body.len() > max {
                return body_too_large();
            }
        }

        // Parse method
        let method = match Method::from_hyper(&method_str) {
            Some(m) => m,
            None => {
                return response::helpers::error_response(&UltimoError::BadRequest(format!(
                    "Unsupported HTTP method: {}",
                    method_str
                )))
                .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap());
            }
        };

        // Handle OPTIONS requests through middleware before routing
        // This allows CORS middleware to respond to preflight requests
        if method_str == hyper::Method::OPTIONS {
            // Create context for OPTIONS request
            let mut ctx = Context::from_parts(parts, body, Params::new());
            ctx.set_client(client_addr, self.trust_proxy);
            let cookie_sink = ctx.set_cookies_handle();

            // Build and execute middleware chain
            let mut chain = MiddlewareChain::new();
            for middleware in &self.middleware {
                chain.push(middleware.clone());
            }

            // Execute with a dummy handler that returns 404
            // CORS middleware should intercept OPTIONS and return early
            let result = chain
                .execute(ctx, |_ctx| async move {
                    Ok(response::helpers::not_found()
                        .unwrap_or_else(|_| response::helpers::text("Not Found").unwrap()))
                })
                .await;

            let response = match result {
                Ok(response) => response,
                Err(err) => {
                    error!("Middleware error: {}", err);
                    response::helpers::error_response(&err)
                        .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap())
                }
            };
            return flush_set_cookies(response, cookie_sink).await;
        }

        // Find matching route
        let (handler_id, params) = match self.router.find_route(method, &path) {
            Some(route_match) => route_match,
            None => {
                // SPA fallback: serve index.html for unmatched GET requests.
                #[cfg(feature = "static-files")]
                if parts.method == hyper::Method::GET {
                    if let Some((ref spa_dir, ref spa_file)) = self.spa_fallback {
                        if let Ok(spa_resp) =
                            crate::static_files::serve_file(spa_dir, spa_file, None).await
                        {
                            return spa_resp;
                        }
                    }
                }
                return response::helpers::not_found()
                    .unwrap_or_else(|_| response::helpers::text("Not Found").unwrap());
            }
        };

        // Get the handler
        let _handler = &self.handlers[handler_id];

        // Create context
        let mut ctx = Context::from_parts(parts, body, params);
        ctx.set_client(client_addr, self.trust_proxy);
        let cookie_sink = ctx.set_cookies_handle();

        // Attach database if configured
        #[cfg(feature = "database")]
        if let Some(ref db) = self.database {
            ctx.attach_database(db.clone());
        }

        // Build middleware chain
        let mut chain = MiddlewareChain::new();
        for middleware in &self.middleware {
            chain.push(middleware.clone());
        }

        // Get the handler
        let handler = self.handlers[handler_id].clone();

        // Execute middleware chain with handler
        let result = chain
            .execute(ctx, move |ctx| async move { handler(ctx).await })
            .await;

        // Handle result
        let response = match result {
            Ok(response) => response,
            Err(err) => {
                error!("Handler error: {}", err);
                response::helpers::error_response(&err)
                    .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap())
            }
        };
        flush_set_cookies(response, cookie_sink).await
    }

    /// Dispatch a fully-buffered request through the app in-process (no socket).
    pub async fn oneshot(&self, req: HyperRequest<http_body_util::Full<Bytes>>) -> Response {
        let (parts, body) = req.into_parts();
        let bytes = body
            .collect()
            .await
            .map(|c| c.to_bytes())
            .unwrap_or_default();
        self.dispatch_parts(parts, bytes, None).await
    }

    /// Start the HTTP server
    pub async fn listen(self, addr: &str) -> Result<()> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|_| UltimoError::Internal(format!("Invalid address: {}", addr)))?;

        let listener = TcpListener::bind(addr).await?;
        info!("🚀 Ultimo server listening on http://{}", addr);

        // Wrap self in Arc for sharing across connections
        let app = Arc::new(self);

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let app = app.clone();

            tokio::task::spawn(async move {
                let service = service_fn(move |req| {
                    let app = app.clone();
                    async move { Ok::<_, hyper::Error>(app.handle_request(req, peer_addr).await) }
                });

                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .with_upgrades() // Enable HTTP upgrades for WebSockets
                    .await
                {
                    error!("Connection error: {}", err);
                }
            });
        }
    }
}

/// 413 Payload Too Large response (body exceeded `max_body_size`).
fn body_too_large() -> Response {
    response::ResponseBuilder::new()
        .status(413)
        .text("Payload Too Large")
        .build()
        .unwrap_or_else(|_| response::helpers::text("Payload Too Large").unwrap())
}

/// 500 response for a genuine body-read failure (preserves the JSON error shape).
fn internal_error() -> Response {
    response::helpers::error_response(&UltimoError::Internal("Failed to read body".to_string()))
        .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap())
}

/// Append queued `Set-Cookie` header values (from `ctx.set_cookie`) onto the
/// response. Uses `append` so multiple cookies become multiple headers.
async fn flush_set_cookies(
    mut response: Response,
    sink: Arc<tokio::sync::RwLock<Vec<String>>>,
) -> Response {
    let cookies = std::mem::take(&mut *sink.write().await);
    for value in cookies {
        if let Ok(hv) = hyper::header::HeaderValue::from_str(&value) {
            response.headers_mut().append(hyper::header::SET_COOKIE, hv);
        }
    }
    response
}

impl Default for Ultimo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = Ultimo::new();
        assert_eq!(app.handlers.len(), 0);
        // new() adds X-Powered-By middleware by default
        assert_eq!(app.middleware.len(), 1);
    }

    #[test]
    fn test_app_creation_without_defaults() {
        let app = Ultimo::new_without_defaults();
        assert_eq!(app.handlers.len(), 0);
        // new_without_defaults() has no middleware
        assert_eq!(app.middleware.len(), 0);
    }

    #[test]
    fn test_app_default() {
        let app = Ultimo::default();
        // Default should be same as new()
        assert_eq!(app.middleware.len(), 1);
    }

    #[test]
    fn test_add_routes() {
        let mut app = Ultimo::new();

        app.get(
            "/users",
            |ctx: Context| async move { ctx.text("users").await },
        );

        app.post("/users", |ctx: Context| async move {
            ctx.text("create user").await
        });

        assert_eq!(app.handlers.len(), 2);
    }

    #[test]
    fn test_route_methods() {
        let mut app = Ultimo::new_without_defaults();

        app.get("/get", |ctx: Context| async move { ctx.text("GET").await });
        app.post(
            "/post",
            |ctx: Context| async move { ctx.text("POST").await },
        );
        app.put("/put", |ctx: Context| async move { ctx.text("PUT").await });
        app.patch(
            "/patch",
            |ctx: Context| async move { ctx.text("PATCH").await },
        );
        app.delete(
            "/delete",
            |ctx: Context| async move { ctx.text("DELETE").await },
        );

        assert_eq!(app.handlers.len(), 5);
    }

    #[test]
    fn test_middleware_addition() {
        use crate::middleware::builtin::logger;

        let mut app = Ultimo::new_without_defaults();
        assert_eq!(app.middleware.len(), 0);

        // Add middleware using builtin
        app.use_middleware(logger());
        assert_eq!(app.middleware.len(), 1);

        // Add another
        app.use_middleware(logger());
        assert_eq!(app.middleware.len(), 2);
    }

    #[test]
    fn test_chaining_routes() {
        let mut app = Ultimo::new_without_defaults();

        app.get("/a", |ctx: Context| async move { ctx.text("a").await })
            .get("/b", |ctx: Context| async move { ctx.text("b").await })
            .post("/c", |ctx: Context| async move { ctx.text("c").await });

        assert_eq!(app.handlers.len(), 3);
    }

    #[test]
    fn test_parameterized_routes() {
        let mut app = Ultimo::new_without_defaults();

        app.get("/users/:id", |ctx: Context| async move {
            ctx.text("user detail").await
        });

        app.get("/posts/:slug/comments/:id", |ctx: Context| async move {
            ctx.text("comment").await
        });

        assert_eq!(app.handlers.len(), 2);
    }

    // Gated on `sqlx` (not just `database`) because it constructs the
    // Database::Sqlx variant, which only exists with the sqlx backend.
    #[cfg(feature = "sqlx")]
    #[test]
    fn test_database_attachment() {
        use std::sync::Arc;

        let mut app = Ultimo::new_without_defaults();

        // Test that database field exists and is None by default
        assert!(app.database.is_none());

        // Mock database attachment (we can't create real pools in unit tests)
        let mock_pool = Arc::new(42);
        app.database = Some(Database::Sqlx(mock_pool));

        assert!(app.database.is_some());
    }

    #[test]
    fn test_app_is_send_sync() {
        // Ensure Ultimo can be used across threads
        fn assert_send<T: Send>() {}

        assert_send::<Ultimo>();
        // Note: Ultimo is not Sync because it contains non-Sync types
        // This is OK since we Arc it in listen()
    }
}

#[cfg(test)]
mod oneshot_tests {
    use super::*;
    use http_body_util::{BodyExt, Full};
    use hyper::Request as HyperRequest;

    async fn body_string(resp: Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn oneshot_routes_and_returns_response() {
        let mut app = Ultimo::new_without_defaults();
        app.get(
            "/ping",
            |ctx: Context| async move { ctx.text("pong").await },
        );

        let req = HyperRequest::builder()
            .method("GET")
            .uri("/ping")
            .body(Full::new(bytes::Bytes::new()))
            .unwrap();

        let resp = app.oneshot(req).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(body_string(resp).await, "pong");
    }

    #[tokio::test]
    async fn oneshot_unknown_route_is_404() {
        let app = Ultimo::new_without_defaults();
        let req = HyperRequest::builder()
            .uri("/nope")
            .body(Full::new(bytes::Bytes::new()))
            .unwrap();
        assert_eq!(app.oneshot(req).await.status(), 404);
    }
}
