# Ultimo Framework - React Examples

This directory contains two separate React applications demonstrating different ways to use Ultimo Framework:

## 📁 Examples

### 1. REST API Example (`react-app-rest/`)

Traditional RESTful API with resource-based endpoints.

- **Port**: 5173
- **Backend**: `rest-server` (port 3000)
- **Architecture**: REST (GET/POST/PUT/DELETE on `/api/users`)

**Start the example:**

```bash
# Terminal 1: Start REST backend
cargo run --bin rest-server

# Terminal 2: Start React frontend
cd examples/react-app-rest
npm install
npm run dev
```

Visit: http://localhost:5173

### 2. RPC Client Example (`react-app-rpc/`)

Type-safe RPC with auto-generated TypeScript client from Rust.

- **Port**: 5174
- **Backend**: `rpc-server` (port 3000)
- **Architecture**: RPC (function calls like `listUsers()`, `createUser()`)

**Start the example:**

```bash
# Terminal 1: Start RPC backend
cargo run --bin rpc-server

# Terminal 2: Start React frontend
cd examples/react-app-rpc
npm install
npm run dev
```

Visit: http://localhost:5174

## 🚀 Running Both Simultaneously

You can run both apps at the same time by switching backend servers:

```bash
# Terminal 1: Start REST backend
cargo run --bin rest-server

# Terminal 2: REST frontend (port 5173)
cd examples/react-app-rest && npm run dev

# Terminal 3: Stop REST backend (Ctrl+C), start RPC backend
cargo run --bin rpc-server

# Terminal 4: RPC frontend (port 5174)
cd examples/react-app-rpc && npm run dev
```

Or use separate Ultimo instances on different ports (requires config changes).

## 📚 Documentation

- **REST Server**: `../openapi-demo/src/rest-server.rs`
- **RPC Server**: `../openapi-demo/src/rpc-server.rs`
- **OpenAPI Docs**: http://localhost:3000/docs (when server is running)

## 🛠️ Technologies

Both apps use:

- React 18 with TypeScript
- TanStack Query for state management
- shadcn/ui components
- Tailwind CSS styling
- Vite for build tooling

The key difference is how they communicate with the backend:

- **REST**: Manual fetch calls to `/api/users` endpoints
- **RPC**: Auto-generated client with typed function calls
