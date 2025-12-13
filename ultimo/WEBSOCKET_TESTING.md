# WebSocket Testing Strategy

This document describes the comprehensive testing approach for Ultimo's WebSocket implementation.

## Test Suite Overview

The WebSocket implementation includes **42 total tests** across multiple categories:

### 1. Unit Tests (21 tests)

Located inline with the implementation code following Rust best practices.

#### Frame Codec Tests (`src/websocket/frame.rs`)

- `test_text_frame_encode_decode` - Basic text frame round-trip
- `test_binary_frame_encode_decode` - Basic binary frame round-trip
- `test_masked_frame` - Frame masking/unmasking
- `test_close_frame_with_code_and_reason` - Close frame parsing
- `test_extended_payload_length_16bit` - 126-65535 byte payloads
- `test_extended_payload_length_64bit` - 65536+ byte payloads
- `test_partial_frame_parsing` - Incomplete frames return None
- `test_empty_payload` - Zero-length payloads
- `test_ping_pong_frames` - Control frame handling
- `test_close_frame_with_reason` - Close with reason text
- `test_close_frame_without_reason` - Close code only
- `test_close_frame_empty` - Empty close frame
- `test_invalid_utf8_in_text_frame` - UTF-8 validation
- `test_message_to_frame_round_trip` - Message conversion
- `test_opcode_is_control` - Control frame detection
- `test_invalid_opcode` - Invalid opcode rejection

#### Pub/Sub Tests (`src/websocket/pubsub.rs`)

- `test_subscribe_unsubscribe` - Basic subscription lifecycle
- `test_publish_to_subscribers` - Multi-subscriber broadcasting
- `test_disconnect_cleanup` - Automatic cleanup on disconnect

#### Upgrade Tests (`src/websocket/upgrade.rs`)

- `test_calculate_accept_key` - WebSocket handshake key calculation

### 2. Integration Tests (9 tests)

Located in `tests/websocket_integration.rs` - test real-world usage scenarios.

- `test_websocket_send_receive` - Text and binary message exchange
- `test_websocket_json_serialization` - JSON message handling with serde
- `test_websocket_with_data` - Typed context data access
- `test_websocket_pubsub` - Multi-subscriber pub/sub
- `test_websocket_unsubscribe` - Unsubscribe behavior
- `test_websocket_topic_isolation` - Topic isolation (separate rooms)
- `test_websocket_connection_count` - Connection tracking
- `test_websocket_close` - Close frame handling
- `test_websocket_is_writable` - Writable state checking

### 3. Property-Based Tests (12 tests)

Located in `tests/websocket_property.rs` - use proptest to verify properties across thousands of random inputs.

#### Proptest Tests (8 tests)

- `prop_frame_round_trip` - Encode/decode preserves payload (all sizes, masked/unmasked)
- `prop_text_frame_valid_utf8` - UTF-8 text always parses correctly
- `prop_opcode_preservation` - All opcodes survive encode/decode
- `prop_masking_reversible` - Double masking restores original
- `prop_frame_size_accurate` - Byte consumption matches encoded size
- `prop_partial_frame_handling` - Partial frames always return None
- `prop_close_frame_reason` - Close codes and reasons preserved
- `prop_control_frames_no_fragment` - Control frames always have FIN=true

#### Exhaustive Tests (4 tests)

- `test_all_valid_opcodes` - All 6 valid opcodes encode correctly
- `test_payload_length_boundaries` - All length encoding ranges (0, 125, 126, 127, 65535, 65536)
- `test_fin_bit_combinations` - FIN bit true/false
- `test_rsv_bits_always_zero` - RSV bits always 0 without extensions

### 4. Benchmark Tests

Located in `benches/websocket_bench.rs` - measure performance with Criterion.

- `bench_frame_encode` - Encoding speed across payload sizes (0, 125, 126, 1KB, 4KB, 64KB, 65KB)
- `bench_frame_encode_masked` - Encoding with masking overhead
- `bench_frame_decode` - Decoding speed across payload sizes
- `bench_frame_decode_masked` - Decoding with unmasking overhead
- `bench_round_trip` - Full encode+decode cycle

## Running Tests

```bash
# Run all WebSocket tests (unit + integration)
cargo test --package ultimo --features websocket,test-helpers websocket

# Run only unit tests
cargo test --package ultimo --lib --features websocket websocket

# Run only integration tests
cargo test --package ultimo --test websocket_integration --features websocket,test-helpers

# Run property-based tests (generates thousands of random test cases)
cargo test --package ultimo --test websocket_property --features websocket,test-helpers

# Run benchmarks (measures performance)
cargo bench --package ultimo --bench websocket_bench --features websocket,test-helpers

# Run benchmarks in test mode (faster, just verifies they work)
cargo bench --package ultimo --bench websocket_bench --features websocket,test-helpers -- --test
```

