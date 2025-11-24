# WebSocket API Design for Ultimo

## Design Goals

1. **Clean API**: Simple, intuitive interface inspired by Bun's WebSocket API
2. **Type Safety**: Strongly-typed context data and messages
3. **Built-in Pub/Sub**: First-class support for channels/rooms
4. **High Performance**: Zero-copy where possible, efficient memory usage
5. **Integration**: Seamless integration with Ultimo's routing and middleware

## API Overview

### Server-Side API

```rust
use ultimo::{Ultimo, WebSocket, WebSocketUpgrade, Request, Response};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct WsData {
    user_id: String,
    channel: String,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    text: String,
    timestamp: u64,
}

let app = Ultimo::new()
    .get("/ws", |req: Request| async move {
        // Upgrade HTTP request to WebSocket
        WebSocketUpgrade::new(req)
            .on_upgrade(|ws: WebSocket<WsData>| async move {
                // Access typed context data
                let user_id = &ws.data().user_id;
                let channel = &ws.data().channel;
                
                // Subscribe to channels
                ws.subscribe(&format!("chat:{}", channel)).await;
                ws.subscribe("global").await;
                
                // Send welcome message
                ws.send_json(&ChatMessage {
                    text: format!("Welcome {}!", user_id),
                    timestamp: now(),
                }).await.ok();
                
                // Announce to channel
                ws.publish(&format!("chat:{}", channel), &ChatMessage {
                    text: format!("{} joined", user_id),
                    timestamp: now(),
                }).await.ok();
                
                // Handle messages
                while let Some(msg) = ws.recv().await {
                    match msg {
                        Message::Text(text) => {
                            let chat_msg = ChatMessage {
                                text,
                                timestamp: now(),
                            };
                            ws.publish(&format!("chat:{}", channel), &chat_msg).await.ok();
                        }
                        Message::Close(_) => break,
                        _ => {}
                    }
                }
                
                // Cleanup on disconnect
                ws.unsubscribe(&format!("chat:{}", channel)).await;
            })
            .with_data(WsData {
                user_id: req.query("user_id").unwrap_or("anonymous").to_string(),
                channel: req.query("channel").unwrap_or("general").to_string(),
            })
            .build()
    });
```

### Alternative: Handler-Based API

```rust
use ultimo::{WebSocketHandler, WebSocket, Message};

struct ChatHandler;

impl WebSocketHandler for ChatHandler {
    type Data = WsData;
    
    async fn on_open(&self, ws: &WebSocket<Self::Data>) {
        let channel = &ws.data().channel;
        ws.subscribe(&format!("chat:{}", channel)).await;
        
        ws.send_json(&ChatMessage {
            text: "Welcome!".to_string(),
            timestamp: now(),
        }).await.ok();
    }
    
    async fn on_message(&self, ws: &WebSocket<Self::Data>, msg: Message) {
        if let Message::Text(text) = msg {
            let channel = &ws.data().channel;
            ws.publish(&format!("chat:{}", channel), &ChatMessage {
                text,
                timestamp: now(),
            }).await.ok();
        }
    }
    
    async fn on_close(&self, ws: &WebSocket<Self::Data>, code: u16, reason: &str) {
        let channel = &ws.data().channel;
        ws.unsubscribe(&format!("chat:{}", channel)).await;
    }
    
    async fn on_drain(&self, ws: &WebSocket<Self::Data>) {
        // Handle backpressure - called when send buffer is writable again
    }
}

// Usage
let app = Ultimo::new()
    .websocket("/ws", ChatHandler);
```

## Core Types

### WebSocket<T>

