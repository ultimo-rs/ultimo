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
type BoxedWebSocketHandler =
    Arc<dyn Fn(WebSocketUpgrade<()>) -> crate::response::Response + Send + Sync>;

/// How long [`Ultimo::serve_with_shutdown`] waits for in-flight requests to
/// finish after a shutdown signal, unless overridden.
const DEFAULT_SHUTDOWN_GRACE_PERIOD: std::time::Duration = std::time::Duration::from_secs(30);

/// Main Ultimo application
pub struct Ultimo {
    router: Router,
    handlers: Vec<BoxedHandler>,
    middleware: Vec<BoxedMiddleware>,
    max_body_size: Option<usize>,
    request_timeout: Option<std::time::Duration>,
    shutdown_grace_period: std::time::Duration,
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
            request_timeout: None,
            shutdown_grace_period: DEFAULT_SHUTDOWN_GRACE_PERIOD,
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
            request_timeout: None,
            shutdown_grace_period: DEFAULT_SHUTDOWN_GRACE_PERIOD,
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

    /// Set a per-request timeout for routing + middleware + handler execution.
    ///
    /// A request that hasn't produced a response within `timeout` is answered
    /// with **408 Request Timeout** and its handler future is dropped
    /// (cancelled at its next `.await`). The timeout bounds the time to
    /// *produce* the response — a streamed body ([`Context::stream`],
    /// [`Context::sse`]) keeps flowing after the handler returns, and
    /// WebSocket upgrades are handled before dispatch, so neither is cut off.
    /// Defaults to no timeout.
    pub fn request_timeout(&mut self, timeout: std::time::Duration) -> &mut Self {
        self.request_timeout = Some(timeout);
        self
    }

