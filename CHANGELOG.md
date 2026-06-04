# Changelog

All notable changes to the Ultimo framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Cookie helper** (`ultimo::cookie`, core): `Cookie`/`CookieOptions`/`SameSite`,
  RFC 6265 parsing + `Set-Cookie` formatting (with header-injection guards), and
  `ctx.cookie`/`cookies`/`set_cookie`/`remove_cookie`.
- **Session management** (`session` feature, #3): `SessionStore` trait +
  `MemoryStore`, `Session` via `ctx.session()`, and secure session middleware
  (256-bit ids, HttpOnly/Secure/SameSite defaults, anti session-fixation,
  anti-DoS, server-side expiry), configurable via `SessionConfig`. Redis/SQL
  stores and CSRF are planned follow-ups. See `docs/sessions.md`.

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
