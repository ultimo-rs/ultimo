<div align="center">
  <img src="docs-site/docs/public/logo.svg" alt="Ultimo Logo" width="200" />
  <h1>Ultimo</h1>
  <p><strong>Type-safe web framework with automatic TypeScript client generation</strong></p>

  <p>
    <a href="https://crates.io/crates/ultimo"><img src="https://img.shields.io/crates/v/ultimo.svg?style=flat-square" alt="Crates.io" /></a>
    <a href="https://docs.rs/ultimo"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="Documentation" /></a>
    <a href="https://github.com/ultimo-rs/ultimo/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square" alt="License" /></a>
    <a href="https://github.com/ultimo-rs/ultimo/actions"><img src="https://img.shields.io/github/actions/workflow/status/ultimo-rs/ultimo/ci.yml?branch=main&style=flat-square" alt="Build Status" /></a>
  </p>

  <p>
    <a href="https://ultimo.dev">Website</a>
    <span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
    <a href="https://docs.ultimo.dev">Documentation</a>
    <span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
    <a href="https://docs.ultimo.dev/getting-started">Getting Started</a>
    <span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
    <a href="https://github.com/ultimo-rs/ultimo/tree/main/examples">Examples</a>
  </p>

  <br />
</div>

---

⚡ **Industry-leading performance** (158k+ req/sec, 0.6ms latency) - Lightweight, type-safe Rust web framework with **automatic TypeScript client generation** for type-safe full-stack development.

## ✨ Key Features

### Available Now

- ⚡ **Blazing Fast Performance** - Industry-leading speed: 158k+ req/sec, sub-millisecond latency
- 🚀 **Automatic TypeScript Generation** - RPC endpoints automatically generate type-safe TypeScript clients
- 📋 **OpenAPI Support** - Generate OpenAPI 3.0 specs from your RPC procedures for Swagger UI, Prism, and OpenAPI Generator
- 🔄 **Hybrid RPC Modes** - Choose between REST (individual endpoints) or JSON-RPC (single endpoint) style
- 🔌 **WebSocket Support** - Zero-dependency RFC 6455 compliant implementation with built-in pub/sub
- 🔧 **CLI Tools** - Build, develop, and generate clients with the `ultimo` CLI
- 🎯 **Hybrid API Design** - Support both REST endpoints and type-safe RPC procedures
- 🛡️ **Type Safety Everywhere** - From Rust backend to TypeScript frontend
- 🔥 **Developer Experience First** - Ergonomic APIs, helpful errors, minimal boilerplate
- 💪 **Production Ready** - Built-in validation, authentication, rate limiting, CORS

### Coming Soon 🚧

