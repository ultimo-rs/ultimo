//! Tests for WebSocket ping/pong heartbeat functionality

#[cfg(feature = "websocket")]
mod websocket_ping_pong_tests {
    use bytes::Bytes;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;
    use ultimo::websocket::test_helpers::*;
    use ultimo::websocket::{ChannelManager, WebSocketConfig};

    fn default_config() -> Arc<WebSocketConfig> {
        Arc::new(WebSocketConfig::default())
    }

    #[tokio::test]
    async fn test_config_with_ping_enabled() {
        let config = WebSocketConfig {
            ping_interval: Some(1), // 1 second
            ping_timeout: 5,
            ..Default::default()
        };

        assert_eq!(config.ping_interval, Some(1));
        assert_eq!(config.ping_timeout, 5);
    }

    #[tokio::test]
    async fn test_config_with_ping_disabled() {
        let config = WebSocketConfig {
            ping_interval: None,
            ..Default::default()
        };

        assert_eq!(config.ping_interval, None);
    }

    #[tokio::test]
    async fn test_ping_frame_creation() {
        let ping = Frame::ping(Bytes::from("test"));
        assert_eq!(ping.opcode, OpCode::Ping);
        assert!(ping.fin);
        assert_eq!(ping.payload, Bytes::from("test"));
    }

    #[tokio::test]
    async fn test_pong_frame_creation() {
        let pong = Frame::pong(Bytes::from("test"));
        assert_eq!(pong.opcode, OpCode::Pong);
        assert!(pong.fin);
        assert_eq!(pong.payload, Bytes::from("test"));
    }

    #[tokio::test]
    async fn test_ping_pong_frame_encoding() {
        // Test ping frame encoding
        let ping = Frame::ping(Bytes::from("hello"));
        let encoded = ping.encode();
        let mut buf = bytes::BytesMut::from(encoded.as_ref());
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();

        assert_eq!(decoded.opcode, OpCode::Ping);
        assert_eq!(decoded.payload, Bytes::from("hello"));

        // Test pong frame encoding
        let pong = Frame::pong(Bytes::from("world"));
        let encoded = pong.encode();
        let mut buf = bytes::BytesMut::from(encoded.as_ref());
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();

        assert_eq!(decoded.opcode, OpCode::Pong);
        assert_eq!(decoded.payload, Bytes::from("world"));
    }

    #[tokio::test]
    async fn test_ping_pong_empty_payload() {
        // Pings can have empty payloads
        let ping = Frame::ping(Bytes::new());
        assert_eq!(ping.payload.len(), 0);

        let pong = Frame::pong(Bytes::new());
        assert_eq!(pong.payload.len(), 0);
    }

    #[tokio::test]
    async fn test_ping_message_type() {
        let message = Message::Ping(Bytes::from("ping data"));
        let frame = message.to_frame();

        assert_eq!(frame.opcode, OpCode::Ping);
        assert!(frame.fin);
        assert_eq!(frame.payload, Bytes::from("ping data"));
    }

    #[tokio::test]
    async fn test_pong_message_type() {
        let message = Message::Pong(Bytes::from("pong data"));
        let frame = message.to_frame();

        assert_eq!(frame.opcode, OpCode::Pong);
        assert!(frame.fin);
        assert_eq!(frame.payload, Bytes::from("pong data"));
    }

    #[tokio::test]
    async fn test_ping_frame_from_message() {
        let ping_frame = Frame {
            fin: true,
            opcode: OpCode::Ping,
            mask: None,
            payload: Bytes::from("test"),
        };

        let message = Message::from_frame(ping_frame).unwrap();
        match message {
            Message::Ping(data) => assert_eq!(data, Bytes::from("test")),
            _ => panic!("Expected Ping message"),
        }
    }

    #[tokio::test]
    async fn test_pong_frame_from_message() {
        let pong_frame = Frame {
            fin: true,
            opcode: OpCode::Pong,
            mask: None,
            payload: Bytes::from("response"),
        };

        let message = Message::from_frame(pong_frame).unwrap();
        match message {
            Message::Pong(data) => assert_eq!(data, Bytes::from("response")),
            _ => panic!("Expected Pong message"),
        }
    }

    #[tokio::test]
    async fn test_ping_interval_configuration() {
        // Test various ping interval configurations
        let configs = vec![
            Some(1),   // 1 second
            Some(5),   // 5 seconds
            Some(30),  // 30 seconds (default)
            Some(60),  // 1 minute
            Some(120), // 2 minutes
            None,      // Disabled
        ];

        for interval in configs {
            let config = WebSocketConfig {
                ping_interval: interval,
                ..Default::default()
            };
            assert_eq!(config.ping_interval, interval);
        }
    }

    #[tokio::test]
    async fn test_ping_timeout_configuration() {
        let config = WebSocketConfig {
            ping_interval: Some(10),
            ping_timeout: 5,
            ..Default::default()
        };

        // Timeout should be less than interval
        assert!(config.ping_timeout < config.ping_interval.unwrap());
    }

    #[tokio::test]
    async fn test_control_frames_not_fragmented() {
        // Ping and Pong should never be fragmented, even with large payloads
        let large_payload = Bytes::from(vec![0u8; 1000]);

        let ping = Message::Ping(large_payload.clone());
        let frames = ping.to_fragmented_frames(100); // Try to fragment with small chunk
        assert_eq!(frames.len(), 1); // Should still be one frame
        assert_eq!(frames[0].opcode, OpCode::Ping);

        let pong = Message::Pong(large_payload);
        let frames = pong.to_fragmented_frames(100);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].opcode, OpCode::Pong);
    }

    #[tokio::test]
    async fn test_websocket_with_ping_config() {
        let channel_manager = Arc::new(ChannelManager::new());
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let config = Arc::new(WebSocketConfig {
            ping_interval: Some(2), // 2 seconds
            ping_timeout: 5,
            ..Default::default()
        });

        let ws = create_websocket(
            (),
            tx,
            channel_manager,
            uuid::Uuid::new_v4(),
            None,
            config.clone(),
        );

        // Verify config is accessible
        assert_eq!(ws.config().ping_interval, Some(2));
        assert_eq!(ws.config().ping_timeout, 5);
    }
}