```rust
pub struct WebSocket<T = ()> {
    // Private fields
}

impl<T> WebSocket<T> {
    /// Get reference to typed context data
    pub fn data(&self) -> &T;
    
    /// Get mutable reference to typed context data
    pub fn data_mut(&mut self) -> &mut T;
    
    /// Send text message
    pub async fn send(&self, text: impl Into<String>) -> Result<()>;
    
    /// Send binary message
    pub async fn send_binary(&self, data: impl Into<Vec<u8>>) -> Result<()>;
    
    /// Send JSON message
    pub async fn send_json<S: Serialize>(&self, data: &S) -> Result<()>;
    
    /// Receive next message
    pub async fn recv(&mut self) -> Option<Message>;
    
    /// Subscribe to a channel/topic
    pub async fn subscribe(&self, topic: impl AsRef<str>) -> Result<()>;
    
    /// Unsubscribe from a channel/topic
    pub async fn unsubscribe(&self, topic: impl AsRef<str>) -> Result<()>;
    
    /// Publish message to all subscribers of a topic
    pub async fn publish<S: Serialize>(&self, topic: impl AsRef<str>, data: &S) -> Result<()>;
    
    /// Close the connection with optional code and reason
    pub async fn close(&self, code: Option<u16>, reason: Option<&str>) -> Result<()>;
    
    /// Check if connection is writable (for backpressure handling)
    pub fn is_writable(&self) -> bool;
    
    /// Get remote address
    pub fn remote_addr(&self) -> Option<SocketAddr>;
}
```

### WebSocketUpgrade

```rust
pub struct WebSocketUpgrade<T = ()> {
    // Private fields
}

impl<T> WebSocketUpgrade<T> {
    /// Create new upgrade from HTTP request
    pub fn new(req: Request) -> Self;
    
    /// Set typed context data
    pub fn with_data(self, data: T) -> Self;
    
    /// Set custom headers for upgrade response
    pub fn with_header(self, key: impl Into<String>, value: impl Into<String>) -> Self;
    
    /// Set protocols
    pub fn with_protocols(self, protocols: Vec<String>) -> Self;
    
    /// Set on_upgrade callback
    pub fn on_upgrade<F, Fut>(self, callback: F) -> Self
    where
        F: FnOnce(WebSocket<T>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static;
    
    /// Build the upgrade response
    pub fn build(self) -> Response;
}
```

### WebSocketHandler Trait

```rust
#[async_trait]
pub trait WebSocketHandler: Send + Sync {
    type Data: Send + Sync + 'static;
    
    /// Called when connection is established
    async fn on_open(&self, ws: &WebSocket<Self::Data>) {
        let _ = ws;
    }
    
    /// Called when message is received
    async fn on_message(&self, ws: &WebSocket<Self::Data>, msg: Message);
    
    /// Called when connection is closed
    async fn on_close(&self, ws: &WebSocket<Self::Data>, code: u16, reason: &str) {
        let _ = (ws, code, reason);
    }
    
    /// Called when send buffer is writable again (backpressure)
    async fn on_drain(&self, ws: &WebSocket<Self::Data>) {
        let _ = ws;
    }
    
    /// Called on error
    async fn on_error(&self, ws: &WebSocket<Self::Data>, error: Error) {
        let _ = (ws, error);
    }
}
```

### Message Enum

```rust
#[derive(Debug, Clone)]
pub enum Message {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close(Option<CloseFrame>),
}

#[derive(Debug, Clone)]
pub struct CloseFrame {
    pub code: u16,
    pub reason: String,
}
```

## Pub/Sub Architecture

### Channel Manager

```rust
pub struct ChannelManager {
    // Maps topic -> Set<WebSocket IDs>
    subscriptions: Arc<RwLock<HashMap<String, HashSet<Uuid>>>>,
    // Maps WebSocket ID -> WebSocket sender
    connections: Arc<RwLock<HashMap<Uuid, mpsc::UnboundedSender<Message>>>>,
}

impl ChannelManager {
    pub async fn subscribe(&self, ws_id: Uuid, topic: &str) -> Result<()>;
    pub async fn unsubscribe(&self, ws_id: Uuid, topic: &str) -> Result<()>;
    pub async fn publish(&self, topic: &str, message: Message) -> Result<usize>;
    pub async fn disconnect(&self, ws_id: Uuid);
}
```

## Configuration

