//! Integration tests for the WebSocket `allowed_origins` allow-list
//! (Cross-Site WebSocket Hijacking defense).

#![cfg(feature = "websocket")]

use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use ultimo::prelude::*;
use ultimo::websocket::{Message, WebSocket, WebSocketConfig, WebSocketHandler};

#[derive(Clone)]
struct EchoHandler;

#[async_trait::async_trait]
impl WebSocketHandler for EchoHandler {
    type Data = ();

    async fn on_open(&self, ws: &WebSocket<Self::Data>) {
        ws.send("connected").await.ok();
    }

    async fn on_message(&self, _ws: &WebSocket<Self::Data>, _msg: Message) {}
}

async fn find_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

async fn start_restricted_server(port: u16) {
    let mut app = Ultimo::new_without_defaults();
    let config = WebSocketConfig {
        ping_interval: None, // avoid heartbeat frames interfering with the test
        ..Default::default()
    };
    app.websocket_with_config_and_origins(
        "/ws",
        EchoHandler,
        config,
        vec!["https://good.example".to_string()],
    );

    tokio::spawn(async move {
        app.listen(&format!("127.0.0.1:{}", port)).await.ok();
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}

fn request_with_origin(
    url: &str,
    origin: Option<&str>,
) -> tokio_tungstenite::tungstenite::http::Request<()> {
    let mut request = url.into_client_request().unwrap();
    if let Some(origin) = origin {
        request
            .headers_mut()
            .insert("Origin", origin.parse().unwrap());
    }
    request
}

#[tokio::test]
async fn disallowed_origin_is_rejected() {
    let port = find_available_port().await;
    start_restricted_server(port).await;

    let req = request_with_origin(
        &format!("ws://127.0.0.1:{}/ws", port),
        Some("https://evil.example"),
    );
    let result = tokio_tungstenite::connect_async(req).await;
    assert!(
        result.is_err(),
        "handshake from a disallowed Origin must be rejected"
    );
}

#[tokio::test]
async fn missing_origin_is_rejected_when_restricted() {
    let port = find_available_port().await;
    start_restricted_server(port).await;

    let req = request_with_origin(&format!("ws://127.0.0.1:{}/ws", port), None);
    let result = tokio_tungstenite::connect_async(req).await;
    assert!(
        result.is_err(),
        "handshake with no Origin header must be rejected once allowed_origins is set"
    );
}

#[tokio::test]
async fn allowed_origin_is_accepted() {
    let port = find_available_port().await;
    start_restricted_server(port).await;

    let req = request_with_origin(
        &format!("ws://127.0.0.1:{}/ws", port),
        Some("https://good.example"),
    );
    let (mut ws, _) = tokio_tungstenite::connect_async(req)
        .await
        .expect("handshake from an allowed Origin must succeed");

    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
    if let Some(Ok(TungsteniteMessage::Text(text))) = ws.next().await {
        assert_eq!(text, "connected");
    } else {
        panic!("expected the connected message");
    }
}

#[tokio::test]
async fn unrestricted_config_allows_any_origin() {
    // Default (empty allowed_origins) preserves pre-existing behavior: no
    // Origin check at all.
    let port = find_available_port().await;
    let mut app = Ultimo::new_without_defaults();
    app.websocket("/ws", EchoHandler);
    tokio::spawn(async move {
        app.listen(&format!("127.0.0.1:{}", port)).await.ok();
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let req = request_with_origin(
        &format!("ws://127.0.0.1:{}/ws", port),
        Some("https://anything.example"),
    );
    let result = tokio_tungstenite::connect_async(req).await;
    assert!(
        result.is_ok(),
        "no allow-list configured means no restriction"
    );
}