See the [full roadmap](https://docs.ultimo.dev/roadmap) for upcoming features:

- 📡 Streaming & SSE
- 🎫 Session Management
- 🧪 Testing Utilities
- 🌍 Multi-language Client Generation
- And more...

## 📊 Performance

Ultimo delivers exceptional performance, matching industry-leading frameworks:

| Framework   | Throughput       | Avg Latency | vs Python      |
| ----------- | ---------------- | ----------- | -------------- |
| **Ultimo**  | **158k req/sec** | **0.6ms**   | **15x faster** |
| Axum (Rust) | 153k req/sec     | 0.6ms       | 15x faster     |
| Hono (Bun)  | 132k req/sec     | 0.8ms       | 13x faster     |
| Hono (Node) | 62k req/sec      | 1.6ms       | 6x faster      |
| FastAPI     | 10k req/sec      | 9.5ms       | baseline       |

**Zero performance penalty** for automatic RPC generation, OpenAPI docs, and client SDK generation.

## 📚 Documentation

> **📖 [Complete documentation available at docs.ultimo.dev →](https://docs.ultimo.dev)**
>
> Comprehensive guides covering:
>
> - Getting Started & Installation
> - Routing & Middleware
> - RPC System & TypeScript Clients
> - OpenAPI Support
> - Database Integration (SQLx/Diesel)
> - Testing Patterns
> - CLI Tools
> - And more...

## 🎯 Quick Examples

## Quick Start

### Simple REST API

Create a basic REST API with routes and JSON responses:

```rust
use ultimo::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Deserialize)]
struct CreateUserInput {
    name: String,
    email: String,
}

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    let mut app = Ultimo::new();

    // GET /users - List all users
    app.get("/users", |ctx: Context| async move {
        let users = vec![
            User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() },
            User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string() },
        ];
        ctx.json(users).await
    });

    // GET /users/:id - Get user by ID
    app.get("/users/:id", |ctx: Context| async move {
        let id: u32 = ctx.param("id")?.parse()?;
        let user = User {
            id,
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
        };
        ctx.json(user).await
    });

    // POST /users - Create new user
    app.post("/users", |ctx: Context| async move {
        let input: CreateUserInput = ctx.req.json().await?;
        let user = User {
            id: 3,
            name: input.name,
            email: input.email,
        };
        ctx.json(user).await
    });

    println!("🚀 Server running on http://127.0.0.1:3000");
    app.listen("127.0.0.1:3000").await
}
```

Test it:

```bash
# List users
curl http://localhost:3000/users

# Get specific user
curl http://localhost:3000/users/1

# Create user
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Charlie","email":"charlie@example.com"}'
```

### 1. Add Ultimo to your project

```toml
[dependencies]
ultimo = { path = "./ultimo" }
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

### 2. Create your API with RPC

Ultimo supports two RPC modes:

#### REST Mode (Individual Endpoints)

```rust
use ultimo::prelude::*;
use ultimo::rpc::RpcMode;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = Ultimo::new();

    // Create RPC registry in REST mode
    let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);

    // Register query (will use GET)
    rpc.query(
        "getUser",
        |input: GetUserInput| async move {
            Ok(User {
                id: input.id,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            })
        },
        "{ id: number }".to_string(),
        "User".to_string(),
    );

    // Register mutation (will use POST)
    rpc.mutation(
        "createUser",
        |input: CreateUserInput| async move {
            Ok(User { /* ... */ })
        },
        "{ name: string; email: string }".to_string(),
        "User".to_string(),
    );

    // Generate TypeScript client with REST endpoints
    rpc.generate_client_file("../frontend/src/lib/ultimo-client.ts")?;

    // Mount individual endpoints
    app.get("/api/getUser", /* handler */);
    app.post("/api/createUser", /* handler */);

    app.listen("127.0.0.1:3000").await
}
```

#### JSON-RPC Mode (Single Endpoint)

```rust
use ultimo::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = Ultimo::new();

    // Create RPC registry (JSON-RPC is default)
    let rpc = RpcRegistry::new();

    // Register procedures
    rpc.register_with_types(
        "getUser",
        |input: GetUserInput| async move {
            Ok(User {
                id: input.id,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            })
        },
        "{ id: number }".to_string(),
        "User".to_string(),
    );

    // Generate TypeScript client
    rpc.generate_client_file("../frontend/src/lib/ultimo-client.ts")?;

    // Single RPC endpoint
    app.post("/rpc", move |ctx: Context| {
        let rpc = rpc.clone();
        async move {
            let req: RpcRequest = ctx.req.json().await?;
            let result = rpc.call(&req.method, req.params).await?;
            ctx.json(result).await
        }
    });

    app.listen("127.0.0.1:3000").await
}
```

**When to use each mode:**

- **REST Mode**: Public APIs, HTTP caching important, clear URLs in browser DevTools
- **JSON-RPC Mode**: Internal APIs, simple routing, easy request batching

[Learn more about RPC →](https://docs.ultimo.dev/rpc)

### 3. Use the generated TypeScript client

The generated client works the same way regardless of RPC mode:

```typescript
import { UltimoRpcClient } from "./lib/ultimo-client";

// REST mode: client uses GET/POST to /api/getUser, /api/createUser
// JSON-RPC mode: client uses POST to /rpc with method dispatch
const client = new UltimoRpcClient(); // Uses appropriate base URL

// Same API for both modes - fully type-safe!
const user = await client.getUser({ id: 1 });
console.log(user.name); // ✅ TypeScript autocomplete works!