```rust
pub struct WebSocketConfig {
    /// Maximum message size in bytes (default: 64 MB)
    pub max_message_size: usize,
    
    /// Maximum frame size in bytes (default: 16 MB)
    pub max_frame_size: usize,
    
    /// Ping interval (default: 30 seconds)
    pub ping_interval: Option<Duration>,
    
    /// Ping timeout (default: 10 seconds)
    pub ping_timeout: Duration,
    
    /// Enable per-message compression (default: false)
    pub compression: bool,
    
    /// Write buffer size (default: 128 KB)
    pub write_buffer_size: usize,
    
    /// Max write queue size (default: 1024)
    pub max_write_queue_size: usize,
    
    /// Accepted subprotocols
    pub subprotocols: Vec<String>,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_message_size: 64 * 1024 * 1024,
            max_frame_size: 16 * 1024 * 1024,
            ping_interval: Some(Duration::from_secs(30)),
            ping_timeout: Duration::from_secs(10),
            compression: false,
            write_buffer_size: 128 * 1024,
            max_write_queue_size: 1024,
            subprotocols: vec![],
        }
    }
}
```

## Implementation Strategy

### Phase 1: WebSocket Protocol & Core (Week 1)
- [ ] Implement hyper upgrade mechanism
- [ ] WebSocket frame codec (RFC 6455)
  - [ ] Frame header parsing (FIN, opcode, mask, length)
  - [ ] Frame masking/unmasking (client frames must be masked)
  - [ ] Frame fragmentation support
  - [ ] Control frames (ping, pong, close)
- [ ] Basic send/recv functionality
- [ ] Typed context data support
- [ ] Connection lifecycle management
- [ ] Automatic ping/pong handling

### Phase 2: Pub/Sub System (Week 2)
- [ ] Channel manager implementation
  - [ ] Topic → Subscribers mapping (HashMap<String, HashSet<Uuid>>)
  - [ ] Connection → Sender mapping (HashMap<Uuid, mpsc::Sender>)
- [ ] Subscribe/unsubscribe functionality
- [ ] Publish to topics (broadcast to all subscribers)
- [ ] Memory-efficient subscriber tracking
- [ ] Automatic cleanup on disconnect

### Phase 3: Advanced Features (Week 3)
- [ ] Backpressure handling (drain callback)
- [ ] Per-message deflate compression (RFC 7692)
- [ ] Binary and JSON helpers (send_json, recv_json)
- [ ] Error handling and recovery
- [ ] Configuration options (max message size, timeouts, etc.)
- [ ] Graceful shutdown

### Phase 4: Integration & Polish (Week 4)
- [ ] Ultimo router integration (.websocket() method)
- [ ] Middleware support for WebSocket routes
- [ ] OpenAPI/TypeScript generation for WebSocket endpoints
- [ ] Examples (chat, live updates, game server)
- [ ] Documentation (API docs, guides)
- [ ] Performance benchmarks vs tokio-tungstenite
- [ ] Autobahn test suite compliance

## Library Evaluation

### Option 1: tokio-tungstenite
- ✅ Mature, widely used
- ✅ Good tokio integration
- ❌ No built-in pub/sub
- ❌ Heavy API we'll wrap anyway
- ❌ Extra dependency layer on top of hyper
- ❌ Requires wrapping for typed context

### Option 2: fastwebsockets
- ✅ Claims 7x faster than tungstenite
- ✅ Modern API
- ❌ Still another dependency
- ❌ Still no pub/sub
- ❌ Still needs wrapper for our API

### Option 3: Direct Hyper Implementation ✅ RECOMMENDED
- ✅ **Zero extra dependencies** - We already have hyper
- ✅ **Full control** over API design and optimizations
- ✅ **Built-in pub/sub** from day one
- ✅ **Type-safe** by design
- ✅ **Smaller binary** - No extra WebSocket library
- ✅ **Learning value** - Deep understanding of protocol
- ✅ **Optimized for Ultimo** - Can make decisions specific to our use case
- ⚠️ Need to implement WebSocket frame codec (~500 lines)
- ⚠️ Need to handle protocol edge cases

### Recommendation: Build Directly on Hyper

Implement WebSocket protocol directly using **hyper's upgrade mechanism**:

1. Use `hyper::upgrade::on()` to upgrade HTTP connection
2. Implement WebSocket frame encoding/decoding (RFC 6455)
3. Build our clean API inspired by Bun on top
4. Add pub/sub system as core feature
5. Add Ultimo-specific integrations

**Why this is better:**
- Hyper already handles HTTP upgrade handshake
- WebSocket framing is straightforward (bit manipulation for headers, XOR for masking)
- We control performance optimizations
- No dependency bloat
- Can benchmark and iterate quickly

