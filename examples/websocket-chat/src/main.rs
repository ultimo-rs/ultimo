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

        // Subscribe to chat room
        if let Err(e) = ws.subscribe(self.room).await {
            tracing::error!("Failed to subscribe: {}", e);
            return;
        }

        // Send welcome message
        ws.send("Welcome to the chat room!").await.ok();

        // Notify all users in the room (including this one)
        let join_msg = json!({"type": "join", "message": "A user joined the room"});
        ws.publish(self.room, &join_msg).await.ok();
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
            Message::Close(frame) => {
                if let Some(cf) = frame {
                    tracing::info!("Client closing: {} - {}", cf.code, cf.reason);
                }
            }
            _ => {}
        }
    }

    async fn on_close(&self, ws: &WebSocket<Self::Data>, code: u16, reason: &str) {
        tracing::info!("Client disconnected: {} - {}", code, reason);

        // Notify others
        let leave_msg = json!({"type": "leave", "message": "A user left the room"});
        ws.publish(self.room, &leave_msg).await.ok();
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
        Ok(ultimo::response::helpers::html(include_str!(
            "../index.html"
        ))?)
    });

    // WebSocket endpoint with custom configuration (Phase 2 features)
    let config = WebSocketConfig {
        // Limit message sizes for chat
        max_message_size: 10 * 1024 * 1024, // 10 MB
        max_frame_size: 1 * 1024 * 1024,    // 1 MB (enables automatic fragmentation)

        // Ping/pong heartbeat to detect dead connections
        ping_interval: Some(30), // Send ping every 30 seconds
        ping_timeout: 10,        // Disconnect if no pong after 10 seconds

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

    app.listen("127.0.0.1:4000").await
}
