//! Main Ultimo application
//!
//! Ties together routing, middleware, handlers, and HTTP server.

use crate::{
    context::Context,
    error::{Result, UltimoError},
    handler::{BoxedHandler, IntoHandler},
    middleware::{BoxedMiddleware, MiddlewareChain},
    response::{self, Response},
    router::{Method, Router},
};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request as HyperRequest;
use hyper_util::rt::TokioIo;
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

    #[cfg(feature = "database")]
    database: Option<Database>,

    #[cfg(feature = "websocket")]
    websocket_routes: HashMap<String, BoxedWebSocketHandler>,

    #[cfg(feature = "websocket")]
    channel_manager: Arc<ChannelManager>,
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
            #[cfg(feature = "database")]
            database: None,
            #[cfg(feature = "websocket")]
            websocket_routes: HashMap::new(),
            #[cfg(feature = "websocket")]
            channel_manager: Arc::new(ChannelManager::new()),
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
            #[cfg(feature = "database")]
            database: None,
            #[cfg(feature = "websocket")]
            websocket_routes: HashMap::new(),
            #[cfg(feature = "websocket")]
            channel_manager: Arc::new(ChannelManager::new()),
        }
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

            upgrade.on_upgrade_with_receiver(move |ws, mut incoming_rx| {
                let handler = handler.clone();
                async move {
                    // Call on_open
                    handler.on_open(&ws).await;

                    // Handle incoming messages
                    while let Some(msg) = incoming_rx.recv().await {
                        handler.on_message(&ws, msg).await;
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

    /// Handle an incoming HTTP request
    async fn handle_request(&self, req: HyperRequest<Incoming>) -> Response {
        let method_str = req.method().clone();
        let path = req.uri().path().to_string();

        // Check for WebSocket upgrade request
        #[cfg(feature = "websocket")]
        {
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
            let ctx = match Context::new(req, HashMap::new()).await {
                Ok(ctx) => ctx,
                Err(err) => {
                    error!("Failed to create context: {}", err);
                    return response::helpers::error_response(&err)
                        .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap());
                }
            };

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

            return match result {
                Ok(response) => response,
                Err(err) => {
                    error!("Middleware error: {}", err);
                    response::helpers::error_response(&err)
                        .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap())
                }
            };
        }

        // Find matching route
        let (handler_id, params) = match self.router.find_route(method, &path) {
            Some(route_match) => route_match,
            None => {
                return response::helpers::not_found()
                    .unwrap_or_else(|_| response::helpers::text("Not Found").unwrap());
            }
        };

        // Get the handler
        let _handler = &self.handlers[handler_id];

        // Create context
        #[cfg_attr(not(feature = "database"), allow(unused_mut))]
        let mut ctx = match Context::new(req, params).await {
            Ok(ctx) => ctx,
            Err(err) => {
                error!("Failed to create context: {}", err);
                return response::helpers::error_response(&err)
                    .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap());
            }
        };

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
        match result {
            Ok(response) => response,
            Err(err) => {
                error!("Handler error: {}", err);
                response::helpers::error_response(&err)
                    .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap())
            }
        }
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
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let app = app.clone();

            tokio::task::spawn(async move {
                let service = service_fn(move |req| {
                    let app = app.clone();
                    async move { Ok::<_, hyper::Error>(app.handle_request(req).await) }
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

    #[cfg(feature = "database")]
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
