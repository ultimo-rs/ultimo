//! Tests for WebSocket backpressure handling

#[cfg(feature = "websocket")]
mod websocket_backpressure_tests {
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use tokio::time;
    use ultimo::websocket::test_helpers::*;

    #[tokio::test]
    async fn test_send_blocks_when_buffer_full() {
        let (tx, mut rx) = mpsc::channel(2); // Small buffer
        let channel_manager = Arc::new(ChannelManager::new());
        let conn_id = uuid::Uuid::new_v4();
        let config = Arc::new(ultimo::websocket::WebSocketConfig {
            max_write_queue_size: 2,
            ..Default::default()
        });

        let ws = create_websocket(
            (),
            tx.clone(),
            channel_manager,
            conn_id,
            None,
            config,
        );

        // Fill the buffer
        assert!(ws.send("message1").await.is_ok());
        assert!(ws.send("message2").await.is_ok());

        // Next send should fail with WouldBlock
        let result = ws.send("message3").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);

        // Drain one message
        let _ = rx.recv().await;

        // Now we should be able to send again
        assert!(ws.send("message4").await.is_ok());
    }

    #[tokio::test]
    async fn test_buffer_capacity_tracking() {
        let (tx, _rx) = mpsc::channel(10);
        let channel_manager = Arc::new(ChannelManager::new());
        let conn_id = uuid::Uuid::new_v4();
        let config = Arc::new(ultimo::websocket::WebSocketConfig {
            max_write_queue_size: 10,
            ..Default::default()
        });

        let ws = create_websocket(
            (),
            tx,
            channel_manager,
            conn_id,
            None,
            config,
        );

        // Check initial capacity
        assert_eq!(ws.max_capacity(), 10);
        assert!(ws.has_capacity());

        // Send messages
        for i in 0..5 {
            ws.send(format!("message{}", i)).await.unwrap();
        }

        // Should still have capacity
        assert!(ws.has_capacity());
        assert!(ws.capacity() > 0);
    }

    #[tokio::test]
    async fn test_binary_send_respects_backpressure() {
        let (tx, mut rx) = mpsc::channel(1);
        let channel_manager = Arc::new(ChannelManager::new());
        let conn_id = uuid::Uuid::new_v4();
        let config = Arc::new(ultimo::websocket::WebSocketConfig {
            max_write_queue_size: 1,
            ..Default::default()
        });

        let ws = create_websocket(
            (),
            tx,
            channel_manager,
            conn_id,
            None,
            config,
        );

        // Fill buffer with binary message
        assert!(ws.send_binary(vec![1, 2, 3, 4]).await.is_ok());

        // Next send should fail
        let result = ws.send_binary(vec![5, 6, 7, 8]).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);

        // Drain
        let _ = rx.recv().await;

        // Should work now
        assert!(ws.send_binary(vec![9, 10]).await.is_ok());
    }

    #[tokio::test]
    async fn test_publish_skips_backpressured_connections() {
        let manager = ChannelManager::new();
        
        // Connection 1: normal buffer
        let (tx1, mut rx1) = mpsc::channel(10);
        let conn1 = uuid::Uuid::new_v4();
        manager.subscribe(conn1, "topic", tx1).await.unwrap();

        // Connection 2: full buffer (size 1)
        let (tx2, _rx2) = mpsc::channel(1);
        let conn2 = uuid::Uuid::new_v4();
        
        // Fill conn2's buffer
        tx2.try_send(Message::Text("blocking".to_string())).unwrap();
        manager.subscribe(conn2, "topic", tx2).await.unwrap();

        // Publish a message
        let sent = manager
            .publish("topic", Message::Text("test".to_string()))
            .await
            .unwrap();

        // Only conn1 should receive it (conn2 is backpressured)
        assert_eq!(sent, 1);

        // Verify conn1 received it
        let msg = time::timeout(Duration::from_millis(100), rx1.recv())
            .await
            .unwrap()
            .unwrap();
        match msg {
            Message::Text(t) => assert_eq!(t, "test"),
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_broadcast_all_respects_backpressure() {
        let manager = ChannelManager::new();

        // Add connections with different buffer states
        let (tx1, mut rx1) = mpsc::channel(10);
        let conn1 = uuid::Uuid::new_v4();
        manager.subscribe(conn1, "topic1", tx1.clone()).await.unwrap();

        let (tx2, _rx2) = mpsc::channel(1);
        let conn2 = uuid::Uuid::new_v4();
        // Fill tx2's buffer
        tx2.try_send(Message::Text("blocking".to_string())).unwrap();
        manager.subscribe(conn2, "topic2", tx2).await.unwrap();

        // Broadcast to all
        let count = manager.broadcast_all(Message::Text("broadcast".to_string())).await;

        // Only conn1 should receive (conn2 backpressured)
        assert_eq!(count, 1);

        // Verify rx1 received it
        let msg = rx1.recv().await.unwrap();
        match msg {
            Message::Text(t) => assert_eq!(t, "broadcast"),
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_close_respects_backpressure() {
        let (tx, mut rx) = mpsc::channel(1);
        let channel_manager = Arc::new(ChannelManager::new());
        let conn_id = uuid::Uuid::new_v4();
        let config = Arc::new(ultimo::websocket::WebSocketConfig {
            max_write_queue_size: 1,
            ..Default::default()
        });

        let ws = create_websocket(
            (),
            tx,
            channel_manager,
            conn_id,
            None,
            config,
        );

        // Fill buffer
        assert!(ws.send("message").await.is_ok());

        // Close should fail when buffer is full
        let result = ws.close(Some(1000), Some("Normal closure")).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);

        // Drain buffer
        let _ = rx.recv().await;

        // Close should work now
        assert!(ws.close(Some(1000), Some("Normal closure")).await.is_ok());
    }

    #[tokio::test]
    async fn test_is_writable_reflects_connection_state() {
        let (tx, rx) = mpsc::channel(10);
        let channel_manager = Arc::new(ChannelManager::new());
        let conn_id = uuid::Uuid::new_v4();
        let config = Arc::new(ultimo::websocket::WebSocketConfig::default());

        let ws = create_websocket(
            (),
            tx.clone(),
            channel_manager,
            conn_id,
            None,
            config,
        );

        // Should be writable initially
        assert!(ws.is_writable());

        // Close the channel
        drop(tx);
        drop(rx);

        // Should no longer be writable
        assert!(!ws.is_writable());
    }

    #[tokio::test]
    async fn test_multiple_send_failures_with_full_buffer() {
        let (tx, mut rx) = mpsc::channel(2);
        let channel_manager = Arc::new(ChannelManager::new());
        let conn_id = uuid::Uuid::new_v4();
        let config = Arc::new(ultimo::websocket::WebSocketConfig {
            max_write_queue_size: 2,
            ..Default::default()
        });

        let ws = create_websocket(
            (),
            tx,
            channel_manager,
            conn_id,
            None,
            config,
        );

        // Fill buffer
        ws.send("msg1").await.unwrap();
        ws.send("msg2").await.unwrap();

        // Multiple failed attempts
        for i in 0..5 {
            let result = ws.send(format!("failed{}", i)).await;
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
        }

        // Drain completely
        let _ = rx.recv().await;
        let _ = rx.recv().await;

        // Should be able to send again
        assert!(ws.send("success").await.is_ok());
    }

    #[tokio::test]
    async fn test_config_max_write_queue_size() {
        // Test with very small queue
        let config1 = ultimo::websocket::WebSocketConfig {
            max_write_queue_size: 1,
            ..Default::default()
        };
        assert_eq!(config1.max_write_queue_size, 1);

        // Test with large queue
        let config2 = ultimo::websocket::WebSocketConfig {
            max_write_queue_size: 10000,
            ..Default::default()
        };
        assert_eq!(config2.max_write_queue_size, 10000);

        // Test default
        let config_default = ultimo::websocket::WebSocketConfig::default();
        assert_eq!(config_default.max_write_queue_size, 1024);
    }

    #[tokio::test]
    async fn test_send_json_respects_backpressure() {
        use serde::Serialize;

        #[derive(Serialize)]
        struct TestData {
            message: String,
            count: u32,
        }

        let (tx, mut rx) = mpsc::channel(1);
        let channel_manager = Arc::new(ChannelManager::new());
        let conn_id = uuid::Uuid::new_v4();
        let config = Arc::new(ultimo::websocket::WebSocketConfig {
            max_write_queue_size: 1,
            ..Default::default()
        });

        let ws = create_websocket(
            (),
            tx,
            channel_manager,
            conn_id,
            None,
            config,
        );

        // Fill buffer
        let data1 = TestData { message: "first".to_string(), count: 1 };
        assert!(ws.send_json(&data1).await.is_ok());

        // Next send should fail
        let data2 = TestData { message: "second".to_string(), count: 2 };
        let result = ws.send_json(&data2).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);

        // Drain
        let _ = rx.recv().await;

        // Should work now
        let data3 = TestData { message: "third".to_string(), count: 3 };
        assert!(ws.send_json(&data3).await.is_ok());
    }
}