    /// How long [`serve_with_shutdown`](Self::serve_with_shutdown) /
    /// [`listen_with_shutdown`](Self::listen_with_shutdown) wait for in-flight
    /// requests to finish after the shutdown signal before giving up and
    /// returning anyway. Defaults to 30 seconds.
    pub fn shutdown_grace_period(&mut self, grace: std::time::Duration) -> &mut Self {
        self.shutdown_grace_period = grace;
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
    pub fn get<Args>(
        &mut self,
        path: &str,
        handler: impl IntoHandler<Args> + 'static,
    ) -> &mut Self {
        self.add_route(Method::GET, path, handler)
    }

    /// Register an SSE route. SSE responses are served over `GET`, so this is
    /// sugar over [`get`](Self::get) that reads as intent. Return
    /// [`Context::sse`](crate::context::Context::sse) from the handler.
    pub fn sse<Args>(
        &mut self,
        path: &str,
        handler: impl IntoHandler<Args> + 'static,
    ) -> &mut Self {
        self.get(path, handler)
    }

    /// Add a POST route
    pub fn post<Args>(
        &mut self,
        path: &str,
        handler: impl IntoHandler<Args> + 'static,
    ) -> &mut Self {
        self.add_route(Method::POST, path, handler)
    }

    /// Add a PUT route
    pub fn put<Args>(
        &mut self,
        path: &str,
        handler: impl IntoHandler<Args> + 'static,
    ) -> &mut Self {
        self.add_route(Method::PUT, path, handler)
    }

    /// Add a DELETE route
    pub fn delete<Args>(
        &mut self,
        path: &str,
        handler: impl IntoHandler<Args> + 'static,
    ) -> &mut Self {
        self.add_route(Method::DELETE, path, handler)
    }

    /// Add a PATCH route
    pub fn patch<Args>(
        &mut self,
        path: &str,
        handler: impl IntoHandler<Args> + 'static,
    ) -> &mut Self {
        self.add_route(Method::PATCH, path, handler)
    }

    /// Add an OPTIONS route
    pub fn options<Args>(
        &mut self,
        path: &str,
        handler: impl IntoHandler<Args> + 'static,
    ) -> &mut Self {
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
        self.websocket_with_config_and_origins(path, handler, config, Vec::new())
    }

    /// Register a WebSocket handler with custom configuration and an
    /// `Origin` allow-list for the handshake (Cross-Site WebSocket
    /// Hijacking defense — see [`WebSocketUpgrade::with_allowed_origins`]).
    ///
    /// `allowed_origins` empty disables the check (matches
    /// [`websocket_with_config`](Self::websocket_with_config)). Set this
    /// whenever the connection relies on ambient cookie authentication.
    #[cfg(feature = "websocket")]
    pub fn websocket_with_config_and_origins<H>(
        &mut self,
        path: &str,
        handler: H,
        config: WebSocketConfig,
        allowed_origins: Vec<String>,
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
                .with_config(config.clone())
                .with_allowed_origins(allowed_origins.clone());

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
    fn add_route<Args>(
        &mut self,
        method: Method,
        path: &str,
        handler: impl IntoHandler<Args> + 'static,
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

    /// Serve interactive API documentation (Swagger UI) at the given path.
    ///
    /// Registers two routes:
    /// - `GET {path}` — Swagger UI HTML page
    /// - `GET {path}/openapi.json` — the OpenAPI JSON spec
    ///
    /// This is the Ultimo equivalent of FastAPI's `/docs`.
    ///
    /// ```rust,no_run
    /// use ultimo::prelude::*;
    /// use ultimo::openapi::OpenApiBuilder;
    ///
    /// let mut app = Ultimo::new();
    /// let spec = OpenApiBuilder::new()
    ///     .title("My API")
    ///     .version("1.0.0")
    ///     .build();
    /// app.serve_docs("/docs", spec);
    /// ```
    pub fn serve_docs(&mut self, path: &str, spec: crate::openapi::OpenApiSpec) -> &mut Self {
        let path = path.trim_end_matches('/');
        let spec_path = format!("{}/openapi.json", path);
        let ui_html = spec.swagger_ui_html(&spec_path);
        let spec_json = std::sync::Arc::new(spec);

        // Serve the OpenAPI JSON spec
        let spec_clone = spec_json.clone();
        self.get(&spec_path, move |ctx: Context| {
            let spec = spec_clone.clone();
            async move { ctx.json(spec.as_ref()).await }
        });

        // Serve the Swagger UI page
        self.get(path, move |ctx: Context| {
            let html = ui_html.clone();
            async move { ctx.html(html).await }
        });

        self
    }

    /// Mount an [`RpcRegistry`](crate::rpc::RpcRegistry) at `path` as a single
    /// JSON-RPC 2.0 endpoint (`POST`), handling single calls, batches, and
    /// notifications per the spec.
    ///
    /// This only applies to [`RpcMode::JsonRpc`](crate::rpc::RpcMode) (the
    /// default) — a single endpoint dispatches every procedure by name.
    /// [`RpcMode::Rest`](crate::rpc::RpcMode) mounts one route per procedure
    /// instead, which doesn't fit this one-call shape — wire up routes
    /// yourself for that mode (see the [RPC guide](https://docs.ultimo.dev/rpc)).
    ///
    /// # Example
    /// ```rust,no_run
    /// use ultimo::prelude::*;
    /// use ultimo::rpc::RpcRegistry;
    ///
    /// let mut app = Ultimo::new();
    /// let rpc = RpcRegistry::new();
    /// rpc.register("ping", |_: serde_json::Value| async move {
    ///     Ok(serde_json::json!("pong"))
    /// });
    /// app.mount_rpc("/rpc", rpc);
    /// ```
    pub fn mount_rpc(&mut self, path: &str, rpc: crate::rpc::RpcRegistry) -> &mut Self {
        self.post(path, move |ctx: Context| {
            let rpc = rpc.clone();
            async move {
                let body = ctx.req.bytes().await?;
                let output = rpc.handle_request(&body).await;
                match output.into_body() {
                    Some(bytes) => {
                        let value: serde_json::Value = serde_json::from_slice(&bytes)
                            .map_err(|e| UltimoError::Internal(e.to_string()))?;
                        ctx.json(value).await
                    }
                    None => {
                        // A notification (no id) produces no response body.
                        ctx.status(204).await;
                        ctx.text("").await
                    }
                }
            }
        })
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

    /// Run routing + middleware + handler against an already-buffered request,
    /// bounded by [`request_timeout`](Self::request_timeout) if one is set.
    async fn dispatch_parts(
        &self,
        parts: hyper::http::request::Parts,
        body: Bytes,
        client_addr: Option<SocketAddr>,
    ) -> Response {
        match self.request_timeout {
            Some(limit) => {
                match tokio::time::timeout(limit, self.dispatch_inner(parts, body, client_addr))
                    .await
                {
                    Ok(response) => response,
                    Err(_) => {
                        error!("Request timed out after {:?}", limit);
                        request_timeout_response()
                    }
                }
            }
            None => self.dispatch_inner(parts, body, client_addr).await,
        }
    }

    async fn dispatch_inner(
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

    /// Start the HTTP server on `addr`, shutting down gracefully when
    /// `shutdown` completes. See [`serve_with_shutdown`](Self::serve_with_shutdown).
    ///
    /// # Example
    /// ```rust,no_run
    /// use ultimo::prelude::*;
    ///
    /// # async fn run() -> ultimo::Result<()> {
    /// let mut app = Ultimo::new();
    /// app.get("/", |ctx: Context| async move { ctx.text("hi").await });
    /// app.listen_with_shutdown("127.0.0.1:3000", ultimo::shutdown_signal()).await
    /// # }
    /// ```
    pub async fn listen_with_shutdown<F>(self, addr: &str, shutdown: F) -> Result<()>
    where
        F: std::future::Future<Output = ()>,
    {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|_| UltimoError::Internal(format!("Invalid address: {}", addr)))?;
        let listener = TcpListener::bind(addr).await?;
        self.serve_with_shutdown(listener, shutdown).await
    }

    /// Serve on an already-bound `listener` until `shutdown` completes, then
    /// shut down gracefully:
    ///
    /// 1. stop accepting new connections;
    /// 2. send a `1001 Going Away` close frame to connected WebSocket clients
    ///    (with the `websocket` feature);
    /// 3. let in-flight HTTP requests finish, up to
    ///    [`shutdown_grace_period`](Self::shutdown_grace_period) (default 30s),
    ///    then return regardless.
    ///
    /// Taking a bound listener (rather than an address) lets callers bind port
    /// `0` and read the chosen port from `listener.local_addr()`.
    pub async fn serve_with_shutdown<F>(self, listener: TcpListener, shutdown: F) -> Result<()>
    where
        F: std::future::Future<Output = ()>,
    {
        if let Ok(addr) = listener.local_addr() {
            info!("🚀 Ultimo server listening on http://{}", addr);
        }

        let grace = self.shutdown_grace_period;
        let app = Arc::new(self);
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let mut connections = tokio::task::JoinSet::new();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let (stream, peer_addr) = accepted?;
                    let io = TokioIo::new(stream);
                    let app = app.clone();
                    let mut stop_rx = stop_rx.clone();

                    connections.spawn(async move {
                        let service = service_fn(move |req| {
                            let app = app.clone();
                            async move {
                                Ok::<_, hyper::Error>(app.handle_request(req, peer_addr).await)
                            }
                        });
                        let conn = http1::Builder::new()
                            .serve_connection(io, service)
                            .with_upgrades(); // Enable HTTP upgrades for WebSockets
                        tokio::pin!(conn);

                        let mut stopping = false;
                        loop {
                            tokio::select! {
                                res = conn.as_mut() => {
                                    if let Err(err) = res {
                                        error!("Connection error: {}", err);
                                    }
                                    break;
                                }
                                _ = stop_rx.changed(), if !stopping => {
                                    // Finish the in-flight request, then close.
                                    stopping = true;
                                    conn.as_mut().graceful_shutdown();
                                }
                            }
                        }
                    });
                }
                // Reap finished connection tasks so the set doesn't grow forever.
                Some(_) = connections.join_next(), if !connections.is_empty() => {}
                _ = &mut shutdown => break,
            }
        }

        info!("Shutting down: no longer accepting connections");
        drop(listener);

        #[cfg(feature = "websocket")]
        {
            let going_away = crate::websocket::Message::Close(Some(crate::websocket::CloseFrame {
                code: 1001,
                reason: "Server shutting down".to_string(),
            }));
            let notified = app.channel_manager.broadcast_all(going_away).await;
            if notified > 0 {
                info!("Sent close frames to {} WebSocket connection(s)", notified);
            }
        }

        let _ = stop_tx.send(true);
        let drained = tokio::time::timeout(grace, async {
            while connections.join_next().await.is_some() {}
        })
        .await;
        match drained {
            Ok(()) => info!("All in-flight requests drained"),
            Err(_) => error!(
                "Shutdown grace period ({:?}) elapsed with requests still in flight",
                grace
            ),
        }
        Ok(())
    }
}

/// A future that completes when the process receives Ctrl+C (all platforms)
/// or `SIGTERM` (Unix) — the signal container orchestrators send to stop a
/// service. Pass it to [`Ultimo::listen_with_shutdown`].
pub async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// 408 response for a request that exceeded `request_timeout`.
fn request_timeout_response() -> Response {
    response::ResponseBuilder::new()
        .status(408)
        .text("Request Timeout")
        .build()
        .unwrap_or_else(|_| response::helpers::text("Request Timeout").unwrap())
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

    #[test]
    fn test_serve_docs_registers_routes() {
        let mut app = Ultimo::new_without_defaults();
        let spec = crate::openapi::OpenApiBuilder::new()
            .title("Test API")
            .version("1.0.0")
            .build();
        app.serve_docs("/docs", spec);
        // Should register 2 routes: /docs and /docs/openapi.json
        assert_eq!(app.handlers.len(), 2);
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

    #[tokio::test]
    async fn mount_rpc_single_call_returns_200_with_result() {
        let mut app = Ultimo::new_without_defaults();
        let rpc = crate::rpc::RpcRegistry::new();
        rpc.register("add", |input: serde_json::Value| async move {
            let a = input.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
            let b = input.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(serde_json::to_value(a + b).unwrap())
        });
        app.mount_rpc("/rpc", rpc);

        let body = serde_json::to_vec(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "add",
            "params": {"a": 3, "b": 4},
            "id": 1
        }))
        .unwrap();
        let req = HyperRequest::builder()
            .method("POST")
            .uri("/rpc")
            .body(Full::new(bytes::Bytes::from(body)))
            .unwrap();

        let resp = app.oneshot(req).await;
        assert_eq!(resp.status(), 200);
        let text = body_string(resp).await;
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["result"], 7);
    }

    #[tokio::test]
    async fn mount_rpc_notification_returns_204_with_empty_body() {
        let mut app = Ultimo::new_without_defaults();
        let rpc = crate::rpc::RpcRegistry::new();
        rpc.register("add", |input: serde_json::Value| async move {
            let a = input.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
            let b = input.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(serde_json::to_value(a + b).unwrap())
        });
        app.mount_rpc("/rpc", rpc);

        // No "id" field => a notification, which produces no response body.
        let body = serde_json::to_vec(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "add",
            "params": {"a": 1, "b": 2}
        }))
        .unwrap();
        let req = HyperRequest::builder()
            .method("POST")
            .uri("/rpc")
            .body(Full::new(bytes::Bytes::from(body)))
            .unwrap();

        let resp = app.oneshot(req).await;
        assert_eq!(resp.status(), 204);
        assert_eq!(body_string(resp).await, "");
    }
}

