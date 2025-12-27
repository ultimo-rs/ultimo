use ultimo::prelude::*;
use ultimo::websocket::{Message, WebSocket, WebSocketHandler};

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

        // Send welcome message
        if let Err(e) = ws.send("Welcome to WebSocket Chat!").await {
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create Ultimo app
    let mut app = Ultimo::new();

    // Add WebSocket handler
    app.websocket("/ws", ChatHandler);

    // Add a simple health check endpoint
    app.get("/health", |ctx: Context| async move {
        ctx.json(json!({ "status": "OK" })).await
    });

    println!("🚀 WebSocket Chat server running on http://localhost:4000");
    println!("📡 WebSocket endpoint: ws://localhost:4000/ws");
    println!("💬 Open React app at http://localhost:5173");

    // Start server
    app.listen("127.0.0.1:4000").await?;

    Ok(())
}
