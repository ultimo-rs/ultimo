# WebSocket Chat Example

A real-time chat room application built with Ultimo's WebSocket support.

## Features

- 💬 Real-time messaging between multiple clients
- 🔔 Join/leave notifications
- 🎨 Modern, responsive UI
- 🔄 Automatic reconnection
- 📡 Pub/Sub pattern for message broadcasting

## Running the Example

```bash
# From the project root
cargo run -p websocket-chat

# Or from the example directory
cd examples/websocket-chat
cargo run
```

Then open your browser to http://localhost:3000

## How It Works

### Server Side

The server uses Ultimo's WebSocket handler to manage connections:

```rust
struct ChatHandler {
    room: &'static str,
}

#[async_trait]
impl WebSocketHandler for ChatHandler {
    type Data = ();

    async fn on_open(&self, ws: &WebSocket<Self::Data>) {
        // Subscribe to room
        ws.subscribe(self.room).await.ok();
        ws.send("Welcome!").await.ok();
    }

    async fn on_message(&self, ws: &WebSocket<Self::Data>, msg: Message) {
        // Broadcast to all clients
        ws.publish(self.room, msg).await.ok();
    }
}
```

### Client Side

The client uses standard WebSocket API:

```javascript
const ws = new WebSocket("ws://localhost:3000/ws");

ws.onmessage = (event) => {
  displayMessage(event.data);
};

ws.send(JSON.stringify({ message: "Hello!" }));
```

## Architecture

- **Pub/Sub Pattern**: Uses Ultimo's built-in `ChannelManager` for topic-based message broadcasting
- **Connection Management**: Automatic cleanup when clients disconnect
- **Message Types**: Supports both text (JSON) and binary messages
- **Error Handling**: Graceful error handling and reconnection logic

## Try It Out

1. Open multiple browser windows/tabs to http://localhost:3000
2. Send messages from any window
3. See messages appear in all connected clients in real-time
4. Close a tab and see the "user left" notification

## API Endpoints

- `GET /` - Serves the HTML chat interface
- `WS /ws` - WebSocket endpoint for chat messages

## Message Format

Messages are sent as JSON:

```json
{
  "type": "message",
  "message": "Hello, world!",
  "timestamp": "2025-11-25T10:00:00Z"
}
```

System notifications:

```json
{
  "type": "join",
  "message": "A user joined the room"
}
```

## Next Steps

- Add user authentication
- Support multiple chat rooms
- Add message history persistence
- Implement typing indicators
- Add file sharing support