#[cfg(test)]
mod timeout_and_shutdown_tests {
    use super::*;
    use http_body_util::Full;
    use hyper::Request as HyperRequest;
    use std::time::{Duration, Instant};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn get(uri: &str) -> HyperRequest<Full<Bytes>> {
        HyperRequest::builder()
            .uri(uri)
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    #[tokio::test]
    async fn request_timeout_returns_408_when_handler_is_too_slow() {
        let mut app = Ultimo::new_without_defaults();
        app.request_timeout(Duration::from_millis(20));
        app.get("/slow", |ctx: Context| async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            ctx.text("late").await
        });

        let started = Instant::now();
        let resp = app.oneshot(get("/slow")).await;
        assert_eq!(resp.status(), 408);
        assert!(started.elapsed() < Duration::from_millis(400));
    }

    #[tokio::test]
    async fn request_within_timeout_is_unaffected() {
        let mut app = Ultimo::new_without_defaults();
        app.request_timeout(Duration::from_millis(500));
        app.get("/fast", |ctx: Context| async move { ctx.text("ok").await });

        assert_eq!(app.oneshot(get("/fast")).await.status(), 200);
    }

    #[tokio::test]
    async fn no_timeout_configured_never_times_out() {
        let mut app = Ultimo::new_without_defaults();
        app.get("/slowish", |ctx: Context| async move {
            tokio::time::sleep(Duration::from_millis(60)).await;
            ctx.text("ok").await
        });

        assert_eq!(app.oneshot(get("/slowish")).await.status(), 200);
    }