const newUser = await client.createUser({
  name: "Bob",
  email: "bob@example.com",
});
```

### 4. Generate OpenAPI Specification

Automatically generate OpenAPI 3.0 specs from your RPC procedures:

```rust
use ultimo::prelude::*;
use ultimo::rpc::RpcMode;

let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);

// Register procedures
rpc.query("getUser", handler, "{ id: number }", "User");
rpc.mutation("createUser", handler, "{ name: string }", "User");

// Generate OpenAPI spec
let openapi = rpc.generate_openapi("My API", "1.0.0", "/api");
openapi.write_to_file("openapi.json")?;
```

**Use with external tools:**

```bash
# View in Swagger UI
docker run -p 8080:8080 -e SWAGGER_JSON=/openapi.json \
  -v $(pwd)/openapi.json:/openapi.json swaggerapi/swagger-ui

# Run Prism mock server
npx @stoplight/prism-cli mock openapi.json

# Generate clients in any language
npx @openapitools/openapi-generator-cli generate \
  -i openapi.json -g typescript-fetch -o ./client
```

[Learn more about OpenAPI →](https://docs.ultimo.dev/openapi)

## 🔧 CLI Tools

Install the Ultimo CLI:

```bash
cargo install --path ultimo-cli
```

### Generate TypeScript Client

```bash
# Generate client from your Rust backend
ultimo generate --project ./backend --output ./frontend/src/lib/client.ts

# Short form
ultimo generate -p ./backend -o ./frontend/src/client.ts
```

### Coming Soon

```bash
ultimo new my-app --template fullstack  # Create new project
ultimo dev --port 3000                   # Development server with hot reload
ultimo build --profile release           # Production build
```

[Learn more about CLI →](https://docs.ultimo.dev/cli)

## 📦 Installation & Demo

### Quick Demo

Run the included demo script to see everything in action:

```bash
./demo.sh
```

This will:

1. Build the Ultimo CLI
2. Start the backend server (auto-generates TypeScript client)
3. Test RPC endpoints
4. Verify generated client
5. Demonstrate CLI usage

### Manual Installation

```bash
# Install CLI
./install-cli.sh

# Or build manually
cargo build --release --manifest-path ultimo-cli/Cargo.toml

# Verify installation
ultimo --help
```

### Run Examples

```bash
# Backend with auto-generation
cd examples/react-backend
cargo run --release

# Frontend
cd examples/react-app
npm install
npm run dev

# Generate client manually
ultimo generate -p ./examples/react-backend -o /tmp/client.ts
```

## 🔄 Complete Workflow

### Backend (Rust)

```rust
use ultimo::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = Ultimo::new();
    let rpc = RpcRegistry::new();

    // 1. Register RPC procedures with TypeScript types
    rpc.register_with_types(
        "listUsers",
        |_params: ()| async move {
            Ok(json!({"users": [...], "total": 2}))
        },
        "{}".to_string(),
        "{ users: User[]; total: number }".to_string(),
    );

    // 2. Auto-generate TypeScript client on startup
    rpc.generate_client_file("../frontend/src/lib/ultimo-client.ts")?;
    println!("✅ TypeScript client generated");

    // 3. Add RPC endpoint
    app.post("/rpc", move |mut c: Context| {
        let rpc = rpc.clone();
        async move {
            let req: RpcRequest = c.req.json().await?;
            let result = rpc.call(&req.method, req.params).await?;
            c.json(RpcResponse { result })
        }
    });

    app.listen("127.0.0.1:3000").await
}
```

### Frontend (TypeScript/React)

```typescript
// src/lib/ultimo-client.ts - Auto-generated, don't edit!
export class UltimoRpcClient {
  async listUsers(params: {}): Promise<{ users: User[]; total: number }> {
    return this.call("listUsers", params);
  }
}

// src/App.tsx - Use the generated client
import { UltimoRpcClient } from "./lib/ultimo-client";
import { useQuery } from "@tanstack/react-query";

const client = new UltimoRpcClient("/api/rpc");

function UserList() {
  const { data } = useQuery({
    queryKey: ["users"],
    queryFn: () => client.listUsers({}), // ✅ Fully type-safe!
  });

  return (
    <div>
      {data?.users.map((user) => (
        <div key={user.id}>{user.name}</div>
      ))}
    </div>
  );
}
```

### Development Flow

```bash
# Terminal 1: Start backend (auto-generates client)
cd backend
cargo run --release
# ✅ TypeScript client generated

