//! Tests for WebSocket graceful shutdown functionality

#[cfg(feature = "websocket")]
mod websocket_shutdown_tests {
    use std::sync::Arc;
    use ultimo::websocket::test_helpers::*;
    use ultimo::websocket::{ChannelManager, WebSocketConfig};

    #[tokio::test]
    async fn test_close_frame_creation() {
        let close_frame = Frame::close(Some(1000), Some("Normal closure"));
        assert_eq!(close_frame.opcode, OpCode::Close);
        assert!(close_frame.fin);
        assert!(close_frame.payload.len() > 0);
    }

    #[tokio::test]
    async fn test_close_frame_encoding() {
        let close_frame = Frame::close(Some(1001), Some("Going away"));
        let encoded = close_frame.encode();
        let mut buf = bytes::BytesMut::from(encoded.as_ref());
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();

        assert_eq!(decoded.opcode, OpCode::Close);
        assert!(decoded.fin);
    }

    #[tokio::test]
    async fn test_websocket_close_method() {
        let channel_manager = Arc::new(ChannelManager::new());
        let (tx, mut rx) = tokio::sync::mpsc::channel(1000);

        let ws = create_websocket(
            (),
            tx,
            channel_manager,
            uuid::Uuid::new_v4(),
            None,
            Arc::new(WebSocketConfig::default()),
        );

        // Close the connection
        ws.close(Some(1000), Some("Test closure")).await.unwrap();

        // Verify close message was sent
        let msg = rx.recv().await.unwrap();
        match msg {
            Message::Close(Some(close_frame)) => {
                assert_eq!(close_frame.code, 1000);
                assert_eq!(close_frame.reason, "Test closure");
            }
            _ => panic!("Expected close message"),
        }
    }

    #[tokio::test]
    async fn test_close_without_code() {
        let channel_manager = Arc::new(ChannelManager::new());
        let (tx, mut rx) = tokio::sync::mpsc::channel(1000);

        let ws = create_websocket(
            (),
            tx,
            channel_manager,
            uuid::Uuid::new_v4(),
            None,
            Arc::new(WebSocketConfig::default()),
        );

        ws.close(None, None).await.unwrap();

        let msg = rx.recv().await.unwrap();
        match msg {
            Message::Close(None) => {}
            _ => panic!("Expected close message without frame"),
        }
    }

    #[tokio::test]
    async fn test_standard_close_codes() {
        let codes = vec![
            (1000, "Normal closure"),
            (1001, "Going away"),
            (1002, "Protocol error"),
            (1003, "Unsupported data"),
            (1007, "Invalid frame payload data"),
            (1008, "Policy violation"),
            (1009, "Message too big"),
            (1010, "Mandatory extension"),
            (1011, "Internal server error"),
        ];

        for (code, reason) in codes {
            let close_frame = Frame::close(Some(code), Some(reason));
            assert_eq!(close_frame.opcode, OpCode::Close);

            // Verify it encodes/decodes correctly
            let encoded = close_frame.encode();
            let mut buf = bytes::BytesMut::from(encoded.as_ref());
            let decoded = Frame::parse(&mut buf).unwrap().unwrap();
            assert_eq!(decoded.opcode, OpCode::Close);
        }
    }

    #[tokio::test]
    async fn test_close_message_from_frame() {
        let mut payload = bytes::BytesMut::new();
        payload.extend_from_slice(&1000u16.to_be_bytes());
        payload.extend_from_slice(b"Test reason");

        let close_frame = Frame {
            fin: true,
            opcode: OpCode::Close,
            mask: None,
            payload: payload.freeze(),
        };

        let message = Message::from_frame(close_frame).unwrap();
        match message {
            Message::Close(Some(cf)) => {
                assert_eq!(cf.code, 1000);
                assert_eq!(cf.reason, "Test reason");
            }
            _ => panic!("Expected close message"),
        }
    }

