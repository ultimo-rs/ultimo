# Changelog

All notable changes to the Ultimo framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Coming Soon

- Server-Sent Events (SSE)
- Session management
- Testing utilities
- Multi-language client generation
- WebSocket Phase 2 (compression, backpressure, advanced features)

## [0.2.0] - 2025-12-14

### Added

**WebSocket Support (Phase 1)** 🔌

- Zero-dependency RFC 6455 compliant WebSocket implementation
- Built on hyper's upgrade mechanism (no tokio-tungstenite required)
- Type-safe WebSocketHandler trait with typed context data
- Built-in pub/sub system (ChannelManager) for topic-based messaging
- Seamless router integration with `app.websocket()` method
- Router optimization: Migrated to Radix Tree for O(L) lookups
- 93 comprehensive tests (21 unit, 9 integration, 12 property-based, 5 router, 14 error, 11 concurrency, 18 edge cases)
- Two working examples:
  - Simple HTML/JS chat application
  - Modern React + TypeScript chat with shadcn/ui
- Complete documentation:
  - WEBSOCKET_DESIGN.md - Architecture and design decisions
  - WEBSOCKET_TESTING.md - Testing strategy and coverage
  - Example READMEs with setup instructions

**Features**

- Frame codec supporting all opcodes (text, binary, ping, pong, close, continuation)
- Frame masking/unmasking (client frames must be masked per RFC 6455)
- Control frame handling (close, ping, pong)
- Message fragmentation support
- Subscribe/unsubscribe to topics
- Publish messages to all topic subscribers
- Automatic cleanup on disconnect
- Connection lifecycle callbacks (on_open, on_message, on_close)
- Type-safe context data per connection (`WebSocket<T>`)
- JSON message helpers (send_json, recv_json)

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
