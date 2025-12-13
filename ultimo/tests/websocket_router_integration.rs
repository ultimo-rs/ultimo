//! Router integration tests for WebSocket
//!
//! Tests the integration between Ultimo's router and WebSocket functionality

use tokio::net::TcpListener;
use ultimo::prelude::*;
use ultimo::websocket::{Message, WebSocket, WebSocketHandler};

#[derive(Clone)]
struct TestHandler;

#[async_trait::async_trait]
impl WebSocketHandler for TestHandler {
    type Data = ();

    async fn on_open(&self, ws: &WebSocket<Self::Data>) {
        ws.send("connected").await.ok();
    }

    async fn on_message(&self, ws: &WebSocket<Self::Data>, msg: Message) {
        if let Message::Text(text) = msg {
            ws.send(format!("echo: {}", text)).await.ok();
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct TypedHandler;

#[async_trait::async_trait]
impl WebSocketHandler for TypedHandler {
    type Data = String;

    async fn on_open(&self, ws: &WebSocket<Self::Data>) {
        let user = ws.data();
        ws.send(format!("Welcome, {}!", user)).await.ok();
    }

    async fn on_message(&self, ws: &WebSocket<Self::Data>, msg: Message) {
        if let Message::Text(text) = msg {
            let user = ws.data();
            ws.send(format!("{}: {}", user, text)).await.ok();
        }
    }
}

/// Helper to find an available port
async fn find_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Helper to start a test server
async fn start_test_server(port: u16) {
    let mut app = Ultimo::new();

    // Add WebSocket route
    app.websocket("/ws", TestHandler);

    // Add regular HTTP route
    app.get("/health", |ctx: Context| async move {
        ctx.json(json!({"status": "ok"})).await
    });

    tokio::spawn(async move {
        let addr = format!("127.0.0.1:{}", port);
        app.listen(&addr).await.ok();
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_websocket_route_registration() {
    let port = find_available_port().await;
    start_test_server(port).await;

    // Connect to WebSocket
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/ws", port))
        .await
        .expect("Failed to connect");

    // Should receive connection message
    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

    if let Some(Ok(TungsteniteMessage::Text(text))) = ws.next().await {
        assert_eq!(text, "connected");
    } else {
        panic!("Expected connection message");
    }
}

#[tokio::test]
async fn test_websocket_echo() {
    let port = find_available_port().await;
    start_test_server(port).await;

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/ws", port))
        .await
        .expect("Failed to connect");

    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

    // Skip connection message
    ws.next().await;

    // Send message
    ws.send(TungsteniteMessage::Text("hello".to_string()))
        .await
        .unwrap();

    // Receive echo
    if let Some(Ok(TungsteniteMessage::Text(text))) = ws.next().await {
        assert_eq!(text, "echo: hello");
    } else {
        panic!("Expected echo response");
    }
}

#[tokio::test]
async fn test_multiple_websocket_connections() {
    let port = find_available_port().await;
    start_test_server(port).await;

    // Connect multiple clients
    let (mut ws1, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/ws", port))
        .await
        .expect("Failed to connect client 1");

    let (mut ws2, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/ws", port))
        .await
        .expect("Failed to connect client 2");

    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

    // Both should receive connection messages
    if let Some(Ok(TungsteniteMessage::Text(text))) = ws1.next().await {
        assert_eq!(text, "connected");
    }

    if let Some(Ok(TungsteniteMessage::Text(text))) = ws2.next().await {
        assert_eq!(text, "connected");
    }
}

#[tokio::test]
async fn test_http_and_websocket_coexist() {
    let port = find_available_port().await;
    start_test_server(port).await;

    // Test HTTP endpoint
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Failed to make HTTP request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // Test WebSocket endpoint
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/ws", port))
        .await
        .expect("Failed to connect to WebSocket");

    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

    if let Some(Ok(TungsteniteMessage::Text(text))) = ws.next().await {
        assert_eq!(text, "connected");
    }
}

#[tokio::test]
async fn test_websocket_binary_messages() {
    let port = find_available_port().await;
    start_test_server(port).await;

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/ws", port))
        .await
        .expect("Failed to connect");

    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

    // Skip connection message
    ws.next().await;

    // Send binary data
    let data = vec![1, 2, 3, 4, 5];
    ws.send(TungsteniteMessage::Binary(data.clone()))
        .await
        .unwrap();

    // Binary messages won't be echoed by our test handler, but connection should remain stable
    // Send a text message to verify connection is still alive
    ws.send(TungsteniteMessage::Text("ping".to_string()))
        .await
        .unwrap();

    if let Some(Ok(TungsteniteMessage::Text(text))) = ws.next().await {
        assert_eq!(text, "echo: ping");
    }
}
