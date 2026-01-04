//! WebSocket upgrade mechanism for Hyper

use super::connection::{ConnectionHandler, WebSocket};
use super::frame::Message;
use super::pubsub::ChannelManager;
use super::WebSocketConfig;
use bytes::Bytes;
use http_body_util::Full;
use hyper::header::{
    CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE,
};
use hyper::{Request as HyperRequest, Response as HyperResponse, StatusCode};
use sha1::{Digest, Sha1};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::mpsc;

const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// WebSocket upgrade builder
pub struct WebSocketUpgrade<T = ()> {
    request: HyperRequest<hyper::body::Incoming>,
    data: Option<T>,
    headers: Vec<(String, String)>,
    protocols: Vec<String>,
    config: WebSocketConfig,
    channel_manager: Arc<ChannelManager>,
}

impl<T> WebSocketUpgrade<T>
where
    T: Send + 'static,
{
    /// Create new WebSocket upgrade from HTTP request
    pub fn new(request: HyperRequest<hyper::body::Incoming>) -> Self {
        Self {
            request,
            data: None,
            headers: Vec::new(),
            protocols: Vec::new(),
            config: WebSocketConfig::default(),
            channel_manager: Arc::new(ChannelManager::new()),
        }
    }

    /// Set typed context data for the WebSocket
    pub fn with_data(mut self, data: T) -> Self {
        self.data = Some(data);
        self
    }

    /// Add custom header to upgrade response
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Set accepted WebSocket subprotocols
    pub fn with_protocols(mut self, protocols: Vec<String>) -> Self {
        self.protocols = protocols;
        self
    }

    /// Set WebSocket configuration
    pub fn with_config(mut self, config: WebSocketConfig) -> Self {
        self.config = config;
        self
    }

    /// Use shared channel manager for pub/sub
    pub fn with_channel_manager(mut self, channel_manager: Arc<ChannelManager>) -> Self {
        self.channel_manager = channel_manager;
        self
    }

    /// Set callback to be executed when WebSocket is upgraded
    pub fn on_upgrade<F, Fut>(self, callback: F) -> HyperResponse<Full<Bytes>>
    where
        F: FnOnce(WebSocket<T>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        T: Send + 'static,
    {
        // Validate WebSocket upgrade request
        if !is_valid_upgrade_request(&self.request) {
            return HyperResponse::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Invalid WebSocket upgrade request")))
                .unwrap();
        }

        // Extract WebSocket key
        let key = match self.request.headers().get(SEC_WEBSOCKET_KEY) {
            Some(key) => key.to_str().unwrap_or(""),
            None => {
                return HyperResponse::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from("Missing Sec-WebSocket-Key header")))
                    .unwrap();
            }
        };

        // Calculate accept key
        let accept_key = calculate_accept_key(key);

        // Build upgrade response
        let mut response = HyperResponse::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(UPGRADE, "websocket")
            .header(CONNECTION, "Upgrade")
            .header(SEC_WEBSOCKET_ACCEPT, accept_key);

        // Add custom headers
        for (key, value) in self.headers {
            response = response.header(key, value);
        }

        let response = response.body(Full::new(Bytes::new())).unwrap();

        // Spawn upgrade handler
        let data = self.data.expect("WebSocket data not set");
        let channel_manager = self.channel_manager;
        let config = Arc::new(self.config);

        tokio::spawn(async move {
            match hyper::upgrade::on(self.request).await {
                Ok(upgraded) => {
                    let (handler, sender, mut incoming_rx, mut _drain_rx) =
                        ConnectionHandler::new(upgraded, channel_manager.clone(), config.clone());
                    let connection_id = uuid::Uuid::new_v4();
                    let remote_addr = None; // TODO: Get from request

                    let ws = WebSocket::new(
                        data,
                        sender,
                        channel_manager,
                        connection_id,
                        remote_addr,
                        config.clone(),
                    );

                    // Spawn the connection handler
                    let handler_task = tokio::spawn(async move {
                        if let Err(e) = handler.handle().await {
                            tracing::error!("WebSocket handler error: {}", e);
                        }
                    });

                    // Spawn user callback with message receiver
                    let callback_task = tokio::spawn(async move {
                        // Call user callback first
                        callback(ws).await;

                        // Keep receiving messages to keep task alive
                        while incoming_rx.recv().await.is_some() {
                            // Messages handled by user's on_message callback
                        }
                    });

                    // Wait for both tasks
                    let _ = tokio::join!(handler_task, callback_task);
                }
                Err(e) => {
                    tracing::error!("WebSocket upgrade error: {}", e);
                }
            }
        });

        response
    }

    /// Set callback that receives incoming messages through a channel
    pub fn on_upgrade_with_receiver<F, Fut>(self, callback: F) -> HyperResponse<Full<Bytes>>
    where
        F: FnOnce(
                WebSocket<T>,
                mpsc::UnboundedReceiver<Message>,
                mpsc::UnboundedReceiver<()>,
            ) -> Fut
            + Send
            + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        T: Send + 'static,
    {
        // Validate WebSocket upgrade request
        if !is_valid_upgrade_request(&self.request) {
            return HyperResponse::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Invalid WebSocket upgrade request")))
                .unwrap();
        }

        // Extract WebSocket key
        let key = match self.request.headers().get(SEC_WEBSOCKET_KEY) {
            Some(key) => key.to_str().unwrap_or(""),
            None => {
                return HyperResponse::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from("Missing Sec-WebSocket-Key header")))
                    .unwrap();
            }
        };

        // Calculate accept key
        let accept_key = calculate_accept_key(key);

        // Build upgrade response
        let mut response = HyperResponse::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(UPGRADE, "websocket")
            .header(CONNECTION, "Upgrade")
            .header(SEC_WEBSOCKET_ACCEPT, accept_key);

        // Add custom headers
        for (key, value) in self.headers {
            response = response.header(key, value);
        }

        let response = response.body(Full::new(Bytes::new())).unwrap();

        // Spawn upgrade handler
        let data = self.data.expect("WebSocket data not set");
        let channel_manager = self.channel_manager;
        let config = Arc::new(self.config);

        tokio::spawn(async move {
            match hyper::upgrade::on(self.request).await {
                Ok(upgraded) => {
                    let (handler, sender, incoming_rx, drain_rx) =
                        ConnectionHandler::new(upgraded, channel_manager.clone(), config.clone());
                    let connection_id = uuid::Uuid::new_v4();
                    let remote_addr = None; // TODO: Get from request

                    let ws = WebSocket::new(
                        data,
                        sender,
                        channel_manager,
                        connection_id,
                        remote_addr,
                        config.clone(),
                    );

                    // Spawn the connection handler
                    let handler_task = tokio::spawn(async move {
                        if let Err(e) = handler.handle().await {
                            tracing::error!("WebSocket handler error: {}", e);
                        }
                    });

                    // Spawn user callback with message receiver
                    let callback_task = tokio::spawn(async move {
                        callback(ws, incoming_rx, drain_rx).await;
                    });

                    // Wait for both tasks
                    let _ = tokio::join!(handler_task, callback_task);
                }
                Err(e) => {
                    tracing::error!("WebSocket upgrade error: {}", e);
                }
            }
        });

        response
    }

    /// Build the upgrade response without a callback (for manual handling)
    pub fn build(self) -> HyperResponse<Full<Bytes>>
    where
        T: Default,
    {
        self.on_upgrade(|_ws| async {
            // Default handler does nothing
        })
    }
}

