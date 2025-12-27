//! Property-based tests for WebSocket frame codec
//!
//! These tests use proptest to generate random inputs and verify properties
//! that should hold for all valid inputs.

#![cfg(all(test, feature = "websocket"))]

use bytes::{Bytes, BytesMut};
use proptest::prelude::*;
use ultimo::websocket::test_helpers::{Frame, OpCode};

/// Strategy for generating random payloads
fn payload_strategy() -> impl Strategy<Value = Bytes> {
    prop::collection::vec(any::<u8>(), 0..1000).prop_map(|v| Bytes::from(v))
}

/// Strategy for generating valid UTF-8 text
fn text_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 .,!?-]{0,1000}"
}

/// Strategy for generating valid opcodes
fn opcode_strategy() -> impl Strategy<Value = OpCode> {
    prop_oneof![
        Just(OpCode::Continue),
        Just(OpCode::Text),
        Just(OpCode::Binary),
        Just(OpCode::Close),
        Just(OpCode::Ping),
        Just(OpCode::Pong),
    ]
}

proptest! {
    /// Property: Encoding and then decoding a frame should yield the original payload
    #[test]
    fn prop_frame_round_trip(payload in payload_strategy(), masked in any::<bool>()) {
        let original = Frame {
            fin: true,
            opcode: OpCode::Binary,
            mask: if masked { Some([0x12, 0x34, 0x56, 0x78]) } else { None },
            payload: payload.clone(),
        };

        let encoded = original.encode();
        let mut buf = BytesMut::from(encoded.as_ref());
        let decoded = Frame::parse(&mut buf).expect("Failed to parse encoded frame")
            .expect("Should have complete frame");

        // Payload should match (masking is transparent after parsing)
        prop_assert_eq!(decoded.payload, payload);
        prop_assert_eq!(decoded.opcode, OpCode::Binary);
        prop_assert_eq!(decoded.fin, true);
    }

    /// Property: Text frames with valid UTF-8 should always parse correctly
    #[test]
    fn prop_text_frame_valid_utf8(text in text_strategy()) {
        let frame = Frame {
            fin: true,
            opcode: OpCode::Text,
            mask: None,
            payload: Bytes::from(text.as_bytes().to_vec()),
        };

        let encoded = frame.encode();
        let mut buf = BytesMut::from(encoded.as_ref());
        let decoded = Frame::parse(&mut buf).expect("Failed to parse text frame")
            .expect("Should have complete frame");

        let decoded_text = String::from_utf8(decoded.payload.to_vec())
            .expect("Decoded payload should be valid UTF-8");
        prop_assert_eq!(decoded_text, text);
    }

    /// Property: Any valid opcode should survive encode/decode
    #[test]
    fn prop_opcode_preservation(opcode in opcode_strategy(), payload in payload_strategy()) {
        let frame = Frame {
            fin: true,
            opcode,
            mask: None,
            payload: payload.clone(),
        };

        let encoded = frame.encode();
        let mut buf = BytesMut::from(encoded.as_ref());
        let decoded = Frame::parse(&mut buf).expect("Failed to parse frame")
            .expect("Should have complete frame");

        prop_assert_eq!(decoded.opcode, opcode);
        prop_assert_eq!(decoded.payload, payload);
    }

    /// Property: Masking should be reversible
    #[test]
    fn prop_masking_reversible(payload_vec in prop::collection::vec(any::<u8>(), 0..1000), mask_bytes in any::<[u8; 4]>()) {
        let mut masked = payload_vec.clone();

        // Apply mask twice
        for (i, byte) in masked.iter_mut().enumerate() {
            *byte ^= mask_bytes[i % 4];
        }
        for (i, byte) in masked.iter_mut().enumerate() {
            *byte ^= mask_bytes[i % 4];
        }

        prop_assert_eq!(masked, payload_vec, "Double masking should restore original");
    }

    /// Property: Frame size calculation should be accurate
    #[test]
    fn prop_frame_size_accurate(payload in payload_strategy()) {
        let frame = Frame {
            fin: true,
            opcode: OpCode::Binary,
            mask: Some([0; 4]),
            payload: payload.clone(),
        };

        let encoded = frame.encode();
        let mut buf = BytesMut::from(encoded.as_ref());
        let buf_len_before = buf.len();
        let _ = Frame::parse(&mut buf).expect("Failed to parse frame");
        let buf_len_after = buf.len();

        prop_assert_eq!(buf_len_before - buf_len_after, encoded.len(), "Consumed bytes should match encoded size");
    }

    /// Property: Partial frames should return None without consuming data
    #[test]
    fn prop_partial_frame_handling(payload in payload_strategy()) {
        if payload.is_empty() {
            return Ok(());
        }

        let frame = Frame {
            fin: true,
            opcode: OpCode::Binary,
            mask: None,
            payload: payload.clone(),
        };

        let encoded = frame.encode();
        let partial_len = encoded.len() / 2;
        if partial_len == 0 {
            return Ok(());
        }

        let mut buf = BytesMut::from(&encoded[..partial_len]);
        let result = Frame::parse(&mut buf).expect("Parse should not error");

        prop_assert!(result.is_none(), "Partial frame should return None");
    }

    /// Property: Close frames should preserve reason codes
    #[test]
    fn prop_close_frame_reason(code in 1000u16..5000u16, reason in text_strategy()) {
        let frame = Frame::close(Some(code), Some(&reason));

        let encoded = frame.encode();
        let mut buf = BytesMut::from(encoded.as_ref());
        let decoded = Frame::parse(&mut buf).expect("Failed to parse close frame")
            .expect("Should have complete frame");

        if decoded.payload.len() >= 2 {
            let decoded_code = u16::from_be_bytes([decoded.payload[0], decoded.payload[1]]);
            prop_assert_eq!(decoded_code, code);

            if decoded.payload.len() > 2 {
                let decoded_reason = String::from_utf8_lossy(&decoded.payload[2..]);
                prop_assert_eq!(decoded_reason, reason);
            }
        }
    }

    /// Property: Control frames (ping/pong/close) should never fragment
    #[test]
    fn prop_control_frames_no_fragment(payload in prop::collection::vec(any::<u8>(), 0..125)) {
        let payload_bytes = Bytes::from(payload.clone());

        for opcode in [OpCode::Ping, OpCode::Pong, OpCode::Close] {
            let frame = Frame {
                fin: true,
                opcode,
                mask: None,
                payload: payload_bytes.clone(),
            };

            let encoded = frame.encode();
            let mut buf = BytesMut::from(encoded.as_ref());
            let decoded = Frame::parse(&mut buf).expect("Failed to parse control frame")
                .expect("Should have complete frame");

            prop_assert!(decoded.fin, "Control frames must have FIN=true");
            prop_assert!(decoded.opcode.is_control(), "Should be control frame");
        }
    }
}

