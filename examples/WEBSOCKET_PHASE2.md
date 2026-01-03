# WebSocket Examples - Phase 2 Features

The WebSocket examples have been updated to demonstrate all Phase 2 features:

## Features Demonstrated

### 1. **Configuration System**
Both examples use `websocket_with_config()` to customize WebSocket behavior:
- Custom message size limits
- Custom frame size limits (enables automatic fragmentation)
- Ping/pong heartbeat configuration
- Write buffer size configuration

### 2. **Message Fragmentation**
- Automatically fragments messages larger than `max_frame_size`
- Transparently reassembles fragmented messages on receive
- Example: `max_frame_size: 512 * 1024` (512 KB chunks)

### 3. **Automatic Ping/Pong**
- Keeps connections alive with automatic heartbeat
- Detects dead connections via timeout
- Configuration:
  ```rust
  ping_interval: Some(30),  // Send ping every 30 seconds
  ping_timeout: 10,         // Disconnect if no pong after 10 seconds
  ```

### 4. **Graceful Shutdown**
- Proper close frame handling
- Clean connection termination
- Standard close codes (1000 = Normal closure)

### 5. **Backpressure Handling**
- Bounded write queues prevent memory exhaustion
- `on_drain()` callback notified when buffer clears
- Send operations return `WouldBlock` when buffer full
- Configuration: `max_write_queue_size: 50`

## Running the Examples

### websocket-chat
```bash
cd examples/websocket-chat
cargo run
# Open http://localhost:4000
```

### websocket-chat-react
```bash
# Terminal 1: Start backend
cd examples/websocket-chat-react
cargo run

# Terminal 2: Start React frontend
cd examples/websocket-chat-react
npm install
npm run dev
# Open http://localhost:5173
```

## Code Highlights

### Custom Configuration
```rust
let ws_config = WebSocketConfig {
    ping_interval: Some(30),         // Heartbeat
    ping_timeout: 10,                // Timeout detection
    max_message_size: 5 * 1024 * 1024,
    max_frame_size: 512 * 1024,      // Fragmentation threshold
    max_write_queue_size: 50,        // Backpressure control
    ..Default::default()
};

app.websocket_with_config("/ws", Handler, ws_config);
```

### Backpressure Callback
```rust
async fn on_drain(&self, ws: &WebSocket<Self::Data>) {
    println!(
        "Buffer drained. Capacity: {}/{}",
        ws.capacity(),
        ws.max_capacity()
    );
}
```

### Capacity Checking
```rust
if !ws.has_capacity() {
    // Buffer is full, handle backpressure
    warn!("Connection backpressured");
}
```

## What's Next?

These examples demonstrate production-ready WebSocket features. For more advanced use cases:

- See `ultimo/tests/websocket_backpressure.rs` for backpressure patterns
- See `ultimo/tests/websocket_fragmentation.rs` for large message handling
- See `ultimo/tests/websocket_ping_pong.rs` for heartbeat examples
- See `ultimo/tests/websocket_shutdown.rs` for graceful shutdown patterns
