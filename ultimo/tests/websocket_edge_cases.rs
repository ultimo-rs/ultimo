//! Edge case tests for WebSocket
//!
//! Tests large messages, fragmentation, timeouts, and other edge cases

use bytes::{Bytes, BytesMut, BufMut};
use ultimo::websocket::test_helpers::{Frame, OpCode};

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_very_large_text_message() {
        // Test with 10 MB text message
        let size = 10 * 1024 * 1024;
        let text = "a".repeat(size);
        
        let frame = Frame {
            fin: true,
            opcode: OpCode::Text,
            mask: None,
            payload: text.as_bytes().to_vec().into(),
        };
        
        let encoded = frame.encode();
        assert!(encoded.len() > size);
        
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.payload.len(), size);
        assert_eq!(decoded.opcode, OpCode::Text);
    }

    #[test]
    fn test_very_large_binary_message() {
        // Test with 20 MB binary message
        let size = 20 * 1024 * 1024;
        let data = vec![42u8; size];
        
        let frame = Frame {
            fin: true,
            opcode: OpCode::Binary,
            mask: None,
            payload: data.into(),
        };
        
        let encoded = frame.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.payload.len(), size);
        assert!(decoded.payload.iter().all(|&b| b == 42));
    }

    #[test]
    fn test_fragmented_message_two_parts() {
        // First fragment
        let frame1 = Frame {
            fin: false,
            opcode: OpCode::Text,
            mask: None,
            payload: b"Hello ".to_vec().into(),
        };
        
        // Continuation frame
        let frame2 = Frame {
            fin: true,
            opcode: OpCode::Continue,
            mask: None,
            payload: b"World!".to_vec().into(),
        };
        
        // Encode both
        let encoded1 = frame1.encode();
        let encoded2 = frame2.encode();
        
        // Parse first
        let mut buf = BytesMut::from(&encoded1[..]);
        let decoded1 = Frame::parse(&mut buf).unwrap().unwrap();
        assert!(!decoded1.fin);
        assert_eq!(decoded1.opcode, OpCode::Text);
        
        // Parse second
        let mut buf = BytesMut::from(&encoded2[..]);
        let decoded2 = Frame::parse(&mut buf).unwrap().unwrap();
        assert!(decoded2.fin);
        assert_eq!(decoded2.opcode, OpCode::Continue);
        
        // Combine payloads
        let mut combined = BytesMut::new();
        combined.extend_from_slice(&decoded1.payload);
        combined.extend_from_slice(&decoded2.payload);
        
        assert_eq!(&combined[..], b"Hello World!");
    }

    #[test]
    fn test_fragmented_message_many_parts() {
        // Split a large message into many fragments
        let total_message = "x".repeat(10000);
        let chunk_size = 100;
        let mut fragments = vec![];
        
        for (i, chunk) in total_message.as_bytes().chunks(chunk_size).enumerate() {
            let is_last = (i + 1) * chunk_size >= total_message.len();
            let is_first = i == 0;
            
            let frame = Frame {
                fin: is_last,
                opcode: if is_first { OpCode::Text } else { OpCode::Continue },
                mask: None,
                payload: chunk.to_vec().into(),
            };
            fragments.push(frame);
        }
        
        assert!(fragments.len() > 1);
        
        // Encode and decode all
        let mut reconstructed = BytesMut::new();
        for frame in fragments {
            let encoded = frame.encode();
            let mut buf = BytesMut::from(&encoded[..]);
            let decoded = Frame::parse(&mut buf).unwrap().unwrap();
            reconstructed.extend_from_slice(&decoded.payload);
        }
        
        assert_eq!(&reconstructed[..], total_message.as_bytes());
    }

    #[test]
    fn test_interleaved_control_frames() {
        // Control frames can be interleaved with data frames
        let data_frame1 = Frame {
            fin: false,
            opcode: OpCode::Text,
            mask: None,
            payload: b"Part 1".to_vec().into(),
        };
        
        let ping_frame = Frame {
            fin: true,
            opcode: OpCode::Ping,
            mask: None,
            payload: b"ping".to_vec().into(),
        };
        
        let data_frame2 = Frame {
            fin: true,
            opcode: OpCode::Continue,
            mask: None,
            payload: b" Part 2".to_vec().into(),
        };
        
        // All should encode/decode correctly
        let frames = vec![data_frame1, ping_frame, data_frame2];
        for frame in frames {
            let encoded = frame.encode();
            let mut buf = BytesMut::from(&encoded[..]);
            let decoded = Frame::parse(&mut buf).unwrap().unwrap();
            assert_eq!(decoded.opcode, frame.opcode);
        }
    }

    #[test]
    fn test_maximum_control_frame_payload() {
        // Control frames can have up to 125 bytes
        let payload = vec![0u8; 125];
        
        let frame = Frame {
            fin: true,
            opcode: OpCode::Ping,
            mask: None,
            payload: payload.into(),
        };
        
        let encoded = frame.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.payload.len(), 125);
    }

    #[test]
    fn test_masked_large_payload() {
        let size = 100000;
        let data = vec![0xAAu8; size];
        let mask = [0x12, 0x34, 0x56, 0x78];
        
        let frame = Frame {
            fin: true,
            opcode: OpCode::Binary,
            mask: Some(mask),
            payload: data.clone().into(),
        };
        
        let encoded = frame.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.payload.len(), size);
        assert_eq!(&decoded.payload[..], &data[..]);
    }

    #[test]
    fn test_empty_close_frame() {
        let frame = Frame {
            fin: true,
            opcode: OpCode::Close,
            mask: None,
            payload: Bytes::new(),
        };
        
        let encoded = frame.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.opcode, OpCode::Close);
        assert_eq!(decoded.payload.len(), 0);
    }

    #[test]
    fn test_close_frame_with_code_only() {
        let mut payload = BytesMut::new();
        payload.put_u16(1000); // Normal closure
        
        let frame = Frame {
            fin: true,
            opcode: OpCode::Close,
            mask: None,
            payload: payload.freeze(),
        };
        
        let encoded = frame.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.payload.len(), 2);
        let code = u16::from_be_bytes([decoded.payload[0], decoded.payload[1]]);
        assert_eq!(code, 1000);
    }

    #[test]
    fn test_close_frame_with_long_reason() {
        let mut payload = BytesMut::new();
        payload.put_u16(1001); // Going away
        let reason = "Server is shutting down for maintenance";
        payload.put_slice(reason.as_bytes());
        
        let frame = Frame {
            fin: true,
            opcode: OpCode::Close,
            mask: None,
            payload: payload.freeze(),
        };
        
        let encoded = frame.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        let code = u16::from_be_bytes([decoded.payload[0], decoded.payload[1]]);
        assert_eq!(code, 1001);
        
        let reason_bytes = &decoded.payload[2..];
        assert_eq!(reason_bytes, reason.as_bytes());
    }

    #[test]
    fn test_ping_pong_round_trip() {
        let ping_data = b"ping_payload";
        
        let ping = Frame {
            fin: true,
            opcode: OpCode::Ping,
            mask: None,
            payload: ping_data.to_vec().into(),
        };
        
        // Encode ping
        let encoded = ping.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded_ping = Frame::parse(&mut buf).unwrap().unwrap();
        
        // Create pong response with same payload
        let pong = Frame {
            fin: true,
            opcode: OpCode::Pong,
            mask: None,
            payload: decoded_ping.payload,
        };
        
        // Encode pong
        let encoded = pong.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded_pong = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded_pong.opcode, OpCode::Pong);
        assert_eq!(&decoded_pong.payload[..], ping_data);
    }

    #[test]
    fn test_multiple_frames_in_buffer() {
        // Put multiple frames in a single buffer
        let frame1 = Frame {
            fin: true,
            opcode: OpCode::Text,
            mask: None,
            payload: b"first".to_vec().into(),
        };
        
        let frame2 = Frame {
            fin: true,
            opcode: OpCode::Text,
            mask: None,
            payload: b"second".to_vec().into(),
        };
        
        let frame3 = Frame {
            fin: true,
            opcode: OpCode::Text,
            mask: None,
            payload: b"third".to_vec().into(),
        };
        
        // Combine all encoded frames in one buffer
        let mut combined = BytesMut::new();
        combined.extend_from_slice(&frame1.encode());
        combined.extend_from_slice(&frame2.encode());
        combined.extend_from_slice(&frame3.encode());
        
        // Parse all three
        let decoded1 = Frame::parse(&mut combined).unwrap().unwrap();
        assert_eq!(&decoded1.payload[..], b"first");
        
        let decoded2 = Frame::parse(&mut combined).unwrap().unwrap();
        assert_eq!(&decoded2.payload[..], b"second");
        
        let decoded3 = Frame::parse(&mut combined).unwrap().unwrap();
        assert_eq!(&decoded3.payload[..], b"third");
        
        // Buffer should be empty now
        assert_eq!(combined.len(), 0);
    }

    #[test]
    fn test_partial_frame_accumulation() {
        let frame = Frame {
            fin: true,
            opcode: OpCode::Text,
            mask: None,
            payload: b"complete message".to_vec().into(),
        };
        
        let encoded = frame.encode();
        
        // Split encoded data into chunks
        let chunk1 = &encoded[..5];
        let chunk2 = &encoded[5..10];
        let chunk3 = &encoded[10..];
        
        // Accumulate in buffer
        let mut buf = BytesMut::new();
        
        // First chunk - incomplete
        buf.extend_from_slice(chunk1);
        assert!(Frame::parse(&mut buf).unwrap().is_none());
        
        // Second chunk - still incomplete
        buf.extend_from_slice(chunk2);
        assert!(Frame::parse(&mut buf).unwrap().is_none());
        
        // Third chunk - now complete
        buf.extend_from_slice(chunk3);
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(&decoded.payload[..], b"complete message");
    }

    #[test]
    fn test_boundary_payload_lengths() {
        // Test at key boundaries: 125, 126, 65535, 65536
        let test_sizes = vec![0, 1, 125, 126, 127, 65535, 65536];
        
        for size in test_sizes {
            let payload = vec![0xFFu8; size];
            
            let frame = Frame {
                fin: true,
                opcode: OpCode::Binary,
                mask: None,
                payload: payload.clone().into(),
            };
            
            let encoded = frame.encode();
            let mut buf = BytesMut::from(&encoded[..]);
            let decoded = Frame::parse(&mut buf).unwrap().unwrap();
            
            assert_eq!(decoded.payload.len(), size, "Failed at size {}", size);
            if size > 0 {
                assert!(decoded.payload.iter().all(|&b| b == 0xFF));
            }
        }
    }

    #[test]
    fn test_utf8_emoji_in_text_frame() {
        let emojis = "🚀💬🎉👍✨";
        
        let frame = Frame {
            fin: true,
            opcode: OpCode::Text,
            mask: None,
            payload: emojis.as_bytes().to_vec().into(),
        };
        
        let encoded = frame.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        let decoded_str = String::from_utf8(decoded.payload.to_vec()).unwrap();
        assert_eq!(decoded_str, emojis);
    }

    #[test]
    fn test_multibyte_utf8_boundaries() {
        // Test that we handle UTF-8 boundaries correctly
        let text = "Hello 世界 🌍"; // Mix of ASCII, CJK, and emoji
        
        let frame = Frame {
            fin: true,
            opcode: OpCode::Text,
            mask: None,
            payload: text.as_bytes().to_vec().into(),
        };
        
        let encoded = frame.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        let decoded_str = String::from_utf8(decoded.payload.to_vec()).unwrap();
        assert_eq!(decoded_str, text);
    }
}

#[cfg(test)]
mod timeout_tests {
    use tokio::time::{timeout, Duration};
    use ultimo::websocket::ChannelManager;
    
    #[tokio::test]
    async fn test_subscribe_with_timeout() {
        let manager = ChannelManager::new();
        let conn_id = uuid::Uuid::new_v4();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        
        let result = timeout(
            Duration::from_millis(100),
            manager.subscribe(conn_id, "test_topic", tx)
        ).await;
        
        assert!(result.is_ok(), "Subscribe should complete quickly");
    }
    
    #[tokio::test]
    async fn test_publish_with_timeout() {
        let manager = ChannelManager::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let conn_id = uuid::Uuid::new_v4();
        
        manager.subscribe(conn_id, "test_topic", tx).await.unwrap();
        
        let msg = ultimo::websocket::Message::Text("test".to_string());
        let result = timeout(
            Duration::from_millis(100),
            manager.publish("test_topic", msg)
        ).await;
        
        assert!(result.is_ok(), "Publish should complete quickly");
    }
}
