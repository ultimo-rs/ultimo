//! WebSocket implementation for Ultimo framework
//!
//! Provides a clean, type-safe WebSocket API with built-in pub/sub support,
//! inspired by Bun's WebSocket API design.

mod connection;
mod frame;
mod pubsub;
mod upgrade;

pub use connection::WebSocket;
pub use frame::Message;
pub use pubsub::ChannelManager;
pub use upgrade::WebSocketUpgrade;

// Test helpers - expose internal constructors for testing
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers {
    pub use super::connection::WebSocket;
    pub use super::frame::{Frame, Message, OpCode};
    pub use super::pubsub::ChannelManager;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    /// Test helper to create WebSocket instances
    pub fn create_websocket<T>(
        data: T,
        sender: mpsc::UnboundedSender<Message>,
        channel_manager: Arc<ChannelManager>,
        connection_id: uuid::Uuid,
        remote_addr: Option<SocketAddr>,
        config: Arc<super::WebSocketConfig>,
    ) -> WebSocket<T> {
        // Call the private constructor directly since we're in the same module
        super::connection::WebSocket::new(
            data,
            sender,
            channel_manager,
            connection_id,
            remote_addr,
            config,
        )
    }
}

/// WebSocket handler trait for implementing custom WebSocket logic
#[async_trait::async_trait]
pub trait WebSocketHandler: Send + Sync {
    /// Type of context data attached to each WebSocket connection
    type Data: Send + Sync + 'static;

    /// Called when WebSocket connection is established
    async fn on_open(&self, ws: &WebSocket<Self::Data>) {
        let _ = ws;
    }

    /// Called when a message is received from the client
    async fn on_message(&self, ws: &WebSocket<Self::Data>, msg: Message);

    /// Called when WebSocket connection is closed
    async fn on_close(&self, ws: &WebSocket<Self::Data>, code: u16, reason: &str) {
        let _ = (ws, code, reason);
    }

    /// Called when send buffer is writable again (backpressure handling)
    async fn on_drain(&self, ws: &WebSocket<Self::Data>) {
        let _ = ws;
    }

    /// Called when an error occurs
    async fn on_error(&self, ws: &WebSocket<Self::Data>, error: std::io::Error) {
        let _ = (ws, error);
    }
}

/// Configuration options for WebSocket connections
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Maximum message size in bytes (default: 64 MB)
    pub max_message_size: usize,

    /// Maximum frame size in bytes (default: 16 MB)
    pub max_frame_size: usize,

    /// Ping interval in seconds (default: 30 seconds, None to disable)
    pub ping_interval: Option<u64>,

    /// Ping timeout in seconds (default: 10 seconds)
    pub ping_timeout: u64,

    /// Enable per-message deflate compression (default: false)
    pub compression: bool,

    /// Write buffer size in bytes (default: 128 KB)
    pub write_buffer_size: usize,

    /// Maximum write queue size (default: 1024 messages)
    pub max_write_queue_size: usize,

    /// Accepted WebSocket subprotocols
    pub subprotocols: Vec<String>,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_message_size: 64 * 1024 * 1024, // 64 MB
            max_frame_size: 16 * 1024 * 1024,   // 16 MB
            ping_interval: Some(30),            // 30 seconds
            ping_timeout: 10,                   // 10 seconds
            compression: false,
            write_buffer_size: 128 * 1024, // 128 KB
            max_write_queue_size: 1024,
            subprotocols: vec![],
        }
    }
}
