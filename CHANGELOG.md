# Changelog

All notable changes to the Ultimo framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1] - 2026-06-15

### Added

- `ultimo dev` — hot-reload development server with file watching

## [0.5.0] - 2026-06-09

### Added

- **TypeScript client type derivation** (`client-gen` feature): RPC client types are now derived from your Rust types via `ts-rs`. `#[derive(TS)]` on your input/output structs and the generated client emits real `type X = {...}` declarations — no more hand-written type strings or dangling references. `ts_rs::TS` is re-exported as `ultimo::rpc::TS`. (#107)

### Changed

- **BREAKING:** `RpcRegistry::query` / `mutation` now take `(name, handler)` and derive their TypeScript input/output types from the Rust types (bounds `I: TS, O: TS`, gated behind `client-gen`). The previous string-typed signatures are preserved as `query_with_types` / `mutation_with_types`. (#107)
- `ts-rs` is now an optional dependency behind `client-gen` (previously an unused hard dependency) and was upgraded 8.1 → 12. Default builds no longer pull it. (#107)
- Removed the hardcoded `User` interface that was previously injected into every generated client. (#107)

## [0.4.1] - 2026-06-09

### Added

- **Static file serving** (`static-files` feature): `serve_static` serves assets from disk with automatic `Content-Type`, `ETag`, and `304 Not Modified`; `serve_spa` adds Single Page Application fallback routing; path traversal is blocked at the filesystem level. Adds catch-all (`*name`) wildcard segments to the router. (#101)
- **Response compression** (`compression` feature): automatic gzip/brotli middleware (brotli preferred), pure Rust with no C dependencies, configurable via the `Compression` builder. Skips binary content types, already-encoded responses, and small bodies; always sets `Vary: Accept-Encoding`. (#101)

### Changed

- Crate install snippets (`ultimo = "…"`) across the README and docs pages are now derived from the workspace version and enforced by the `version-sync` CI gate, eliminating version drift. (#102)

## [0.4.0] - 2026-06-08

The **Security & Performance** milestone.

### Added

- **JWT authentication** (`jwt` feature): HS256 verify + sign, algorithm pinned (`alg: none` rejected), `exp` validated, claims on `Context`. (#84)
- **API-key authentication** (`api-key` feature): pluggable `ApiKeyStore` + built-in `StaticKeys` (SHA-256 hashed, constant-time), resolving to an identity (id + scopes). (#86)
- **Authorization guards**: unified `auth::Principal`; `ctx.require_auth` / `require_scope` / `require_any_scope` / `require_all_scopes`, fed by both JWT and API-key. (#87)
- **Security-headers middleware** (HSTS, X-Frame-Options, X-Content-Type-Options, Referrer-Policy, Permissions-Policy; opt-in CSP). (#56, #71)
- **CSRF protection** (`csrf` feature): double-submit cookie, constant-time compare. (#57, #80)
- **Request body-size limit** (`max_body_size`) — 413 on oversize, without buffering the whole body. (#58, #72)
- **Real client IP** (`ctx.client_ip`), trusted-proxy aware via `trust_proxy`. (#65, #76)
- `#![forbid(unsafe_code)]` (100% safe Rust) + `SECURITY.md` disclosure policy. (#69)

### Performance

- **O(1) static route lookup** via a hash index — was O(N) in route count. (#90)
- **Framework-overhead benchmark suite** (criterion) + `BENCHMARKS.md` methodology + an advisory CI regression check. (#88, #13)

### Docs

- New `/performance`, `/jwt`, `/api-keys`, and `/authorization` pages; an "Authentication" docs group; and an honest "Secure & Fast" website (removed unsubstantiated benchmark numbers).

## [0.3.0] - 2026-06-04

### Added

- **Cookie helper** (`ultimo::cookie`, core): `Cookie`/`CookieOptions`/`SameSite`,
  RFC 6265 parsing + `Set-Cookie` formatting (with header-injection guards), and
  `ctx.cookie`/`cookies`/`set_cookie`/`remove_cookie`.
- **Session management** (`session` feature, #3): `SessionStore` trait +
  `MemoryStore`, `Session` via `ctx.session()`, and secure session middleware
  (256-bit ids, HttpOnly/Secure/SameSite defaults, anti session-fixation,
  anti-DoS, server-side expiry), configurable via `SessionConfig`. Redis/SQL
  stores and CSRF are planned follow-ups. See `docs/sessions.md`.
- `examples/session-auth` — a full-stack login/logout demo of the session
  feature (Rust backend serving an HTML+JS frontend).

- **Testing utilities** (`testing` feature, #4): in-process `TestClient`, fluent
  request builder, `TestResponse` with assertion helpers, `assert_json_eq!` /
  `assert_status!` macros, middleware test helpers (`test_context`,
  `run_middleware`), a database transaction/rollback helper
  (`with_test_transaction`), and `load_fixture` / `Fixture`. See
  `docs/testing-utilities.md`.
- `Ultimo::oneshot` — dispatch a fully-buffered request through the app
  in-process (no socket); the seam the testing utilities build on.
- CI: a `cargo-audit` (RUSTSEC) job and an MSRV job; `Cargo.lock` is now
  committed.
- `Request::raw_body()` — raw request body bytes; the body is buffered and
  cached so `json`/`text`/`bytes`/`raw_body` can be called repeatedly. (#21)

### Fixed

- Router precedence: static routes now take priority over parameterized ones
  regardless of registration order (most-specific match wins), so e.g.
  `/users/me` is no longer shadowed by a `/users/:id` registered before it. (#22)

### Changed

- **MSRV raised to 1.86.0** (a transitive dependency requires `edition2024`).
  This is a breaking change for consumers on older Rust — the next release is
  planned as **0.3.0**.
- Hardened dependency floors (`bytes >= 1.11.1`, `diesel >= 2.3.8`) to exclude
  versions with known RUSTSEC advisories.

## [0.2.1] - 2026-01-04

### Fixed

- Updated WebSocket pubsub benchmark to match new ChannelManager API

### Coming Soon

- Server-Sent Events (SSE)
- Session management
- Testing utilities
- Multi-language client generation
- Per-message deflate compression (RFC 7692)

## [0.2.0] - 2026-01-04

### Added

**WebSocket Support (Complete)** 🔌

- Zero-dependency RFC 6455 compliant WebSocket implementation
- Built on hyper's upgrade mechanism (no tokio-tungstenite required)
- Type-safe WebSocketHandler trait with typed context data
- Built-in pub/sub system (ChannelManager) for topic-based messaging
- Seamless router integration with `app.websocket()` method
- Router optimization: Migrated to Radix Tree for O(L) lookups
- 279 comprehensive tests (128 unit, 151 integration)
- Production-ready features:
  - **Configuration System** (`WebSocketConfig`) with size limits, timeouts, and buffer sizes
  - **Message Fragmentation** for large payloads with automatic reassembly
  - **Automatic Ping/Pong** heartbeat with configurable intervals and timeout detection
  - **Graceful Shutdown** with `broadcast_all()` and proper close handshakes
  - **Backpressure Handling** with bounded channels, `on_drain()` callback, and capacity tracking
- Two working examples:
  - Simple HTML/JS chat application
  - Modern React + TypeScript chat with shadcn/ui
- Complete documentation:
  - WEBSOCKET_DESIGN.md - Architecture and design decisions
  - WEBSOCKET_TESTING.md - Testing strategy and coverage
  - Example READMEs with setup instructions

**Core Features**

- Frame codec supporting all opcodes (text, binary, ping, pong, close, continuation)
- Frame masking/unmasking (client frames must be masked per RFC 6455)
- Control frame handling (close, ping, pong)
- Automatic message fragmentation for large payloads (>max_frame_size)
- Fragment reassembly with `FragmentAccumulator`
- Subscribe/unsubscribe to topics
- Publish messages to all topic subscribers with backpressure handling
- Automatic cleanup on disconnect
- Connection lifecycle callbacks (on_open, on_message, on_close, on_drain)
- Type-safe context data per connection (`WebSocket<T>`)
- JSON message helpers (send_json, recv_json)

**Production Features**

- Configurable size limits (max_message_size: 10MB, max_frame_size: 1MB)
- Bounded channels with configurable buffer (default 1024)
- Automatic ping/pong heartbeat (configurable interval, default 30s)
- Timeout detection for unresponsive clients (default 10s)
- Backpressure notifications via `on_drain()` callback
- Capacity tracking: `capacity()`, `max_capacity()`, `has_capacity()`
- Graceful shutdown with `broadcast_all()` for server-wide notifications
- Custom close frames with reason codes

**Performance**

- Zero additional dependencies (uses existing hyper, tokio, bytes)
- Efficient memory usage with BytesMut for frame parsing
- O(L) router lookups with Radix Tree optimization

### Changed

- Router: Optimized from linear scan to Radix Tree (Prefix Tree) for better performance
- Updated README.md to list WebSocket as available feature

### Fixed

- Duplicate message broadcasting in chat examples
- Port conflicts in examples (standardized to port 4000)

## [0.1.0] - 2025-11-21

### Core Features

**Framework**

- ⚡ High-performance HTTP server built on Hyper
- 🎯 Type-safe routing with path parameters
- 🔧 Composable middleware system (CORS, Logger, PoweredBy, Custom)
- 📊 Built-in RPC support (REST & JSON-RPC modes)
- 📝 OpenAPI 3.0 specification generation
- ✨ Automatic TypeScript client generation
- ✅ Request validation with detailed errors
- 🛡️ Comprehensive error handling

**Developer Experience**

- 🧪 70.7% test coverage (124 tests)
- 📈 Custom coverage tool with modern HTML reports
- 🔍 Git hooks for code quality (pre-commit, pre-push)
- 📚 Complete documentation and examples
- 🛠️ CLI tool for client generation
- 📦 Monorepo management with Moonrepo

**Examples**

- Basic REST API
- Database integration (SQLx & Diesel)
- OpenAPI documentation
- React full-stack applications
- RPC modes demonstration
- Benchmark comparisons

### Technical Details

- **MSRV**: Rust 1.75.0
- **Runtime**: Tokio (async)
- **HTTP**: Hyper 1.x
- **Performance**: 152k+ req/sec (matches Axum)

---

**Initial Release** - Complete type-safe web framework with automatic client generation and comprehensive testing.
