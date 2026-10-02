//! Graceful HTTP shutdown also tells connected WebSocket clients to go away.

#![cfg(feature = "websocket")]

use futures_util::StreamExt;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message as ClientMessage;
use ultimo::prelude::*;
use ultimo::websocket::{Message, WebSocket, WebSocketHandler};

#[derive(Clone)]
struct Quiet;

#[async_trait::async_trait]
impl WebSocketHandler for Quiet {
    type Data = ();

    async fn on_open(&self, ws: &WebSocket<Self::Data>) {
        ws.send("hello").await.ok();
    }

    async fn on_message(&self, _ws: &WebSocket<Self::Data>, _msg: Message) {}
}

#[tokio::test]
async fn shutdown_sends_1001_close_frame_to_websocket_clients() {
    let mut app = Ultimo::new_without_defaults();
    app.shutdown_grace_period(Duration::from_secs(1));
    app.websocket("/ws", Quiet);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(app.serve_with_shutdown(listener, async {
        rx.await.ok();
    }));

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .expect("connect");
    // Wait for the greeting so the connection is registered server-side.
    assert!(matches!(
        ws.next().await,
        Some(Ok(ClientMessage::Text(t))) if t == "hello"
    ));

    tx.send(()).unwrap();

    let close = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(msg) = ws.next().await {
            if let Ok(ClientMessage::Close(frame)) = msg {
                return frame;
            }
        }
        None
    })
    .await
    .expect("client should receive a close frame on shutdown");
    assert_eq!(u16::from(close.expect("close frame payload").code), 1001);

    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("server exits within the grace period")
        .unwrap()
        .unwrap();
}