# Terminal 2: Start frontend
cd frontend
npm run dev

# Terminal 3: Regenerate client manually if needed
ultimo generate -p ./backend -o ./frontend/src/lib/client.ts
```

### Benefits

✅ **Single Source of Truth** - Types defined in Rust, automatically propagate to TypeScript  
✅ **No Manual Typing** - TypeScript types generated automatically from Rust  
✅ **Type Safety** - Catch API mismatches at compile time  
✅ **Great DX** - Full IDE autocomplete and type checking  
✅ **Zero Maintenance** - Client updates automatically when backend changes

## Core Philosophy

- **Hybrid API Design**: Support both traditional REST endpoints and RPC-style procedures
- **Type Safety Everywhere**: From Rust backend to TypeScript frontend with automatic type export
- **Developer Experience First**: Ergonomic APIs, helpful error messages, minimal boilerplate
- **Production Ready**: Built-in validation, authentication, rate limiting, file uploads

## Tech Stack Requirements

- **Runtime**: Tokio for async execution
- **HTTP Server**: Hyper for high-performance HTTP handling
- **Serialization**: Serde for JSON handling
- **Validation**: validator crate for request validation
- **File Uploads**: multer for multipart form data
- **Type Export**: ts-rs for TypeScript type generation
- **Logging**: tracing for structured logging
- **Testing**: tokio-test for async testing utilities

## Minimum Supported Rust Version (MSRV)

- Rust 1.75.0 or higher

## Project Structure

```
ultimo-rs/
├── Cargo.toml (workspace)
├── ultimo/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs (public API)
│       ├── app.rs (main application)
│       ├── context.rs (request/response context)
│       ├── error.rs (error handling)
│       ├── response.rs (response builder)
│       ├── router.rs (routing engine)
│       ├── handler.rs (handler traits)
│       ├── middleware.rs (middleware system)
│       ├── validation.rs (validation helpers)
│       ├── upload.rs (file upload handling)
│       ├── guard.rs (authentication guards)
│       └── rpc.rs (RPC system)
└── examples/
    └── basic/
        └── src/main.rs
```

## Feature Requirements

### 1. Error Handling System

Create a comprehensive error system that returns structured JSON responses with proper HTTP status codes. Support validation errors with field-level details, authentication errors, authorization errors, and general HTTP errors.

### 2. Response Builder (via Context)

Provide response methods directly on Context (like Hono's `c.json()`, `c.text()`, `c.html()`). Support for JSON, text, HTML, redirects, custom headers, and status codes. Methods should be chainable where appropriate.

### 3. Request Context

Wrap incoming HTTP requests with a context object that provides:

- Easy access via `c.req.param()` for path parameters
- `c.req.query()` for query parameters
- `c.req.json()`, `c.req.text()`, `c.req.parse_body()` for request bodies
- `c.req.header()` for headers
- `c.set()` / `c.get()` for passing values between middleware
- Response methods: `c.json()`, `c.text()`, `c.html()`, `c.redirect()`
- `c.status()` and `c.header()` for setting response metadata

### 4. Fast Router

Implement efficient path-based routing with support for:

- Static paths (`/users`)
- Path parameters (`/users/:id`)
- Multiple parameters (`/users/:userId/posts/:postId`)
- HTTP method matching (GET, POST, PUT, DELETE, etc.)

### 5. Handler System

Create a trait-based system for request handlers that supports async functions. Handlers receive a Context and return a Result<Response>.

### 6. Middleware Chain

Implement composable middleware that can:

- Execute before and after handlers via `next()` await point
- Access and modify context with `c.set()` / `c.get()`
- Short-circuit by returning early without calling `next()`
- Access response after handler via `await next()` then modify `c.res`
- Include built-in middleware for:
  - Logging (request/response details)
  - CORS (configurable origins, methods, headers)
  - Request timing

### 7. Request Validation

Integrate with the validator crate to provide automatic request body validation with custom error messages. Convert validation failures into structured JSON error responses.

### 8. File Upload Handling

Support multipart form data parsing with:

- Automatic separation of files and text fields
- File metadata (name, size, content type)
- Easy saving to disk
- Type checking helpers (is_image, is_pdf, etc.)

### 9. Authentication Guards

Create a guard system for protecting routes with:

- Bearer token authentication (validate tokens from Authorization header)
- API key authentication (validate X-API-Key header)
- Custom guard implementations via trait
- Composable guards using middleware pattern
  - Multiple guards execute in order (AND logic by default)
  - Early return on first failure
  - Can implement OR logic via custom guard composition

### 10. Rate Limiting

Implement rate limiting middleware with:

- Time-window based limiting (sliding window algorithm)
- Per-client tracking (identified by IP address from socket)
- Configurable limits (requests per window duration)
- In-memory storage with automatic cleanup via background task
- Returns 429 (Too Many Requests) when limit exceeded

### 11. RPC System

Build a type-safe RPC system where:

- Procedures have strongly-typed inputs and outputs
- Automatic JSON serialization/deserialization
- Automatic TypeScript type export (via ts-rs)
- RPC endpoints accessible at `/rpc/{procedure-name}` (POST method)
- TypeScript types exported to `./bindings/` directory
- Support for nested namespaces: `/rpc/{namespace}.{procedure}`

### 12. Main Application

Tie everything together in an Ultimo struct that:

- Manages routes and middleware
- Starts an HTTP server
- Handles incoming requests
- Applies middleware chain
- Routes to appropriate handlers
- Returns structured error responses on failures

## API Design Patterns

### Simple Route Definition (Hono-style)

```rust
use ultimo::prelude::*;

