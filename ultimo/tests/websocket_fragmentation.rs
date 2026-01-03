//! Tests for WebSocket message fragmentation

#[cfg(feature = "websocket")]
mod websocket_fragmentation_tests {
    use bytes::Bytes;
    use ultimo::websocket::test_helpers::*;

    #[test]
    fn test_to_fragmented_frames_small_message() {
        let message = Message::Text("Hello".to_string());
        let frames = message.to_fragmented_frames(1024);

        // Small message should not be fragmented
        assert_eq!(frames.len(), 1);
        assert!(frames[0].fin);
        assert_eq!(frames[0].opcode, OpCode::Text);
    }

    #[test]
    fn test_to_fragmented_frames_large_text() {
        // Create a 10KB message
        let large_text = "A".repeat(10_000);
        let message = Message::Text(large_text.clone());

        // Fragment with 4KB chunks
        let frames = message.to_fragmented_frames(4096);

        // Should create 3 frames: 4KB + 4KB + ~2KB
        assert_eq!(frames.len(), 3);

        // First frame: Text opcode, not final
        assert!(!frames[0].fin);
        assert_eq!(frames[0].opcode, OpCode::Text);
        assert_eq!(frames[0].payload.len(), 4096);

        // Middle frame: Continue opcode, not final
        assert!(!frames[1].fin);
        assert_eq!(frames[1].opcode, OpCode::Continue);
        assert_eq!(frames[1].payload.len(), 4096);

        // Last frame: Continue opcode, final
        assert!(frames[2].fin);
        assert_eq!(frames[2].opcode, OpCode::Continue);
        assert_eq!(frames[2].payload.len(), 10_000 - 8192);

        // Reassemble and verify
        let mut reassembled = Vec::new();
        for frame in frames {
            reassembled.extend_from_slice(&frame.payload);
        }
        assert_eq!(String::from_utf8(reassembled).unwrap(), large_text);
    }

    #[test]
    fn test_to_fragmented_frames_large_binary() {
        // Create a 15KB binary message
        let large_binary = vec![42u8; 15_000];
        let message = Message::Binary(Bytes::from(large_binary.clone()));

        // Fragment with 5KB chunks
        let frames = message.to_fragmented_frames(5120);

        // Should create 3 frames
        assert_eq!(frames.len(), 3);

        // First frame
        assert!(!frames[0].fin);
        assert_eq!(frames[0].opcode, OpCode::Binary);
        assert_eq!(frames[0].payload.len(), 5120);

        // Second frame
        assert!(!frames[1].fin);
        assert_eq!(frames[1].opcode, OpCode::Continue);
        assert_eq!(frames[1].payload.len(), 5120);

        // Last frame
        assert!(frames[2].fin);
        assert_eq!(frames[2].opcode, OpCode::Continue);
        assert_eq!(frames[2].payload.len(), 15_000 - 10_240);

        // Reassemble and verify
        let mut reassembled = Vec::new();
        for frame in frames {
            reassembled.extend_from_slice(&frame.payload);
        }
        assert_eq!(reassembled, large_binary);
    }

    #[test]
    fn test_to_fragmented_frames_exact_chunk_size() {
        // Message exactly 8KB
        let message = Message::Text("X".repeat(8192));
        let frames = message.to_fragmented_frames(4096);

        // Should create exactly 2 frames
        assert_eq!(frames.len(), 2);
        assert!(!frames[0].fin);
        assert_eq!(frames[0].payload.len(), 4096);
        assert!(frames[1].fin);
        assert_eq!(frames[1].payload.len(), 4096);
    }

    #[test]
    fn test_to_fragmented_frames_control_frames_not_fragmented() {
        // Ping and Pong frames should never be fragmented
        let ping = Message::Ping(Bytes::from(vec![1u8; 10_000]));
        let frames = ping.to_fragmented_frames(100);
        assert_eq!(frames.len(), 1);
        assert!(frames[0].fin);

        let pong = Message::Pong(Bytes::from(vec![1u8; 10_000]));
        let frames = pong.to_fragmented_frames(100);
        assert_eq!(frames.len(), 1);
        assert!(frames[0].fin);
    }

    #[test]
    fn test_fragmented_frame_encoding() {
        let message = Message::Text("Hello World!".to_string());
        let frames = message.to_fragmented_frames(5);

        // Each frame should encode/decode correctly
        for frame in frames {
            let encoded = frame.encode();
            let mut buf = bytes::BytesMut::from(encoded.as_ref());
            let decoded = Frame::parse(&mut buf).unwrap().unwrap();

            assert_eq!(decoded.fin, frame.fin);
            assert_eq!(decoded.opcode, frame.opcode);
            assert_eq!(decoded.payload, frame.payload);
        }
    }

    #[test]
    fn test_single_byte_chunks() {
        // Extreme case: fragment into single bytes
        let message = Message::Text("ABC".to_string());
        let frames = message.to_fragmented_frames(1);

        assert_eq!(frames.len(), 3);

        assert!(!frames[0].fin);
        assert_eq!(frames[0].opcode, OpCode::Text);
        assert_eq!(frames[0].payload.len(), 1);

        assert!(!frames[1].fin);
        assert_eq!(frames[1].opcode, OpCode::Continue);
        assert_eq!(frames[1].payload.len(), 1);

        assert!(frames[2].fin);
        assert_eq!(frames[2].opcode, OpCode::Continue);
        assert_eq!(frames[2].payload.len(), 1);
    }

    #[test]
    fn test_empty_message() {
        let message = Message::Text(String::new());
        let frames = message.to_fragmented_frames(1024);

        // Empty message should still produce one frame
        assert_eq!(frames.len(), 1);
        assert!(frames[0].fin);
        assert_eq!(frames[0].payload.len(), 0);
    }

    #[test]
    fn test_fragmentation_preserves_utf8() {
        // Test with UTF-8 characters
        let message = Message::Text("Hello 世界 🌍".to_string());
        let frames = message.to_fragmented_frames(5);

        // Reassemble
        let mut reassembled = Vec::new();
        for frame in frames {
            reassembled.extend_from_slice(&frame.payload);
        }

        let result = String::from_utf8(reassembled).unwrap();
        assert_eq!(result, "Hello 世界 🌍");
    }
}
