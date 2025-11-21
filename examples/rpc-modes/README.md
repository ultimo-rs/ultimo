# Ultimo RPC Modes Example

This example demonstrates the two RPC modes available in Ultimo:

## 🌐 REST Mode

Individual endpoints with HTTP semantics:

```rust
let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);

// Queries use GET
rpc.query("listUsers", handler, input_type, output_type);

// Mutations use POST
rpc.mutation("createUser", handler, input_type, output_type);
```

**Generated TypeScript Client (REST Mode):**

```typescript
const client = new UltimoRpcClient("/api");

// GET /api/listUsers
await client.listUsers({});

// POST /api/createUser
await client.createUser({ name: "Alice", email: "alice@example.com" });
```

**Benefits:**

- ✅ Clear URLs in browser Network tab
- ✅ HTTP caching works (GET requests)
- ✅ RESTful conventions
- ⚠️ More complex routing

## ⚡ JSON-RPC Mode (Default)

Single endpoint with method dispatch:

```rust
let rpc = RpcRegistry::new(); // or new_with_mode(RpcMode::JsonRpc)

rpc.register_with_types("listUsers", handler, input_type, output_type);
rpc.register_with_types("createUser", handler, input_type, output_type);
```

**Generated TypeScript Client (JSON-RPC Mode):**

```typescript
const client = new UltimoRpcClient("/rpc");

// POST /rpc with { method: "listUsers", params: {} }
await client.listUsers({});

// POST /rpc with { method: "createUser", params: {...} }
await client.createUser({ name: "Alice", email: "alice@example.com" });
```

**Benefits:**

- ✅ Simple routing (one endpoint)
- ✅ Easy request batching
- ✅ Single middleware point
- ⚠️ All requests look the same in Network tab

## Running the Example

```bash
cargo run
```

This will:

1. Generate `ultimo-client-rest.ts` for REST mode
2. Generate `ultimo-client-jsonrpc.ts` for JSON-RPC mode
3. Show a comparison of both approaches

## When to Use Each Mode

**Use REST Mode when:**

- Building public APIs
- HTTP caching is important
- You want clear, RESTful URLs
- Debugging in browser DevTools is frequent

**Use JSON-RPC Mode when:**

- Building internal/backend-to-backend APIs
- Request batching is needed
- You prefer simpler routing logic
- You're familiar with JSON-RPC patterns

## Comparison with Other Frameworks

Based on analysis of Hono.js RPC, Elysia Eden, and tRPC, Ultimo's hybrid approach provides:

- **Flexibility**: Choose the pattern that fits your use case
- **Type Safety**: Full TypeScript generation in both modes
- **Rust Backend**: Performance and safety of Rust
- **Explicit Generation**: No magic proxies or runtime type reflection

See [RPC_COMPARISON.md](../../docs/RPC_COMPARISON.md) for detailed analysis.