**Architecture:**
```
HTTP Request → Hyper Handler → hyper::upgrade::on()
                                      ↓
                                Upgraded TcpStream
                                      ↓
                             Our WebSocket Frame Codec
                                      ↓
                          WebSocket<T> + Pub/Sub Manager
                                      ↓
                              User's Handler Code
```

## Example Use Cases

### Real-time Chat
```rust
app.websocket("/chat", ChatHandler)
```

### Live Updates
```rust
app.get("/api/orders/:id/ws", |req: Request| async move {
    let order_id = req.param("id")?;
    
    WebSocketUpgrade::new(req)
        .on_upgrade(|ws: WebSocket<OrderData>| async move {
            ws.subscribe(&format!("order:{}", order_id)).await;
            
            while let Some(msg) = ws.recv().await {
                // Handle client messages
            }
        })
        .with_data(OrderData { order_id })
        .build()
})
```

### Game Server
```rust
app.websocket("/game", GameHandler)
    .with_config(WebSocketConfig {
        ping_interval: Some(Duration::from_secs(5)),
        max_message_size: 1024 * 1024, // 1 MB
        ..Default::default()
    })
```

## TypeScript Client Generation

```typescript
// Auto-generated from Rust types
interface WsData {
  user_id: string;
  channel: string;
}

interface ChatMessage {
  text: string;
  timestamp: number;
}

// Type-safe client
const ws = new WebSocket('/ws?user_id=john&channel=general');

ws.addEventListener('message', (event) => {
  const msg: ChatMessage = JSON.parse(event.data);
  console.log(msg.text);
});

ws.send(JSON.stringify({
  text: "Hello!",
  timestamp: Date.now()
} as ChatMessage));
```

## Performance Targets

- **Throughput**: 500k+ messages/second (single core)
- **Latency**: < 1ms p99 for local pub/sub
- **Memory**: < 1KB per connection overhead
- **Concurrent Connections**: 100k+ per server
- **Goal**: Match or exceed tokio-tungstenite performance while providing cleaner API

## WebSocket Protocol Implementation Notes

### Frame Structure (RFC 6455)
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-------+-+-------------+-------------------------------+
|F|R|R|R| opcode|M| Payload len |    Extended payload length    |
|I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
|N|V|V|V|       |S|             |   (if payload len==126/127)   |
| |1|2|3|       |K|             |                               |
+-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
|     Extended payload length continued, if payload len == 127  |
+ - - - - - - - - - - - - - - - +-------------------------------+
|                               |Masking-key, if MASK set to 1  |
+-------------------------------+-------------------------------+
| Masking-key (continued)       |          Payload Data         |
+-------------------------------- - - - - - - - - - - - - - - - +
:                     Payload Data continued ...                :
+ - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
|                     Payload Data continued ...                |
+---------------------------------------------------------------+
```

### Key Implementation Points
1. **Client frames MUST be masked** (server frames MUST NOT be masked)
2. **Control frames** (close, ping, pong) must have payload ≤ 125 bytes
3. **Fragmentation**: Large messages can be split into multiple frames
4. **Close handshake**: Both sides must send close frame
5. **Ping/Pong**: Server should respond to ping with pong (same payload)

### Dependencies We Already Have
- `tokio` - Async runtime
- `hyper` - HTTP server with upgrade support
- `bytes` - Efficient byte buffer manipulation
- `serde_json` - JSON serialization for messages

### What We Need to Build
- Frame parser/serializer (~200 lines)
- Frame masking/unmasking (~20 lines)
- Connection state machine (~100 lines)
- Message fragmentation handler (~100 lines)
- Pub/sub manager (~150 lines)

## Testing Strategy

1. **Unit Tests**: Core WebSocket logic, pub/sub manager
2. **Integration Tests**: Full request/response cycle
3. **Load Tests**: Concurrent connections, message throughput
4. **Compliance Tests**: WebSocket protocol conformance (autobahn)
5. **Example Apps**: Chat, live updates, game server

## Documentation Plan

1. API reference (rustdoc)
2. Getting started guide
3. Pub/sub patterns guide
4. TypeScript integration guide
5. Performance tuning guide
6. Migration guide (from other WebSocket libraries)

---

**Next Steps**: Choose implementation approach and start with Phase 1 core functionality.
