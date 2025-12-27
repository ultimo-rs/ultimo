//! Integration tests for WebSocket functionality

#[cfg(feature = "websocket")]
mod websocket_tests {
    use bytes::Bytes;
    use ultimo::websocket::test_helpers::*;
    use ultimo::websocket::{ChannelManager, Message};

    #[tokio::test]
    async fn test_websocket_send_receive() {
        let channel_manager = std::sync::Arc::new(ChannelManager::new());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let ws: WebSocket<()> =
            create_websocket((), tx, channel_manager, uuid::Uuid::new_v4(), None);

        // Send text message
        ws.send("Hello").await.unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            Message::Text(text) => assert_eq!(text, "Hello"),
            _ => panic!("Expected text message"),
        }

        // Send binary message
        ws.send_binary(vec![1, 2, 3]).await.unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            Message::Binary(data) => assert_eq!(data, Bytes::from(vec![1, 2, 3])),
            _ => panic!("Expected binary message"),
        }
    }

    #[tokio::test]
    async fn test_websocket_json() {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestMessage {
            text: String,
            count: u32,
        }

        let channel_manager = std::sync::Arc::new(ChannelManager::new());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let ws: WebSocket<()> =
            create_websocket((), tx, channel_manager, uuid::Uuid::new_v4(), None);

        let test_msg = TestMessage {
            text: "test".to_string(),
            count: 42,
        };

        ws.send_json(&test_msg).await.unwrap();
        let msg = rx.recv().await.unwrap();

        match msg {
            Message::Text(json) => {
                let decoded: TestMessage = serde_json::from_str(&json).unwrap();
                assert_eq!(decoded, test_msg);
            }
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_websocket_typed_data() {
        #[derive(Clone)]
        struct UserData {
            user_id: String,
            room: String,
        }

        let channel_manager = std::sync::Arc::new(ChannelManager::new());
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let user_data = UserData {
            user_id: "user123".to_string(),
            room: "general".to_string(),
        };

        let ws = create_websocket(user_data, tx, channel_manager, uuid::Uuid::new_v4(), None);

        assert_eq!(ws.data().user_id, "user123");
        assert_eq!(ws.data().room, "general");
    }

    #[tokio::test]
    async fn test_pubsub_multiple_subscribers() {
        let channel_manager = std::sync::Arc::new(ChannelManager::new());

        let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();
        let (tx3, mut rx3) = tokio::sync::mpsc::unbounded_channel();

        let ws1: WebSocket<()> =
            create_websocket((), tx1, channel_manager.clone(), uuid::Uuid::new_v4(), None);

        let ws2: WebSocket<()> =
            create_websocket((), tx2, channel_manager.clone(), uuid::Uuid::new_v4(), None);

        let ws3: WebSocket<()> =
            create_websocket((), tx3, channel_manager.clone(), uuid::Uuid::new_v4(), None);

        // Subscribe all to same topic
        ws1.subscribe("chat:lobby").await.unwrap();
        ws2.subscribe("chat:lobby").await.unwrap();
        ws3.subscribe("chat:lobby").await.unwrap();

        assert_eq!(channel_manager.subscriber_count("chat:lobby").await, 3);

        // Publish message
        #[derive(serde::Serialize)]
        struct ChatMsg {
            text: String,
        }

        ws1.publish(
            "chat:lobby",
            &ChatMsg {
                text: "Hello everyone!".to_string(),
            },
        )
        .await
        .unwrap();

        // All subscribers should receive
        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();
        let msg3 = rx3.recv().await.unwrap();

        for msg in [msg1, msg2, msg3] {
            match msg {
                Message::Text(json) => {
                    assert!(json.contains("Hello everyone!"));
                }
                _ => panic!("Expected text message"),
            }
        }
    }

    #[tokio::test]
    async fn test_pubsub_unsubscribe() {
        let channel_manager = std::sync::Arc::new(ChannelManager::new());
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let ws: WebSocket<()> =
            create_websocket((), tx, channel_manager.clone(), uuid::Uuid::new_v4(), None);

        ws.subscribe("topic1").await.unwrap();
        ws.subscribe("topic2").await.unwrap();

        assert_eq!(channel_manager.subscriber_count("topic1").await, 1);
        assert_eq!(channel_manager.subscriber_count("topic2").await, 1);

        ws.unsubscribe("topic1").await.unwrap();

        assert_eq!(channel_manager.subscriber_count("topic1").await, 0);
        assert_eq!(channel_manager.subscriber_count("topic2").await, 1);
    }

    #[tokio::test]
    async fn test_pubsub_isolated_topics() {
        let channel_manager = std::sync::Arc::new(ChannelManager::new());

        let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();

        let ws1: WebSocket<()> =
            create_websocket((), tx1, channel_manager.clone(), uuid::Uuid::new_v4(), None);

        let ws2: WebSocket<()> =
            create_websocket((), tx2, channel_manager.clone(), uuid::Uuid::new_v4(), None);

        ws1.subscribe("room:A").await.unwrap();
        ws2.subscribe("room:B").await.unwrap();

        // Publish to room A
        ws1.publish("room:A", &serde_json::json!({"msg": "Room A"}))
            .await
            .unwrap();

        // Only ws1 should receive
        let msg1 = rx1.recv().await;
        assert!(msg1.is_some());

        // ws2 should not receive anything
        tokio::select! {
            _ = rx2.recv() => panic!("ws2 should not receive room:A messages"),
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(50)) => {}
        }
    }

    #[tokio::test]
    async fn test_connection_count() {
        let channel_manager = std::sync::Arc::new(ChannelManager::new());

        assert_eq!(channel_manager.connection_count().await, 0);

        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();

        let id1 = uuid::Uuid::new_v4();
        let id2 = uuid::Uuid::new_v4();

        channel_manager.subscribe(id1, "test", tx1).await.unwrap();
        channel_manager.subscribe(id2, "test", tx2).await.unwrap();

        assert_eq!(channel_manager.connection_count().await, 2);

        channel_manager.disconnect(id1).await;
        assert_eq!(channel_manager.connection_count().await, 1);

        channel_manager.disconnect(id2).await;
        assert_eq!(channel_manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_websocket_close() {
        let channel_manager = std::sync::Arc::new(ChannelManager::new());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let ws: WebSocket<()> =
            create_websocket((), tx, channel_manager, uuid::Uuid::new_v4(), None);

        ws.close(Some(1000), Some("Normal closure")).await.unwrap();

        let msg = rx.recv().await.unwrap();
        match msg {
            Message::Close(Some(close_frame)) => {
                assert_eq!(close_frame.code, 1000);
                assert_eq!(close_frame.reason, "Normal closure");
            }
            _ => panic!("Expected close message"),
        }
    }

    #[tokio::test]
    async fn test_websocket_is_writable() {
        let channel_manager = std::sync::Arc::new(ChannelManager::new());
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let ws: WebSocket<()> =
            create_websocket((), tx.clone(), channel_manager, uuid::Uuid::new_v4(), None);

        assert!(ws.is_writable());

        // Drop both tx and rx to close the channel
        drop(tx);
        drop(rx);

        // Give it a moment for the channel to register as closed
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        assert!(!ws.is_writable());
    }
}