// GET request with JSON response
app.get("/users", |c| async move {
    c.json(json!({"users": ["Alice", "Bob"]}))
});

// Path parameters
app.get("/users/:id", |c| async move {
    let id = c.req.param("id")?;
    c.json(json!({"id": id}))
});

// Query parameters
app.get("/search", |c| async move {
    let q = c.req.query("q")?;
    c.json(json!({"query": q}))
});
```

### Request Body Parsing

```rust
#[derive(Deserialize, Validate, TS)]
#[ts(export)]
struct CreateUser {
    #[validate(length(min = 3, max = 50))]
    name: String,
    #[validate(email)]
    email: String,
}

app.post("/users", |mut c| async move {
    // Parse and validate in one step
    let input: CreateUser = c.req.json().await?;
    validate(&input)?;

    // Use the validated data
    let user = create_user(input);
    c.status(201);
    c.json(user)
});
```

### Middleware Usage

```rust
use ultimo::middleware::{logger, cors};

// Global middleware
app.use_middleware(logger());
app.use_middleware(cors::new()
    .allow_origin("https://example.com")
    .allow_methods(vec!["GET", "POST"]));

// Custom middleware
app.use_middleware(|c, next| async move {
    c.set("request_id", generate_id());
    let start = Instant::now();

    next().await?;

    let duration = start.elapsed();
    c.res.headers.append("X-Response-Time", duration.as_millis().to_string());
    Ok(())
});
```

### Authentication Guards

```rust
use ultimo::guards::{bearer_auth, api_key_auth};

// Protect specific routes
let auth = bearer_auth(vec!["secret_token_123"]);

app.get("/protected", auth, |c| async move {
    c.json(json!({"message": "You are authenticated!"}))
});

// Or use as middleware for a group
app.use_middleware(api_key_auth(vec!["api_key_123"]));
```

### File Uploads

```rust
app.post("/upload", |mut c| async move {
    let form_data = c.req.parse_body().await?;

    for (field_name, file) in form_data.files {
        if file.is_image() {
            let path = format!("./uploads/{}", file.name);
            file.save(&path).await?;
        }
    }

    c.json(json!({"uploaded": form_data.files.len()}))
});
```

### RPC Procedures

```rust
#[derive(Deserialize, TS)]
#[ts(export)]
struct CalculateInput {
    a: i32,
    b: i32,
}

#[derive(Serialize, TS)]
#[ts(export)]
struct CalculateOutput {
    result: i32,
}

