use ultimo::prelude::*;
use ultimo::websocket::{Message, WebSocket, WebSocketConfig, WebSocketHandler};

#[derive(Clone)]
struct ChatHandler;

#[async_trait::async_trait]
impl WebSocketHandler for ChatHandler {
    type Data = ();

    async fn on_open(&self, ws: &WebSocket<Self::Data>) {
        println!("Client connected");

        // Subscribe to chat channel for broadcasting
        if let Err(e) = ws.subscribe("chat").await {
            eprintln!("Error subscribing to chat: {}", e);
            return;
        }

        // Send welcome message with connection info
        let welcome = json!({
            "type": "system",
            "message": "Welcome to WebSocket Chat!",
            "features": {
                "ping_pong": "Automatic heartbeat enabled",
                "fragmentation": "Large messages auto-fragmented",
                "backpressure": "Flow control active"
            }
        });

        if let Err(e) = ws.send_json(&welcome).await {
            eprintln!("Error sending welcome message: {}", e);
        }
    }

    async fn on_message(&self, ws: &WebSocket<Self::Data>, message: Message) {
        match message {
            Message::Text(text) => {
                println!("Received message: {}", text);

                // Broadcast to all subscribers (including sender)
                if let Err(e) = ws.publish("chat", &text).await {
                    eprintln!("Error publishing message: {}", e);
                }
            }
            Message::Binary(data) => {
                println!("Received binary data: {} bytes", data.len());
            }
            Message::Close(close_frame) => {
                if let Some(frame) = close_frame {
                    println!(
                        "Received close message (code: {}, reason: {})",
                        frame.code, frame.reason
                    );
                } else {
                    println!("Received close message");
                }
            }
            _ => {}
        }
    }

    async fn on_drain(&self, ws: &WebSocket<Self::Data>) {
        // Backpressure relief - buffer has drained
        println!(
            "Connection buffer drained. Available capacity: {}/{}",
            ws.capacity(),
            ws.max_capacity()
        );
    }

    async fn on_close(&self, _ws: &WebSocket<Self::Data>, code: u16, reason: &str) {
        println!("Client disconnected: {} - {}", code, reason);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create Ultimo app
    let mut app = Ultimo::new();

    // Configure WebSocket with Phase 2 features
    let ws_config = WebSocketConfig {
        // Enable automatic ping/pong heartbeat
        ping_interval: Some(30), // Ping every 30 seconds
        ping_timeout: 10,        // Timeout after 10 seconds

        // Configure message size limits (enables fragmentation for large messages)
        max_message_size: 5 * 1024 * 1024, // 5 MB total message
        max_frame_size: 512 * 1024,        // 512 KB per frame

        // Backpressure control
        max_write_queue_size: 50, // Buffer up to 50 messages

        ..Default::default()
    };

    // Add WebSocket handler with custom config
    app.websocket_with_config("/ws", ChatHandler, ws_config);

    // Add a simple health check endpoint
    app.get("/health", |ctx: Context| async move {
        ctx.json(json!({ "status": "OK" })).await
    });

    println!("🚀 WebSocket Chat server running on http://localhost:4000");
    println!("📡 WebSocket endpoint: ws://localhost:4000/ws");
    println!("💬 Open React app at http://localhost:5173");
    println!();
    println!("✨ Phase 2 Features Enabled:");
    println!("   🔄 Automatic ping/pong (30s interval, 10s timeout)");
    println!("   📦 Message fragmentation (chunks > 512KB)");
    println!("   ⚡ Backpressure handling (50 message buffer)");
    println!("   🛡️  Connection limits enforced");

    // Start server
    app.listen("127.0.0.1:4000").await?;

    Ok(())
}
