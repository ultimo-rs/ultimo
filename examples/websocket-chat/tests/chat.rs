//! Integration tests for the WebSocket Chat example.
//!
//! These tests demonstrate how to test WebSocket applications built with Ultimo.
//! They use `tokio-tungstenite` to connect real WebSocket clients to the server.
//!
//! Run with: `cargo test -p websocket-chat`

use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Start the chat server on a given port and wait for it to be ready.
async fn start_server(port: u16) {
    use async_trait::async_trait;
    use ultimo::prelude::*;
    use ultimo::websocket::{WebSocket, WebSocketConfig, WebSocketHandler};

    #[derive(Clone)]
    struct ChatHandler;

    #[async_trait]
    impl WebSocketHandler for ChatHandler {
        type Data = ();

        async fn on_open(&self, ws: &WebSocket<Self::Data>) {
            ws.send("Welcome to the chat room!").await.ok();
            ws.subscribe("lobby").await.ok();
        }

        async fn on_message(&self, ws: &WebSocket<Self::Data>, msg: ultimo::websocket::Message) {
            if let ultimo::websocket::Message::Text(text) = msg {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                    ws.publish("lobby", &data).await.ok();
                } else {
                    ws.send(&text).await.ok();
                }
            }
        }

        async fn on_close(&self, _ws: &WebSocket<Self::Data>, _code: u16, _reason: &str) {}
    }

    tokio::spawn(async move {
        let mut app = Ultimo::new();
        let config = WebSocketConfig {
            ping_interval: None,
            ..Default::default()
        };
        app.websocket_with_config("/ws", ChatHandler, config);
        app.listen(&format!("127.0.0.1:{port}")).await.ok();
    });

    // Wait for server to be ready
    tokio::time::sleep(Duration::from_millis(100)).await;
}

/// Helper: read the next text message, skipping ping/pong frames.
async fn next_text(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> String {
    let timeout = Duration::from_secs(5);
    loop {
        match tokio::time::timeout(timeout, ws.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => return text.to_string(),
            Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => continue,
            Ok(Some(Ok(other))) => panic!("Unexpected message type: {other:?}"),
            Ok(Some(Err(e))) => panic!("WebSocket error: {e}"),
            Ok(None) => panic!("Connection closed unexpectedly"),
            Err(_) => panic!("Timed out waiting for message"),
        }
    }
}

/// Find a free port for testing.
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn connects_and_receives_welcome() {
    let port = free_port();
    start_server(port).await;

    let (mut ws, _) = connect_async(format!("ws://127.0.0.1:{port}/ws"))
        .await
        .expect("Failed to connect");

    let msg = next_text(&mut ws).await;
    assert_eq!(msg, "Welcome to the chat room!");
}

#[tokio::test]
async fn echoes_plain_text() {
    let port = free_port();
    start_server(port).await;

    let (mut ws, _) = connect_async(format!("ws://127.0.0.1:{port}/ws"))
        .await
        .unwrap();

    // Consume welcome
    next_text(&mut ws).await;

    // Send plain text (not JSON) — should echo back as-is
    ws.send(Message::Text("hello".into())).await.unwrap();
    let reply = next_text(&mut ws).await;
    assert_eq!(reply, "hello");
}

#[tokio::test]
async fn broadcasts_json_message_to_sender() {
    let port = free_port();
    start_server(port).await;

    let (mut ws, _) = connect_async(format!("ws://127.0.0.1:{port}/ws"))
        .await
        .unwrap();

    // Consume welcome
    next_text(&mut ws).await;

    // Send a JSON chat message
    let msg = serde_json::json!({
        "type": "message",
        "message": "hi there",
        "sender": "test-client"
    });
    ws.send(Message::Text(msg.to_string().into()))
        .await
        .unwrap();

    // Should receive it back (published to lobby, we're subscribed)
    let reply = next_text(&mut ws).await;
    let data: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(data["message"], "hi there");
    assert_eq!(data["sender"], "test-client");
}

#[tokio::test]
async fn multi_client_broadcast() {
    let port = free_port();
    start_server(port).await;

    // Client A connects
    let (mut ws_a, _) = connect_async(format!("ws://127.0.0.1:{port}/ws"))
        .await
        .unwrap();
    next_text(&mut ws_a).await; // welcome

    // Client B connects
    let (mut ws_b, _) = connect_async(format!("ws://127.0.0.1:{port}/ws"))
        .await
        .unwrap();
    next_text(&mut ws_b).await; // welcome

    // Client A sends a message
    let msg = serde_json::json!({
        "type": "message",
        "message": "from A",
        "sender": "client-a"
    });
    ws_a.send(Message::Text(msg.to_string().into()))
        .await
        .unwrap();

    // Both A and B should receive it
    let reply_a = next_text(&mut ws_a).await;
    let reply_b = next_text(&mut ws_b).await;

    let data_a: serde_json::Value = serde_json::from_str(&reply_a).unwrap();
    let data_b: serde_json::Value = serde_json::from_str(&reply_b).unwrap();

    assert_eq!(data_a["message"], "from A");
    assert_eq!(data_b["message"], "from A");
    assert_eq!(data_b["sender"], "client-a");
}

#[tokio::test]
async fn multiple_messages_in_sequence() {
    let port = free_port();
    start_server(port).await;

    let (mut ws, _) = connect_async(format!("ws://127.0.0.1:{port}/ws"))
        .await
        .unwrap();
    next_text(&mut ws).await; // welcome

    // Send multiple messages rapidly
    for i in 1..=5 {
        let msg = serde_json::json!({
            "type": "message",
            "message": format!("msg-{i}"),
            "sender": "rapid"
        });
        ws.send(Message::Text(msg.to_string().into()))
            .await
            .unwrap();
    }

    // Should receive all 5 back in order
    for i in 1..=5 {
        let reply = next_text(&mut ws).await;
        let data: serde_json::Value = serde_json::from_str(&reply).unwrap();
        assert_eq!(data["message"], format!("msg-{i}"));
    }
}
