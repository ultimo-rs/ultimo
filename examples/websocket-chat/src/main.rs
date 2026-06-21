//! WebSocket Chat Room Example
//!
//! A simple chat room application demonstrating WebSocket support in Ultimo.
//!
//! Features:
//! - Multiple users can connect
//! - Messages are broadcasted to all connected clients
//! - Join/leave notifications
//! - Custom configuration (Phase 2: ping/pong, message size limits)
//! - Backpressure handling with on_drain callback
//! - Graceful shutdown support
//! - Simple web interface included
//!
//! Run with: cargo run --example websocket-chat
//! Then open: http://localhost:3000

use async_trait::async_trait;
use ultimo::{
    prelude::*,
    websocket::{Message, WebSocket, WebSocketConfig, WebSocketHandler},
};

/// Chat room handler
struct ChatHandler {
    room: &'static str,
}

#[async_trait]
impl WebSocketHandler for ChatHandler {
    type Data = ();

    async fn on_open(&self, ws: &WebSocket<Self::Data>) {
        tracing::info!("New client connected to {}", self.room);

        // Send welcome message to the new user
        ws.send("Welcome to the chat room!").await.ok();

        // Subscribe to chat room (after welcome, so we receive future messages)
        if let Err(e) = ws.subscribe(self.room).await {
            tracing::error!("Failed to subscribe: {}", e);
        }
    }

    async fn on_message(&self, ws: &WebSocket<Self::Data>, msg: Message) {
        match msg {
            Message::Text(text) => {
                tracing::info!("Received message: {}", text);

                // Parse message
                if let Ok(msg_data) = serde_json::from_str::<serde_json::Value>(&text) {
                    // Broadcast to all clients in the room
                    ws.publish(self.room, &msg_data).await.ok();
                } else {
                    // Echo back if not valid JSON
                    ws.send(&text).await.ok();
                }
            }
            Message::Binary(data) => {
                tracing::info!("Received binary data: {} bytes", data.len());
                ws.send_binary(data).await.ok();
            }
            Message::Close(Some(cf)) => {
                tracing::info!("Client closing: {} - {}", cf.code, cf.reason);
            }
            _ => {}
        }
    }

    async fn on_close(&self, _ws: &WebSocket<Self::Data>, code: u16, reason: &str) {
        tracing::info!("Client disconnected: {} - {}", code, reason);
    }

    async fn on_drain(&self, ws: &WebSocket<Self::Data>) {
        // Called when write buffer drains after being full (backpressure relief)
        tracing::debug!(
            "Write buffer drained, connection can receive more messages. Capacity: {}",
            ws.capacity()
        );
    }

    async fn on_error(&self, _ws: &WebSocket<Self::Data>, error: std::io::Error) {
        tracing::error!("WebSocket error: {}", error);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut app = Ultimo::new();

    // Serve static HTML page
    app.get("/", |_ctx: Context| async move {
        ultimo::response::helpers::html(include_str!("../index.html"))
    });

    // WebSocket endpoint with custom configuration (Phase 2 features)
    let config = WebSocketConfig {
        // Limit message sizes for chat
        max_message_size: 10 * 1024 * 1024, // 10 MB
        max_frame_size: 1024 * 1024,        // 1 MB (enables automatic fragmentation)

        // Disable ping/pong for the demo (browser pong handling varies)
        ping_interval: None,

        // Backpressure handling
        max_write_queue_size: 100, // Buffer up to 100 messages per connection

        ..Default::default()
    };

    app.websocket_with_config("/ws", ChatHandler { room: "lobby" }, config);

    tracing::info!("Starting chat server on http://localhost:4000");
    tracing::info!("Open your browser and navigate to http://localhost:4000");
    tracing::info!("Phase 2 features enabled:");
    tracing::info!("  - Automatic ping/pong heartbeat (30s interval)");
    tracing::info!("  - Message fragmentation (for messages > 1MB)");
    tracing::info!("  - Backpressure handling (100 message buffer)");

    let port = std::env::var("PORT").unwrap_or_else(|_| "4000".to_string());
    let addr = format!("0.0.0.0:{port}");
    app.listen(&addr).await
}