app.rpc("calculate", |input: CalculateInput| async move {
    Ok(CalculateOutput {
        result: input.a + input.b,
    })
});

// Access at: POST /rpc/calculate
// TypeScript types auto-generated in ./bindings/
```

### Sharing Data Between Middleware

```rust
// Set data in middleware
app.use_middleware(|c, next| async move {
    let user = authenticate(&c).await?;
    c.set("user", user);
    next().await
});

// Access in handler
app.get("/profile", |c| async move {
    let user: User = c.get("user")?;
    c.json(user)
});
```

## Expected Behavior

### Error Responses

All errors return JSON with structure:

```json
{
  "error": "Error Type",
  "message": "Human readable message",
  "details": [] // Optional, for validation errors
}
```

### Path Parameter Matching

- `/users/:id` matches `/users/123` → `{id: "123"}`
- `/users/:userId/posts/:postId` matches `/users/1/posts/2` → `{userId: "1", postId: "2"}`

### Middleware Execution

Middleware executes in order, each can call next() to continue the chain or return early to short-circuit.

### Rate Limiting

Track requests per client within a time window. Reject requests exceeding the limit with 429 (Too Many Requests) status code.

### TypeScript Export

Types decorated with `#[ts(export)]` automatically generate `.ts` files in the `./bindings/` directory for frontend use.

## Quality Requirements

- **Type Safety**: Leverage Rust's type system fully
- **Async Throughout**: All I/O operations must be async
- **Error Handling**: Use Result types, avoid panics
- **Documentation**: Public APIs need doc comments
- **Testing**: Include unit tests for core logic
- **Performance**: Efficient routing and minimal allocations

## Success Criteria

When complete, this code should work:

```rust
#[tokio::main]
async fn main() -> ultimo::Result<()> {
    let mut app = Ultimo::new();

    app.use_middleware(ultimo::middleware::logger());

    app.get("/", |ctx| async move {
        Ok(Context::json(json!({"message": "Hello Ultimo!"}))?)
    });

    app.post("/users", |mut ctx| async move {
        let input: CreateUser = ctx.req.json().await?;
        validate(&input)?;
        Ok(Context::json(create_user(input))?)
    });

    app.listen("127.0.0.1:3000").await
}
```

And support curl commands like:

```bash
curl http://localhost:3000/
curl -X POST http://localhost:3000/users -H "Content-Type: application/json" -d '{"name":"Alice"}'
curl http://localhost:3000/protected -H "Authorization: Bearer token123"
```

## Implementation Guidelines

### Phase 1: Core Types

1. Error types and Result alias
2. Response builder (used internally by Context)
3. Request wrapper (HonoRequest-style)

### Phase 2: Context & Router

4. Context with request/response methods (c.json(), c.text(), etc.)
5. Router with path matching and parameter extraction
6. Handler trait for async functions

### Phase 3: Middleware & App

7. Middleware trait and chain execution
8. Main Ultimo app struct
9. HTTP server integration with Hyper

### Phase 4: Features

10. Built-in middleware (logger, CORS, rate limiting)
11. Validation helpers
12. File upload support
13. Authentication guards
14. RPC system with TypeScript export

### Phase 5: Polish

15. Comprehensive examples
16. Documentation with examples
17. Unit and integration tests

## Design Principles

- **Hono.js-inspired API**: Method names and patterns match Hono.js (c.json(), c.req.param(), etc.)
- **Type Safety**: Leverage Rust's type system, use generics for flexibility
- **Async First**: All I/O operations are async, handlers return futures
- **Zero Panics**: Use Result types throughout, no unwrap() in library code
- **Composability**: Middleware and guards can be chained and combined
- **Error Messages**: Provide helpful, actionable error messages
- **Performance**: Efficient routing, minimal allocations, lazy evaluation where possible

Focus on clean abstractions and excellent developer experience. The framework should feel natural to Rust developers while being immediately familiar to those coming from Hono.js or Express.

## Development