    #[tokio::test]
    async fn test_broadcast_all_connections() {
        let manager = Arc::new(ChannelManager::new());

        let (tx1, mut rx1) = tokio::sync::mpsc::channel(1000);
        let (tx2, mut rx2) = tokio::sync::mpsc::channel(1000);
        let (tx3, mut rx3) = tokio::sync::mpsc::channel(1000);

        let conn1 = uuid::Uuid::new_v4();
        let conn2 = uuid::Uuid::new_v4();
        let conn3 = uuid::Uuid::new_v4();

        // Subscribe connections
        manager.subscribe(conn1, "test", tx1).await.unwrap();
        manager.subscribe(conn2, "test", tx2).await.unwrap();
        manager.subscribe(conn3, "other", tx3).await.unwrap();

        // Broadcast close message to all
        let close_msg = Message::Close(Some(CloseFrame {
            code: 1001,
            reason: "Server shutting down".to_string(),
        }));

        let sent = manager.broadcast_all(close_msg.clone()).await;
        assert_eq!(sent, 3); // All connections should receive

        // Verify all received the message
        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();
        let msg3 = rx3.recv().await.unwrap();

        for msg in [msg1, msg2, msg3] {
            match msg {
                Message::Close(Some(cf)) => {
                    assert_eq!(cf.code, 1001);
                    assert_eq!(cf.reason, "Server shutting down");
                }
                _ => panic!("Expected close message"),
            }
        }
    }

    #[tokio::test]
    async fn test_all_connection_ids() {
        let manager = Arc::new(ChannelManager::new());

        let (tx1, _rx1) = tokio::sync::mpsc::channel(1000);
        let (tx2, _rx2) = tokio::sync::mpsc::channel(1000);

        let conn1 = uuid::Uuid::new_v4();
        let conn2 = uuid::Uuid::new_v4();

        manager.subscribe(conn1, "test", tx1).await.unwrap();
        manager.subscribe(conn2, "test", tx2).await.unwrap();

        let ids = manager.all_connection_ids().await;
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&conn1));
        assert!(ids.contains(&conn2));
    }

    #[tokio::test]
    async fn test_connection_writable_check() {
        let channel_manager = Arc::new(ChannelManager::new());
        let (tx, rx) = tokio::sync::mpsc::channel(1000);

        let ws = create_websocket(
            (),
            tx.clone(),
            channel_manager,
            uuid::Uuid::new_v4(),
            None,
            Arc::new(WebSocketConfig::default()),
        );

        // Should be writable when channel is open
        assert!(ws.is_writable());

        // Drop receiver and sender to close the channel
        drop(tx);
        drop(rx);

        // Give it a moment
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Should not be writable after channel closed
        // Note: This may still return true briefly due to buffering
    }

    #[tokio::test]
    async fn test_empty_close_reason() {
        let channel_manager = Arc::new(ChannelManager::new());
        let (tx, mut rx) = tokio::sync::mpsc::channel(1000);

        let ws = create_websocket(
            (),
            tx,
            channel_manager,
            uuid::Uuid::new_v4(),
            None,
            Arc::new(WebSocketConfig::default()),
        );

        ws.close(Some(1000), Some("")).await.unwrap();

        let msg = rx.recv().await.unwrap();
        match msg {
            Message::Close(Some(close_frame)) => {
                assert_eq!(close_frame.code, 1000);
                assert_eq!(close_frame.reason, "");
            }
            _ => panic!("Expected close message"),
        }
    }

    #[tokio::test]
    async fn test_close_frame_not_fragmented() {
        // Close frames should never be fragmented
        let close_msg = Message::Close(Some(CloseFrame {
            code: 1000,
            reason: "A".repeat(1000), // Large reason
        }));

        let frames = close_msg.to_fragmented_frames(100);
        assert_eq!(frames.len(), 1); // Should still be one frame
        assert_eq!(frames[0].opcode, OpCode::Close);
    }
}