/// Check if request is a valid WebSocket upgrade request
fn is_valid_upgrade_request(req: &HyperRequest<hyper::body::Incoming>) -> bool {
    // Must be GET request
    if req.method() != hyper::Method::GET {
        return false;
    }

    // Must have Upgrade: websocket header
    let upgrade = req.headers().get(UPGRADE);
    if upgrade.is_none() || upgrade.unwrap().to_str().unwrap_or("").to_lowercase() != "websocket" {
        return false;
    }

    // Must have Connection: Upgrade header
    let connection = req.headers().get(CONNECTION);
    if connection.is_none() {
        return false;
    }

    // Must have Sec-WebSocket-Version: 13
    let version = req.headers().get(SEC_WEBSOCKET_VERSION);
    if version.is_none() || version.unwrap() != "13" {
        return false;
    }

    // Must have Sec-WebSocket-Key header
    if req.headers().get(SEC_WEBSOCKET_KEY).is_none() {
        return false;
    }

    true
}

/// Calculate WebSocket accept key from client key
fn calculate_accept_key(key: &str) -> String {
    use base64::{engine::general_purpose, Engine as _};
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WEBSOCKET_GUID.as_bytes());
    let result = hasher.finalize();
    general_purpose::STANDARD.encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_accept_key() {
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = calculate_accept_key(key);
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
}
