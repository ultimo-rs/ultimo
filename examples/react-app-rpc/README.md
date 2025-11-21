# Ultimo RPC Client Example (React)

This is a full-stack example demonstrating Ultimo Framework's RPC capabilities with auto-generated TypeScript client.

## Features

- **Type-Safe RPC**: Auto-generated TypeScript client from Rust procedure definitions
- **Function-Based API**: Call backend functions directly (`listUsers()`, `getUser()`, `createUser()`)
- **TanStack Query**: Server state management with caching and optimistic updates
- **shadcn/ui**: Beautiful, accessible UI components
- **Zero Manual Typing**: Types automatically synced between Rust and TypeScript

## Prerequisites

1. **Backend Server**: Run the RPC server first

   ```bash
   cd ../../
   cargo run --bin rpc-server
   ```

   Server will start at `http://localhost:3000`

2. **Node.js**: Version 18 or higher

## Getting Started

```bash
# Install dependencies
npm install

# Start development server (runs on port 5174)
npm run dev
```

Open [http://localhost:5174](http://localhost:5174) to view the app.

## RPC Procedures Used

- `listUsers()` - List all users
- `getUser(id)` - Get user by ID
- `createUser(name, email)` - Create new user

## Type Generation

The TypeScript client (`src/lib/ultimo-client.ts`) is automatically generated from Rust types. When you modify the backend:

1. Update Rust procedures in `../../openapi-demo/src/rpc-server.rs`
2. Regenerate types (future: `ultimo generate-client`)
3. Types automatically stay in sync!

## Project Structure

```
src/
├── App.tsx              # Main app component
├── main.tsx             # App entry point
├── pages/
│   └── RpcExample.tsx   # RPC demo with function calls
├── components/
│   └── ui/              # shadcn/ui components
└── lib/
    ├── ultimo-client.ts   # Auto-generated RPC client
    └── utils.ts         # Utility functions
```

## OpenAPI Documentation

Visit [http://localhost:3000/docs](http://localhost:3000/docs) to see the interactive Swagger UI documentation for the RPC API.

## Technologies

- **React 18**: Modern React with hooks
- **TypeScript**: Static type checking (auto-generated from Rust!)
- **Vite**: Fast build tool and dev server
- **TanStack Query**: Async state management
- **shadcn/ui**: Component library built on Radix UI
- **Tailwind CSS**: Utility-first styling
- **Ultimo RPC**: Type-safe remote procedure calls