#[cfg(test)]
mod exhaustive_tests {
    use bytes::{Bytes, BytesMut};
    use ultimo::websocket::test_helpers::{Frame, OpCode};

    /// Test all valid single-byte opcodes
    #[test]
    fn test_all_valid_opcodes() {
        let opcodes = [
            (0x0, OpCode::Continue),
            (0x1, OpCode::Text),
            (0x2, OpCode::Binary),
            (0x8, OpCode::Close),
            (0x9, OpCode::Ping),
            (0xA, OpCode::Pong),
        ];

        for (byte, expected_opcode) in opcodes {
            let frame = Frame {
                fin: true,
                opcode: expected_opcode,
                mask: None,
                payload: Bytes::new(),
            };

            let encoded = frame.encode();
            assert_eq!(encoded[0] & 0x0F, byte, "Opcode should match");

            let mut buf = BytesMut::from(encoded.as_ref());
            let decoded = Frame::parse(&mut buf)
                .expect("Failed to parse")
                .expect("Should have complete frame");
            assert_eq!(decoded.opcode, expected_opcode);
        }
    }

    /// Test all payload length encoding ranges
    #[test]
    fn test_payload_length_boundaries() {
        let test_cases = vec![
            (0, 2),      // 0 bytes: 2 byte header
            (125, 2),    // 125 bytes: 2 byte header
            (126, 4),    // 126 bytes: 2 + 2 byte extended length
            (127, 4),    // 127 bytes: 2 + 2 byte extended length
            (65535, 4),  // 65535 bytes: 2 + 2 byte extended length
            (65536, 10), // 65536 bytes: 2 + 8 byte extended length
        ];

        for (payload_len, expected_header_size) in test_cases {
            let payload = Bytes::from(vec![0u8; payload_len]);
            let frame = Frame {
                fin: true,
                opcode: OpCode::Binary,
                mask: None,
                payload,
            };

            let encoded = frame.encode();
            assert_eq!(
                encoded.len(),
                expected_header_size + payload_len,
                "Encoded size should match for payload length {}",
                payload_len
            );

            let mut buf = BytesMut::from(encoded.as_ref());
            let buf_len_before = buf.len();
            let decoded = Frame::parse(&mut buf)
                .expect("Failed to parse")
                .expect("Should have complete frame");
            let consumed = buf_len_before - buf.len();
            assert_eq!(consumed, encoded.len());
            assert_eq!(decoded.payload.len(), payload_len);
        }
    }

    /// Test FIN bit in all combinations
    #[test]
    fn test_fin_bit_combinations() {
        for fin in [true, false] {
            let frame = Frame {
                fin,
                opcode: OpCode::Text,
                mask: None,
                payload: Bytes::from(&b"test"[..]),
            };

            let encoded = frame.encode();
            if fin {
                assert_eq!(encoded[0] & 0x80, 0x80, "FIN bit should be set");
            } else {
                assert_eq!(encoded[0] & 0x80, 0x00, "FIN bit should be clear");
            }

            let mut buf = BytesMut::from(encoded.as_ref());
            let decoded = Frame::parse(&mut buf)
                .expect("Failed to parse")
                .expect("Should have complete frame");
            assert_eq!(decoded.fin, fin);
        }
    }

    /// Test RSV bits (should always be 0 without extensions)
    #[test]
    fn test_rsv_bits_always_zero() {
        let frame = Frame {
            fin: true,
            opcode: OpCode::Binary,
            mask: None,
            payload: Bytes::from(vec![1, 2, 3]),
        };

        let encoded = frame.encode();
        let rsv_bits = encoded[0] & 0x70; // Bits 4-6
        assert_eq!(rsv_bits, 0, "RSV bits should be 0");
    }
}