    async fn raw_get(addr: std::net::SocketAddr, path: &str) -> String {
        let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
        s.write_all(
            format!("GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .await
        .unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).await.unwrap();
        out
    }

    #[tokio::test]
    async fn graceful_shutdown_drains_in_flight_requests_then_stops_accepting() {
        let mut app = Ultimo::new_without_defaults();
        app.shutdown_grace_period(Duration::from_secs(5));
        app.get("/slow", |ctx: Context| async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            ctx.text("drained").await
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(app.serve_with_shutdown(listener, async {
            rx.await.ok();
        }));

        let in_flight = tokio::spawn(raw_get(addr, "/slow"));
        tokio::time::sleep(Duration::from_millis(50)).await;
        tx.send(()).unwrap();

        // In-flight request still completes successfully...
        let body = in_flight.await.unwrap();
        assert!(
            body.contains("200 OK") && body.contains("drained"),
            "{body}"
        );
        // ...and the server then exits cleanly.
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .expect("server should exit after draining")
            .unwrap()
            .unwrap();
        // New connections are refused once shut down.
        assert!(tokio::net::TcpStream::connect(addr).await.is_err());
    }

    #[tokio::test]
    async fn graceful_shutdown_gives_up_after_grace_period() {
        let mut app = Ultimo::new_without_defaults();
        app.shutdown_grace_period(Duration::from_millis(100));
        app.get("/stuck", |ctx: Context| async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
            ctx.text("never").await
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(app.serve_with_shutdown(listener, async {
            rx.await.ok();
        }));

        let _stuck = tokio::spawn(raw_get(addr, "/stuck"));
        tokio::time::sleep(Duration::from_millis(50)).await;
        let started = Instant::now();
        tx.send(()).unwrap();

        tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .expect("server must not wait past the grace period")
            .unwrap()
            .unwrap();
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
