# WebSocket Chat with React

A real-time chat application demonstrating Ultimo's WebSocket capabilities with a modern React frontend using shadcn/ui components.

## Features

- 🔌 **Real-time WebSocket Communication** - Instant message delivery
- ⚡ **Auto-Reconnect** - Automatically reconnects on connection loss
- 🎨 **Modern UI** - Built with shadcn/ui and Tailwind CSS
- 📱 **Responsive Design** - Works on all screen sizes
- 💬 **Message Echo** - Server echoes messages back to demonstrate bi-directional communication
- 🔄 **Connection Status** - Visual indicator of WebSocket connection state

## Architecture

### Backend (Rust)

- Ultimo web framework with WebSocket support
- Handles WebSocket connections at `/ws` endpoint
- Echoes messages back to clients
- Publishes messages to a "chat" channel for broadcasting

### Frontend (React)

- Vite + React 19 + TypeScript
- shadcn/ui components for modern UI
- Custom `useWebSocket` hook for connection management
- Auto-scrolling message list
- Real-time connection status indicator

## Running the Application

### Start the Backend Server

```bash
cd examples/websocket-chat-react
cargo run
```

The server will start on `http://localhost:3000` with WebSocket endpoint at `ws://localhost:3000/ws`.

### Start the Frontend

In a separate terminal:

```bash
cd examples/websocket-chat-react
npm install  # First time only
npm run dev
```

The React app will open at `http://localhost:5173`.

## Usage

1. Open the React app in your browser
2. Once connected (green indicator), type a message and press Enter or click Send
3. Your message appears on the right (blue)
4. The server's echo appears on the left (gray)
5. System messages (connection status) appear in the center

## Code Structure

```
websocket-chat-react/
├── src/
│   ├── main.rs                    # Rust WebSocket server
│   ├── App.tsx                    # Main React app
│   ├── hooks/
│   │   └── useWebSocket.ts        # WebSocket connection hook
│   ├── components/
│   │   ├── ChatMessage.tsx        # Individual message component
│   │   ├── MessageList.tsx        # Scrollable message list
│   │   ├── MessageInput.tsx       # Input field with send button
│   │   ├── ConnectionStatus.tsx   # Connection indicator
│   │   └── ui/                    # shadcn/ui components
│   │       ├── button.tsx
│   │       ├── input.tsx
│   │       └── card.tsx
│   └── lib/
│       └── utils.ts               # Utility functions
├── Cargo.toml                     # Rust dependencies
└── package.json                   # npm dependencies
```

## WebSocket Handler Implementation

The `ChatHandler` implements the `WebSocketHandler` trait:

```rust
impl WebSocketHandler for ChatHandler {
    async fn on_connect(&self, conn: Arc<WebSocketConnection>) {
        // Send welcome message
    }

    async fn on_message(&self, conn: Arc<WebSocketConnection>, message: Message) {
        // Handle incoming messages, echo back, and broadcast
    }

    async fn on_disconnect(&self, conn: Arc<WebSocketConnection>) {
        // Handle disconnection
    }
}
```

## React WebSocket Hook

The `useWebSocket` hook provides:

- Automatic connection management
- Auto-reconnect on disconnect (3 second delay)
- Message history
- Connection status tracking
- Send message functionality

## Extending the Example

Ideas for enhancement:

- Add usernames/authentication
- Implement chat rooms/channels
- Add message persistence
- Show online user list
- Add typing indicators
- Support message editing/deletion
- Add file/image sharing
- Implement private messages

## Dependencies

### Backend

- `ultimo` - Web framework with WebSocket support
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization

### Frontend

- `react` - UI library
- `vite` - Build tool
- `tailwindcss` - Styling
- `shadcn/ui` - UI components
- `lucide-react` - Icons