## Test Coverage

### Frame Codec Coverage

- ✅ All 6 valid opcodes (Continue, Text, Binary, Close, Ping, Pong)
- ✅ All 3 payload length encodings (7-bit, 16-bit, 64-bit extended)
- ✅ Masked and unmasked frames
- ✅ FIN bit variations (fragmentation)
- ✅ RSV bits (reserved for extensions)
- ✅ Empty payloads
- ✅ Maximum size payloads (65KB+)
- ✅ Partial frame parsing (incomplete data)
- ✅ Invalid UTF-8 in text frames
- ✅ Invalid opcodes
- ✅ Close frames (with/without code/reason)
- ✅ Ping/Pong control frames

### Connection & Messaging Coverage

- ✅ Text message send/receive
- ✅ Binary message send/receive
- ✅ JSON serialization/deserialization
- ✅ Typed context data (`WebSocket<T>`)
- ✅ Close frame handling
- ✅ Writable state checking

### Pub/Sub Coverage

- ✅ Subscribe to topics
- ✅ Unsubscribe from topics
- ✅ Publish to single topic
- ✅ Multi-subscriber broadcasting
- ✅ Topic isolation (separate rooms)
- ✅ Automatic cleanup on disconnect
- ✅ Connection count tracking
- ✅ Topic count tracking
- ✅ Subscriber count per topic

### Property-Based Testing

- ✅ Frame round-trip correctness (all sizes, masking combinations)
- ✅ UTF-8 validation in text frames
- ✅ Opcode preservation across encode/decode
- ✅ Masking reversibility
- ✅ Frame size calculation accuracy
- ✅ Partial frame detection
- ✅ Close frame reason preservation
- ✅ Control frame FIN bit enforcement
- ✅ All opcode values (exhaustive enumeration)
- ✅ All length boundary cases (exhaustive enumeration)
- ✅ FIN bit combinations (exhaustive enumeration)
- ✅ RSV bits always zero (property)

## Test Helpers

The `test-helpers` feature provides utilities for testing:

```rust
use ultimo::websocket::test_helpers::{create_websocket, Frame, OpCode, WebSocket};
use bytes::Bytes;

// Create a test WebSocket connection
let (tx, rx) = mpsc::unbounded_channel();
let manager = Arc::new(ChannelManager::new());
let ws = create_websocket("context_data", tx, manager, Uuid::new_v4(), None);

// Direct frame manipulation (for low-level tests)
let frame = Frame {
    fin: true,
    opcode: OpCode::Text,
    mask: None,
    payload: Bytes::from("hello"),
};
```

## Testing Philosophy

1. **Inline Unit Tests**: Following Rust best practices (tokio, serde, std), unit tests live next to the code they test. This provides:

   - Direct access to private functions and types
   - Clear documentation through examples
   - Easy refactoring (tests move with code)

2. **Separate Integration Tests**: Real-world scenarios in `tests/` directory that test the public API like users will.

3. **Property-Based Testing**: Proptest generates thousands of random inputs to verify universal properties, catching edge cases that example-based tests might miss.

4. **Benchmark Tests**: Criterion measures performance across different payload sizes and operations to detect regressions.

## Test Quality Metrics

- **Total Tests**: 42 (21 unit + 9 integration + 12 property-based)
- **Property Test Cases**: Each proptest runs 256 cases by default = ~2,048 additional test executions
- **Coverage Areas**: Frame encoding/decoding, connection lifecycle, pub/sub, typed context, control frames
- **Edge Cases**: Empty payloads, maximum sizes, partial frames, invalid data, UTF-8 validation
- **Performance**: Benchmarks across 7 payload sizes (0B to 65KB+)

## Future Test Additions

Phase 2 (Ultimo Integration):

- Router `.websocket()` method tests
- Middleware integration tests
- Upgrade handling tests

Phase 3 (Advanced Features):

- Compression tests (per-message deflate)
- Backpressure tests (drain callback)
- Fragmentation tests (large message splitting)
- Ping/Pong timeout tests

Phase 4 (Tooling):

- Autobahn Test Suite compliance
- Performance benchmarks vs tokio-tungstenite
- Memory leak tests
- Concurrent connection stress tests
