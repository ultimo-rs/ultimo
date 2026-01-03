//! Tests for WebSocket configuration

#[cfg(feature = "websocket")]
mod websocket_config_tests {
    use std::sync::Arc;
    use ultimo::websocket::test_helpers::*;
    use ultimo::websocket::{ChannelManager, WebSocketConfig};

    #[test]
    fn test_default_config() {
        let config = WebSocketConfig::default();

        assert_eq!(config.max_message_size, 64 * 1024 * 1024); // 64MB
        assert_eq!(config.max_frame_size, 16 * 1024 * 1024); // 16MB
        assert_eq!(config.ping_interval, Some(30)); // 30 seconds
        assert_eq!(config.ping_timeout, 10); // 10 seconds
        assert_eq!(config.compression, false);
        assert_eq!(config.write_buffer_size, 128 * 1024); // 128KB
        assert_eq!(config.max_write_queue_size, 1024);
        assert_eq!(config.subprotocols, Vec::<String>::new());
    }

    #[test]
    fn test_custom_config() {
        let config = WebSocketConfig {
            max_message_size: 1024 * 1024, // 1MB
            max_frame_size: 512 * 1024,    // 512KB
            ping_interval: Some(15),       // 15 seconds
            ping_timeout: 5,               // 5 seconds
            compression: true,
            write_buffer_size: 64 * 1024, // 64KB
            max_write_queue_size: 512,
            subprotocols: vec!["chat".to_string(), "binary".to_string()],
        };

        assert_eq!(config.max_message_size, 1024 * 1024);
        assert_eq!(config.max_frame_size, 512 * 1024);
        assert_eq!(config.ping_interval, Some(15));
        assert_eq!(config.ping_timeout, 5);
        assert!(config.compression);
        assert_eq!(config.write_buffer_size, 64 * 1024);
        assert_eq!(config.max_write_queue_size, 512);
        assert_eq!(config.subprotocols, vec!["chat", "binary"]);
    }

    #[tokio::test]
    async fn test_websocket_with_custom_config() {
        let channel_manager = Arc::new(ChannelManager::new());
        let (tx, _rx) = tokio::sync::mpsc::channel(1000);

        let config = Arc::new(WebSocketConfig {
            max_message_size: 1024,
            max_frame_size: 512,
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
        assert_eq!(ws.config().max_message_size, 1024);
        assert_eq!(ws.config().max_frame_size, 512);
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = WebSocketConfig {
            max_message_size: 2 * 1024 * 1024,
            ping_interval: Some(60),
            compression: true,
            ..Default::default()
        };

        assert_eq!(config.max_message_size, 2 * 1024 * 1024);
        assert_eq!(config.ping_interval, Some(60));
        assert!(config.compression);
        // Verify defaults are preserved
        assert_eq!(config.max_frame_size, 16 * 1024 * 1024);
    }

    #[test]
    fn test_config_disable_ping() {
        let config = WebSocketConfig {
            ping_interval: None, // Disable ping
            ..Default::default()
        };

        assert_eq!(config.ping_interval, None);
        assert_eq!(config.ping_timeout, 10); // Still has timeout value
    }

    #[test]
    fn test_config_reasonable_limits() {
        let config = WebSocketConfig {
            max_message_size: 1000,
            max_frame_size: 500,
            ping_interval: Some(5),
            ping_timeout: 2,
            ..Default::default()
        };

        // Ensure message size >= frame size makes sense
        assert!(config.max_message_size >= config.max_frame_size);
        // Ensure ping interval > timeout makes sense
        assert!(config.ping_interval.unwrap_or(0) > config.ping_timeout);
    }
}
