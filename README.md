<div align="center">
  <img src="docs-site/docs/public/logo.svg" alt="Ultimo Logo" width="200" />
  <h1>Ultimo</h1>
  <p><strong>Type-safe web framework with automatic TypeScript client generation</strong></p>

  <p>
    <a href="https://crates.io/crates/ultimo"><img src="https://img.shields.io/crates/v/ultimo.svg?style=flat-square" alt="Crates.io" /></a>
    <a href="https://docs.rs/ultimo"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="Documentation" /></a>
    <a href="https://github.com/ultimo-rs/ultimo/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square" alt="License" /></a>
    <a href="https://github.com/ultimo-rs/ultimo/actions"><img src="https://img.shields.io/github/actions/workflow/status/ultimo-rs/ultimo/ci.yml?branch=main&style=flat-square" alt="Build Status" /></a>
    <a href="https://github.com/ultimo-rs/ultimo/blob/main/SECURITY.md"><img src="https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat-square" alt="Unsafe forbidden" /></a>
    <a href="https://deps.rs/repo/github/ultimo-rs/ultimo"><img src="https://deps.rs/repo/github/ultimo-rs/ultimo/status.svg?style=flat-square" alt="Dependency status" /></a>
    <img src="https://img.shields.io/badge/MSRV-1.86-blue.svg?style=flat-square" alt="MSRV 1.86" />
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

**Ultimo** is a modern Rust web framework built on **Hyper + Tokio**:
secure-by-default, fast, and type-safe end to end — with **automatic TypeScript
client generation** from your Rust API. REST and JSON-RPC live in one app, and
the framework is 100% safe Rust (`#![forbid(unsafe_code)]`).

## Why Ultimo

- 🚀 **Automatic TypeScript clients** — define your API in Rust, get a fully typed TS client generated for you.
- 🔄 **REST + JSON-RPC 2.0 in one app** — plain HTTP routes and RPC procedures side by side, with batch requests and notifications.
- 🔌 **WebSockets** — RFC 6455 with a built-in pub/sub system (zero extra deps).
- 🔐 **Auth, built in** — JWT and API-key middleware plus scope-based [authorization guards](https://docs.ultimo.dev/authorization).
- 🛡️ **Secure by default** — 100% safe Rust, secure sessions/cookies, CSRF, security-headers middleware, request body-size limits, and supply-chain CI.
- ⚡ **Fast** — native Rust on the Hyper + Tokio core, O(1) constant-time routing, benchmarks regression-guarded in CI ([details](https://docs.ultimo.dev/performance)).
- 🗄️ **Databases** — first-class SQLx and Diesel integration (PostgreSQL / MySQL / SQLite).
- 🧪 **Testing utilities** — in-process `TestClient`, response assertions, and fixtures.

## Quick start

```toml
[dependencies]
ultimo = "0.5"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

```rust
use ultimo::prelude::*;

#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
}

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    let mut app = Ultimo::new();

    app.get("/users/:id", |ctx: Context| async move {
        let id: u32 = ctx
            .req
            .param("id")?
            .parse()
            .map_err(|_| UltimoError::BadRequest("invalid id".into()))?;
        ctx.json(User { id, name: format!("User {id}") }).await
    });

    println!("→ http://127.0.0.1:3000");
    app.listen("127.0.0.1:3000").await
}
```

> **MSRV:** Rust 1.86. Everything beyond the core is opt-in via Cargo features (see below).

## Type-safe clients

Ultimo's headline feature: define an API once in Rust and generate a typed
TypeScript client — no hand-written types, no drift.

```bash
# Generate a TypeScript client from your RPC definitions
cargo run -p ultimo-cli -- generate --project ./backend --output ./client
```

```typescript
// Generated, fully typed — autocomplete + compile-time checks
const user = await client.getUser({ id: 1 });
console.log(user.name);
```

See [TypeScript Clients](https://docs.ultimo.dev/typescript) for the full workflow.

## Feature flags

Everything is opt-in (`default = []`):

| Feature                                              | What it enables                                                   |
| ---------------------------------------------------- | ----------------------------------------------------------------- |
| `websocket`                                          | RFC 6455 WebSocket support + pub/sub                              |
| `session`                                            | Cookie-based session management                                   |
| `jwt`                                                | JWT authentication middleware (HS256)                             |
| `api-key`                                            | API-key authentication with a pluggable store                     |
| `csrf`                                               | CSRF protection (double-submit cookie)                            |
| `static-files`                                       | Static file serving + SPA fallback (`serve_static`, `serve_spa`)  |
| `compression`                                        | Automatic gzip/brotli response compression (pure Rust, no C deps) |
| `client-gen`                                         | Derive RPC client TypeScript types from Rust types (via `ts-rs`)  |
| `testing`                                            | In-process `TestClient`, assertions, fixtures                     |
| `test-helpers`                                       | WebSocket test helpers (for integration tests)                    |
| `sqlx-postgres` · `sqlx-mysql` · `sqlx-sqlite`       | SQLx integration per backend                                      |
| `diesel-postgres` · `diesel-mysql` · `diesel-sqlite` | Diesel integration per backend                                    |

```toml
ultimo = { version = "0.5", features = ["websocket", "jwt", "sqlx-postgres"] }
```

## CLI

```bash
cargo install ultimo-cli   # installs the `ultimo` binary

ultimo new my-app --template fullstack                 # scaffold a new project
ultimo generate --project ./backend --output ./client  # generate the TypeScript client
```

> `ultimo dev` and `ultimo build` are **not implemented yet** — use `cargo run` and
> `cargo build --release` for now. See the [roadmap](https://docs.ultimo.dev/roadmap).

## Documentation

Full guides at **[docs.ultimo.dev](https://docs.ultimo.dev)** — getting started,
routing, middleware, RPC + TypeScript clients, OpenAPI, sessions, authentication,
WebSockets, database integration, testing, and [performance](https://docs.ultimo.dev/performance).

## Examples

Runnable examples live in [`examples/`](https://github.com/ultimo-rs/ultimo/tree/main/examples)
— including `session-auth` and `jwt-auth` full-stack demos. Run one with:

```bash
cargo run -p jwt-auth-example
```

## Contributing

Issues and PRs welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) and the
[roadmap](https://docs.ultimo.dev/roadmap). Security policy: [SECURITY.md](SECURITY.md).

## License

MIT © Ultimo Contributors. See [LICENSE](LICENSE).