This project uses [Moonrepo](https://moonrepo.dev/) for monorepo management. See [MOONREPO.md](./docs/MOONREPO.md) for detailed commands.

**Quick Start:**

```bash
# Install Moonrepo
curl -fsSL https://moonrepo.dev/install/moon.sh | bash

# Install git hooks (recommended)
./scripts/install-hooks.sh

# Build core framework
moon run ultimo:build

# Run all tests
moon run :test

# Check code quality
moon run ultimo:clippy

# Build documentation
moon run docs-site:build
```

### Git Hooks

We provide pre-commit and pre-push hooks to ensure code quality:

```bash
# Install hooks
./scripts/install-hooks.sh
```

**What the hooks do:**

- **pre-commit**: Checks code formatting with `cargo fmt`
- **pre-push**: Runs tests, clippy, and verifies coverage threshold (60%)

### Testing & Coverage

The Ultimo framework maintains **high test coverage standards** with a custom coverage tool built for **security and transparency**.

#### Why Custom Coverage Tool?

We built **ultimo-coverage** instead of using external tools because:

- 🔒 **Security First**: External tools with low GitHub adoption (< 500 stars) posed trust concerns
- 🎯 **Project-Only Coverage**: Filters out dependency code for accurate metrics
- 🔍 **Transparent**: Simple 200-line Rust tool with only 3 dependencies
- ⚡ **Fast**: Uses Rust's built-in LLVM instrumentation directly
- 🛠️ **Maintainable**: Full control over coverage reporting and thresholds

#### Quick Start

```bash
# Run tests with coverage report
cargo coverage

# Or use make
make coverage

# View HTML report (modern UI with Tailwind CSS)
open target/coverage/html/index.html
```

#### Current Coverage Stats

**Overall: 63.58%** ✅ (exceeds 60% minimum threshold)

| Module         | Coverage | Status        |
| -------------- | -------- | ------------- |
| database/error | 100%     | ✅ Excellent  |
| validation     | 95.12%   | ✅ Excellent  |
| response       | 92.35%   | ✅ Excellent  |
| rpc            | 85.07%   | ✅ Excellent  |
| router         | 82.41%   | ✅ Excellent  |
| openapi        | 76.21%   | ✅ Good       |
| context        | 40.18%   | ⚠️ Improved   |
| app            | 25.62%   | ⚠️ Needs work |

**Test Stats:**

- 89 unit tests across all modules
- All critical paths tested
- Comprehensive middleware, RPC, and OpenAPI coverage

#### Coverage Standards

- ✅ **Minimum 60% overall coverage required**
- ✅ **Core modules (router, RPC, OpenAPI) target 70%+**
- ✅ **All new features must include tests**
- ✅ **CI fails if coverage drops below threshold**

#### Coverage Tool Details

**ultimo-coverage** is our custom LLVM-based coverage tool:

```bash
# How it works
cd coverage-tool
cargo build --release

# The tool:
# 1. Instruments code with LLVM coverage
# 2. Runs tests and collects profiling data
# 3. Merges .profraw files with llvm-profdata
# 4. Generates reports with llvm-cov
# 5. Filters out dependency code (.cargo/registry, .rustup)
```

**Why it's trustworthy:**

- ✅ Only 3 dependencies (serde, serde_json, walkdir)
- ✅ Uses Rust's official LLVM tools (bundled with rustc)
- ✅ Auditable source code (~200 lines)
- ✅ No network access or external data collection
- ✅ Generates local HTML/JSON reports only

**Key Features:**

- 📊 HTML report with line-by-line coverage
- 📈 JSON output for CI integration
- 🎯 Filters dependency coverage automatically
- 🚀 Fast incremental builds
- 🔄 Cross-platform (macOS, Linux, Windows)
- 🎨 **Modern UI with Tailwind CSS** - Beautiful, color-coded coverage visualization

#### For Contributors

Run tests before submitting PRs:

```bash
# Run all tests
cargo test --lib

# Check coverage
cargo coverage

# Ensure coverage meets standards
# Overall must be ≥60%, new code should increase coverage
```

**Project Structure:**

- `ultimo/` - Core framework
- `ultimo-cli/` - CLI tool for project scaffolding and TypeScript generation
- `examples/` - Example projects demonstrating features
- `docs-site/` - Documentation website (Vocs)
- `scripts/` - Development and testing scripts
