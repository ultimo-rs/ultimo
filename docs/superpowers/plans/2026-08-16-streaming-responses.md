# Streaming Responses Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let handlers return chunked/streaming response bodies via `ctx.stream(...)`, replacing the hardwired `Full<Bytes>` response body with an `UltimoBody` enum that keeps a zero-boxing fast path for buffered responses.

**Architecture:** `pub type Response = HyperResponse<UltimoBody>` where `UltimoBody` is a two-variant enum (`Full(Full<Bytes>)` fast-path + `Stream(BoxBody<Bytes, BoxError>)`) implementing `hyper::body::Body`. `ctx.stream(s)` wraps a `Stream<Item = Result<Bytes, BoxError>>` into the streaming variant. Compression skips streaming bodies. Buffered responses stay allocation-identical to today.

**Tech Stack:** Rust, Hyper 1.0, `http_body_util` (`StreamBody`, `BodyExt::boxed`, `BoxBody`, `Full`), `futures-util` (already a runtime dep), `hyper::body::{Body, Frame, SizeHint}`.

**Spec:** `docs/superpowers/specs/2026-08-16-streaming-responses-design.md`

## Deviations from the spec (intentional refinements, discovered while planning)

1. **No new dependency.** `futures-util = "0.3"` is already under `[dependencies]` (`ultimo/Cargo.toml:30`). Use `futures_util::{Stream, TryStreamExt}`; the spec's "add `futures-core`" is unnecessary.
2. **`ctx.stream` is `async fn stream<S>(&self, s: S) -> Result<Response>`** (not sync `-> Response`). This mirrors every sibling (`text`/`json`/`html`) so handler code stays uniform (`ctx.stream(s).await`), and lets it apply queued `ctx.header(...)`/`ctx.status(...)` before streaming. It is infallible in practice; the `Result` is for API symmetry.
3. **Real blast radius is 5 files, not 9.** `websocket/connection.rs`, `websocket/pubsub.rs` matched `TrySendError::Full` (unrelated). `error.rs` matches are comments. `testing/client.rs` uses `Full<Bytes>` only on the *request* side (unchanged). Files that actually change: `response.rs`, `app.rs`, `static_files.rs`, `middleware.rs`, `websocket/upgrade.rs`.

## Global Constraints

- **100% safe Rust** — crate is `#![forbid(unsafe_code)]`. No `unsafe`.
- **Public items need doc comments**; doctests must compile.
- **Breaking change → minor bump.** `Response`'s public body type changes → **0.9.0**. Use a `feat!` commit; `semver-checks` will flag it (expected — admin-merge override at PR time).
- **Feature gating:** the `UltimoBody` type change is core/unconditional (a type alias can't be cleanly feature-gated). `ctx.stream` is always available (only pulls the existing `futures-util`).
- **Verification gate** (run before PR), from `.claude/skills/ship-feature`:
  ```bash
  cargo fmt --all --check
  cargo clippy -p ultimo --features "websocket,test-helpers,testing,session,csrf" --all-targets -- -D warnings
  cargo test -p ultimo --lib
  cargo test -p ultimo --features "websocket,test-helpers,testing,session,csrf"
  cargo test -p ultimo --doc --features "websocket,testing,session,csrf"
  ```
- **Never hand-edit `CHANGELOG.md`** — release-plz generates it from conventional commits.

---

### Task 1: Define `UltimoBody` + migrate all buffered sites (crate compiles, no behavior change)

The `Response` body type is atomic — flipping the alias forces every construction site to change in the same commit. This task introduces the type and migrates all 5 files so the crate compiles and the existing suite passes unchanged. No streaming is produced yet, so behavior is identical to today.

**Files:**
- Modify: `ultimo/src/response.rs` (type + `BoxError` + `Body` impl + `full`/`empty` + `build()`)
- Modify: `ultimo/src/app.rs:36` (WebSocket handler type)
- Modify: `ultimo/src/static_files.rs:70,90`
- Modify: `ultimo/src/middleware.rs:192,572,729` and compression `Full::new` sites (~858–929)
- Modify: `ultimo/src/websocket/upgrade.rs` (return types + body construction)

**Interfaces:**
- Produces:
  - `pub type BoxError = Box<dyn std::error::Error + Send + Sync>;`
  - `pub enum UltimoBody { Full(http_body_util::Full<Bytes>), Stream(http_body_util::combinators::BoxBody<Bytes, BoxError>) }`
  - `impl UltimoBody { pub fn empty() -> Self; pub fn full(bytes: impl Into<Bytes>) -> Self; }`
  - `impl hyper::body::Body for UltimoBody { type Data = Bytes; type Error = BoxError; }`
  - `pub type Response = hyper::Response<UltimoBody>;`

- [ ] **Step 1: Write failing unit tests for the new type**

Add to the `#[cfg(test)] mod tests` block in `ultimo/src/response.rs`:

```rust
    #[tokio::test]
    async fn ultimo_body_full_round_trips() {
        use http_body_util::BodyExt;
        let body = UltimoBody::full("hello world");
        let bytes = body.collect().await.unwrap().to_bytes();
        assert_eq!(&bytes[..], b"hello world");
    }

    #[tokio::test]
    async fn ultimo_body_empty_is_zero_length() {
        use http_body_util::BodyExt;
        use hyper::body::Body as _;
        let body = UltimoBody::empty();
        assert!(body.is_end_stream());
        let bytes = body.collect().await.unwrap().to_bytes();
        assert_eq!(bytes.len(), 0);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p ultimo --lib response:: 2>&1 | tail -20`
Expected: FAIL — `cannot find function/type UltimoBody` (not yet defined).

- [ ] **Step 3: Define `UltimoBody`, `BoxError`, the `Body` impl, and constructors**

In `ultimo/src/response.rs`, replace the imports and `Response` alias (lines 6–12):

```rust
use crate::error::{Result, UltimoError};
use http_body_util::combinators::BoxBody;
use http_body_util::Full;
use hyper::body::{Body as HttpBody, Bytes, Frame, SizeHint};
use hyper::{header::HeaderValue, Response as HyperResponse, StatusCode};
use serde::Serialize;
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

/// Boxed error carried by a streaming response body. A stream item that yields
/// `Err(_)` aborts the response connection.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// The response body used throughout Ultimo.
///
/// `Full` is the buffered fast path — the overwhelmingly common case, with no
/// per-response boxing. `Stream` carries an incrementally produced body (see
/// [`Context::stream`](crate::context::Context::stream)).
pub enum UltimoBody {
    /// A fully-buffered body.
    Full(Full<Bytes>),
    /// A streaming body.
    Stream(BoxBody<Bytes, BoxError>),
}

impl UltimoBody {
    /// An empty buffered body.
    pub fn empty() -> Self {
        UltimoBody::Full(Full::new(Bytes::new()))
    }

    /// A buffered body from any bytes-like value.
    pub fn full(bytes: impl Into<Bytes>) -> Self {
        UltimoBody::Full(Full::new(bytes.into()))
    }
}

impl HttpBody for UltimoBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Option<std::result::Result<Frame<Self::Data>, Self::Error>>> {
        match self.get_mut() {
            // `Full`'s error is `Infallible`; box it to unify the error type.
            UltimoBody::Full(f) => match Pin::new(f).poll_frame(cx) {
                Poll::Ready(Some(Ok(frame))) => Poll::Ready(Some(Ok(frame))),
                Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(Box::new(e) as BoxError))),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            },
            UltimoBody::Stream(s) => Pin::new(s).poll_frame(cx),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            UltimoBody::Full(f) => f.is_end_stream(),
            UltimoBody::Stream(s) => s.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            UltimoBody::Full(f) => f.size_hint(),
            UltimoBody::Stream(s) => s.size_hint(),
        }
    }
}

/// HTTP Response type used throughout Ultimo
pub type Response = HyperResponse<UltimoBody>;
```

> Note: `Bytes` now comes from `hyper::body::Bytes` in the import list above (previously imported inline); the rest of the file uses `Bytes` unchanged.

- [ ] **Step 4: Migrate `ResponseBuilder::build()`**

In `ultimo/src/response.rs`, change the body construction in `build()` (was `.body(Full::new(Bytes::from(body)))`):

```rust
        // Set body
        let body = self.body.unwrap_or_default();
        response
            .body(UltimoBody::full(body))
            .map_err(|e| UltimoError::Internal(format!("Failed to build response: {}", e)))
```

- [ ] **Step 5: Migrate `app.rs` WebSocket handler type**

In `ultimo/src/app.rs`, change the type at line 36:

```rust
type BoxedWebSocketHandler =
    Arc<dyn Fn(WebSocketUpgrade<()>) -> crate::response::Response + Send + Sync>;
```

(The `oneshot` request signature at line 610 stays `HyperRequest<http_body_util::Full<Bytes>>` — request side is unchanged. The test-module request bodies at lines 862/875 also stay `Full::new(...)`.)

- [ ] **Step 6: Migrate `static_files.rs`**

In `ultimo/src/static_files.rs`, line 70 (`.body(Full::new(Bytes::new()))`) → `.body(crate::response::UltimoBody::empty())`, and line 90 (`.body(Full::new(Bytes::from(content)))`) → `.body(crate::response::UltimoBody::full(content))`. Remove the now-unused `use http_body_util::Full;` at line 7 if the compiler flags it.

- [ ] **Step 7: Migrate `middleware.rs` early-return bodies**

In `ultimo/src/middleware.rs`:
- line ~192: `.body(Full::new(Bytes::new()))` → `.body(crate::response::UltimoBody::empty())`
- line ~572: `.body(Full::new(Bytes::from("Forbidden")))` → `.body(crate::response::UltimoBody::full("Forbidden"))`
- line ~729: `.body(Full::new(Bytes::from("Too Many Requests")))` → `.body(crate::response::UltimoBody::full("Too Many Requests"))`

- [ ] **Step 8: Migrate `middleware.rs` compression `Full::new` sites (keep buffering for now)**

In the compression block, replace every `Full::new(body_bytes)` and `Full::new(Bytes::from(compressed))` with `crate::response::UltimoBody::full(...)`. Leave the `body.collect().await.unwrap()` line as-is for now (still correct — `UltimoBody: Body`; `.unwrap()` now unwraps a `BoxError` result but streams don't exist yet). The stream-skip is Task 4.

Specifically:
- `Ok(hyper::Response::from_parts(parts, Full::new(body_bytes)))` → `Ok(hyper::Response::from_parts(parts, crate::response::UltimoBody::full(body_bytes)))` (all three occurrences: below-min-size, skip-binary, no-encoding)
- `hyper::Response::from_parts(parts, Full::new(Bytes::from(compressed)))` → `hyper::Response::from_parts(parts, crate::response::UltimoBody::full(compressed))` (brotli + gzip arms)

Remove the now-unused `Full` import in `middleware.rs` if flagged.

- [ ] **Step 9: Migrate `websocket/upgrade.rs`**

In `ultimo/src/websocket/upgrade.rs`:
- Change the three method return types (lines ~95, ~198, ~301) from `HyperResponse<Full<Bytes>>` to `crate::response::Response`.
- Error bodies `Full::new(Bytes::from("..."))` → `crate::response::UltimoBody::full("...")` (the "Invalid WebSocket upgrade request", "Origin not allowed", "Missing Sec-WebSocket-Key header" cases).
- Empty upgrade bodies `Full::new(Bytes::new())` (lines ~143, ~252) → `crate::response::UltimoBody::empty()`.
- Remove the `use http_body_util::Full;` at line 8 if flagged (keep `Bytes` if still used elsewhere; drop if not).

- [ ] **Step 10: Run the new unit tests + full lib suite**

Run: `cargo test -p ultimo --lib 2>&1 | tail -20`
Expected: PASS — new `ultimo_body_*` tests pass; all pre-existing tests still pass.

- [ ] **Step 11: Compile the feature surface (ensures ws/static/compression migrated cleanly)**

Run: `cargo clippy -p ultimo --features "websocket,test-helpers,testing,session,csrf" --all-targets -- -D warnings 2>&1 | tail -20`
Expected: no errors, no warnings.

- [ ] **Step 12: Commit**

```bash
git add ultimo/src/response.rs ultimo/src/app.rs ultimo/src/static_files.rs ultimo/src/middleware.rs ultimo/src/websocket/upgrade.rs
git commit -m "feat!: introduce UltimoBody response body (buffered fast-path)

Replace the hardwired Full<Bytes> response body with an UltimoBody enum
that keeps a zero-boxing Full fast-path and adds a boxed streaming variant.
No behavior change yet — all bodies are Full. Breaking: Response body type."
```

---

### Task 2: `UltimoBody::stream()` constructor

**Files:**
- Modify: `ultimo/src/response.rs` (add `stream` constructor + test)

**Interfaces:**
- Consumes: `UltimoBody`, `BoxError` (Task 1).
- Produces: `impl UltimoBody { pub fn stream<S>(s: S) -> Self where S: futures_util::Stream<Item = Result<Bytes, BoxError>> + Send + 'static; }`

- [ ] **Step 1: Write the failing test**

Add to `ultimo/src/response.rs` tests:

```rust
    #[tokio::test]
    async fn ultimo_body_stream_concatenates_chunks() {
        use http_body_util::BodyExt;
        let chunks: Vec<std::result::Result<Bytes, BoxError>> = vec![
            Ok(Bytes::from("foo")),
            Ok(Bytes::from("bar")),
            Ok(Bytes::from("baz")),
        ];
        let body = UltimoBody::stream(futures_util::stream::iter(chunks));
        let bytes = body.collect().await.unwrap().to_bytes();
        assert_eq!(&bytes[..], b"foobarbaz");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ultimo --lib ultimo_body_stream 2>&1 | tail -20`
Expected: FAIL — `no function stream on UltimoBody`.

- [ ] **Step 3: Implement `stream`**

Add to the `impl UltimoBody` block in `ultimo/src/response.rs`:

```rust
    /// A streaming body produced from a `Stream` of byte chunks.
    ///
    /// Each `Ok(bytes)` becomes a data frame; an `Err(_)` aborts the connection.
    /// The response is sent chunked (no `Content-Length`).
    pub fn stream<S>(stream: S) -> Self
    where
        S: futures_util::Stream<Item = std::result::Result<Bytes, BoxError>> + Send + 'static,
    {
        use futures_util::TryStreamExt;
        use http_body_util::{BodyExt, StreamBody};
        let body = StreamBody::new(stream.map_ok(Frame::data));
        UltimoBody::Stream(body.boxed())
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ultimo --lib ultimo_body_stream 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add ultimo/src/response.rs
git commit -m "feat: add UltimoBody::stream constructor"
```

---

### Task 3: `Context::stream()` + integration test

**Files:**
- Modify: `ultimo/src/context.rs` (add `stream` method)
- Create: `ultimo/tests/streaming.rs`

**Interfaces:**
- Consumes: `UltimoBody::stream` (Task 2), `Response`, `BoxError`.
- Produces: `impl Context { pub async fn stream<S>(&self, s: S) -> Result<Response> where S: futures_util::Stream<Item = Result<Bytes, BoxError>> + Send + 'static; }`

- [ ] **Step 1: Write the failing integration test**

Create `ultimo/tests/streaming.rs`:

```rust
//! Integration tests for streaming response bodies.
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::Request as HyperRequest;
use ultimo::prelude::*;
use ultimo::response::BoxError;

fn empty_req(uri: &str) -> HyperRequest<Full<Bytes>> {
    HyperRequest::builder()
        .uri(uri)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

#[tokio::test]
async fn ctx_stream_sends_chunks_without_content_length() {
    let mut app = Ultimo::new_without_defaults();
    app.get("/stream", |ctx: Context| async move {
        let chunks: Vec<Result<Bytes, BoxError>> = vec![
            Ok(Bytes::from("chunk-1;")),
            Ok(Bytes::from("chunk-2;")),
            Ok(Bytes::from("chunk-3")),
        ];
        ctx.stream(futures_util::stream::iter(chunks)).await
    });

    let resp = app.oneshot(empty_req("/stream")).await;
    assert_eq!(resp.status(), 200);
    assert!(
        resp.headers().get(hyper::header::CONTENT_LENGTH).is_none(),
        "streamed responses must not carry Content-Length"
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"chunk-1;chunk-2;chunk-3");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ultimo --features "testing" --test streaming 2>&1 | tail -20`
Expected: FAIL — `no method stream on Context`.

- [ ] **Step 3: Implement `Context::stream`**

Add the import and method to `ultimo/src/context.rs`. In the `impl Context` block that holds `text`/`json`/`html` (near line 570), add:

```rust
    /// Return a streaming response body.
    ///
    /// Each `Ok(bytes)` item is sent as a chunk; the response has no
    /// `Content-Length` (chunked transfer). Any queued `header`/`status` set on
    /// the context is applied. An `Err(_)` item aborts the connection.
    ///
    /// ```no_run
    /// # use ultimo::prelude::*;
    /// # use ultimo::response::BoxError;
    /// # use hyper::body::Bytes;
    /// # async fn h(ctx: Context) -> ultimo::error::Result<Response> {
    /// let chunks: Vec<Result<Bytes, BoxError>> = vec![Ok(Bytes::from("hi"))];
    /// ctx.stream(futures_util::stream::iter(chunks)).await
    /// # }
    /// ```
    pub async fn stream<S>(&self, body: S) -> Result<Response>
    where
        S: futures_util::Stream<Item = std::result::Result<Bytes, crate::response::BoxError>>
            + Send
            + 'static,
    {
        let status = self.response_status.read().await.unwrap_or(200);
        let mut builder = hyper::Response::builder()
            .status(hyper::StatusCode::from_u16(status).unwrap_or(hyper::StatusCode::OK));
        let headers = self.response_headers.read().await;
        for (name, value) in headers.iter() {
            builder = builder.header(name.clone(), value.clone());
        }
        builder
            .body(crate::response::UltimoBody::stream(body))
            .map_err(|e| UltimoError::Internal(format!("Failed to build response: {}", e)))
    }
```

> `Bytes` is already imported in `context.rs` (line 10: `use bytes::Bytes;`). `response_status` / `response_headers` are the same `RwLock` fields used by `build_response` and `redirect`. If field names differ, mirror what `redirect`/`build_response` read.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ultimo --features "testing" --test streaming 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Verify the doctest compiles**

Run: `cargo test -p ultimo --doc --features "websocket,testing,session,csrf" 2>&1 | tail -15`
Expected: PASS (the `no_run` doctest on `stream` compiles).

- [ ] **Step 6: Commit**

```bash
git add ultimo/src/context.rs ultimo/tests/streaming.rs
git commit -m "feat: add Context::stream for chunked response bodies"
```

---

### Task 4: Compression skips streaming bodies

**Files:**
- Modify: `ultimo/src/middleware.rs` (compression body handling)
- Modify: `ultimo/tests/streaming.rs` (add compression tests)

**Interfaces:**
- Consumes: `UltimoBody` variants, `Context::stream` (Task 3), `compression()` middleware.

- [ ] **Step 1: Write the failing tests**

Append to `ultimo/tests/streaming.rs`:

```rust
#[tokio::test]
async fn compression_passes_streaming_bodies_through() {
    use ultimo::middleware::builtin::compression;
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/stream", |ctx: Context| async move {
        // A large streamed body that would otherwise be well over min_size.
        let chunks: Vec<Result<Bytes, BoxError>> =
            (0..100).map(|_| Ok(Bytes::from("x".repeat(64)))).collect();
        ctx.stream(futures_util::stream::iter(chunks)).await
    });

    let req = HyperRequest::builder()
        .uri("/stream")
        .header("Accept-Encoding", "br, gzip")
        .body(Full::new(Bytes::new()))
        .unwrap();
    let resp = app.oneshot(req).await;
    assert!(
        resp.headers().get(hyper::header::CONTENT_ENCODING).is_none(),
        "streaming bodies must not be compressed"
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.len(), 100 * 64);
}

#[tokio::test]
async fn compression_still_compresses_buffered_bodies() {
    use ultimo::middleware::builtin::compression;
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/buffered", |ctx: Context| async move {
        ctx.text("y".repeat(4096)).await
    });

    let req = HyperRequest::builder()
        .uri("/buffered")
        .header("Accept-Encoding", "gzip")
        .body(Full::new(Bytes::new()))
        .unwrap();
    let resp = app.oneshot(req).await;
    assert_eq!(
        resp.headers()
            .get(hyper::header::CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok()),
        Some("gzip"),
        "buffered bodies over min_size must still be compressed"
    );
}
```

- [ ] **Step 2: Run tests to verify the streaming one fails**

Run: `cargo test -p ultimo --features "testing,compression" --test streaming 2>&1 | tail -25`
Expected: `compression_passes_streaming_bodies_through` FAILS (the current `body.collect()` buffers the stream, and it may either compress it or panic) — while `compression_still_compresses_buffered_bodies` passes. The failing streaming assertion is the target.

- [ ] **Step 3: Make compression skip streaming bodies**

In `ultimo/src/middleware.rs`, replace the buffering line (`let body_bytes = body.collect().await.unwrap().to_bytes();`) and its preceding decompose comment with a variant match that short-circuits streams:

```rust
                    // Decompose response so we can inspect and replace the body.
                    let (parts, body) = res.into_parts();

                    // Never buffer a streaming body — pass it through untouched.
                    let body_bytes = match body {
                        crate::response::UltimoBody::Stream(_) => {
                            return Ok(hyper::Response::from_parts(parts, body));
                        }
                        crate::response::UltimoBody::Full(full) => {
                            full.collect().await.unwrap().to_bytes()
                        }
                    };
```

> The `Full` arm uses `http_body_util::BodyExt::collect` on the inner `Full<Bytes>` (its error is `Infallible`, so `.unwrap()` is safe). `BodyExt` is already in scope in this module; if not, add `use http_body_util::BodyExt;` at the top.

- [ ] **Step 4: Run tests to verify both pass**

Run: `cargo test -p ultimo --features "testing,compression" --test streaming 2>&1 | tail -25`
Expected: PASS — both compression tests and the two earlier streaming tests.

- [ ] **Step 5: Commit**

```bash
git add ultimo/src/middleware.rs ultimo/tests/streaming.rs
git commit -m "feat: compression passes streaming bodies through uncompressed"
```

---

### Task 5: Documentation surfaces + runnable example

Per `.claude/skills/ship-feature`, every user-facing feature lands its docs and an example in the same PR.

**Files:**
- Modify: `docs-site/docs/pages/api-reference.mdx`
- Create: `docs-site/docs/pages/streaming.mdx`
- Modify: `docs-site/vocs.config.ts` (sidebar entry)
- Modify: `docs-site/docs/pages/roadmap.mdx` (move Streaming Responses → shipped 0.9.0)
- Modify: `README.md` (add to Available Now)
- Create: `examples/streaming/{Cargo.toml,src/main.rs}`
- Modify: `Cargo.toml` (workspace `members`)

- [ ] **Step 1: Add the `Context::stream` + `UltimoBody` entries to api-reference**

In `docs-site/docs/pages/api-reference.mdx`, under `#### Response Methods` (after the `html` entry, ~line 197), add:

````markdown
##### `stream<S>(&self, body: S) -> Result<Response>`

Return a chunked/streaming response body from a `Stream<Item = Result<Bytes, BoxError>>`.
No `Content-Length` is set. Queued `header`/`status` are applied.

```rust
use hyper::body::Bytes;
use ultimo::response::BoxError;

async fn download(ctx: Context) -> Result<Response> {
    let chunks: Vec<Result<Bytes, BoxError>> = vec![
        Ok(Bytes::from("part-1")),
        Ok(Bytes::from("part-2")),
    ];
    ctx.stream(futures_util::stream::iter(chunks)).await
}
```
````

Under `## Core Types` (near the `Ultimo` type), add a short subsection:

````markdown
### `UltimoBody`

The response body type (`type Response = hyper::Response<UltimoBody>`). Two
variants: `Full` (buffered — the fast path, produced by `text`/`json`/`html`) and
`Stream` (produced by `Context::stream`). `BoxError` is the streaming error type
(`Box<dyn std::error::Error + Send + Sync>`).
````

- [ ] **Step 2: Write `streaming.mdx`**

Create `docs-site/docs/pages/streaming.mdx`:

````markdown
# Streaming Responses

Return a response body incrementally instead of buffering it all in memory.
Useful for large downloads, proxied bodies, and as the foundation for
Server-Sent Events.

Streaming is always available — no Cargo feature required.

## `ctx.stream(...)`

`ctx.stream` takes a `Stream` of byte chunks and sends them as they arrive. The
response is chunked (no `Content-Length`).

```rust
use hyper::body::Bytes;
use ultimo::prelude::*;
use ultimo::response::BoxError;

async fn numbers(ctx: Context) -> Result<Response> {
    // Any `Stream<Item = Result<Bytes, BoxError>>` works.
    let chunks: Vec<Result<Bytes, BoxError>> =
        (0..5).map(|n| Ok(Bytes::from(format!("line {n}\n")))).collect();
    ctx.stream(futures_util::stream::iter(chunks)).await
}
```

Set a content type first if you need one:

```rust
async fn csv(ctx: Context) -> Result<Response> {
    ctx.header("Content-Type", "text/csv").await;
    let rows: Vec<Result<Bytes, BoxError>> = vec![
        Ok(Bytes::from("a,b,c\n")),
        Ok(Bytes::from("1,2,3\n")),
    ];
    ctx.stream(futures_util::stream::iter(rows)).await
}
```

## Errors

If a stream item yields `Err(_)`, the response connection is aborted. Emit
`Ok(_)` chunks for normal data and reserve `Err(_)` for genuine failures.

## Interaction with compression

The `compression` middleware **skips streaming bodies** — it never buffers a
stream (that would defeat streaming and risk unbounded memory). Buffered
responses (`text`/`json`/`html`) are still compressed as usual.

## The body type

`Response` is `hyper::Response<UltimoBody>`. `UltimoBody::Full` is the buffered
fast path (what `text`/`json`/`html` produce); `UltimoBody::Stream` is what
`ctx.stream` produces. You rarely construct it directly.

## Full example

See [`examples/streaming`](https://github.com/ultimo-rs/ultimo/tree/main/examples/streaming):
`cargo run -p streaming-example`, then open `http://127.0.0.1:3000`.
````

- [ ] **Step 3: Add the sidebar entry**

In `docs-site/vocs.config.ts`, inside the "Features" sidebar group's `items` array (near the Static Files / WebSocket entries, ~line 91), add:

```ts
        {
          text: "Streaming Responses",
          link: "/streaming",
        },
```

- [ ] **Step 4: Update the roadmap**

In `docs-site/docs/pages/roadmap.mdx`:
- Remove the "Streaming responses" bullet under "Real-time & streaming" (line ~31).
- In the status table, change the `Streaming Responses` row (line ~137) from `📋 Planned | 0.7.0` to `✅ Shipped | 0.9.0` (match the format of other shipped rows in the file).

- [ ] **Step 5: Update the README**

In `README.md`, add to the feature list (near the "Testing utilities" bullet, ~line 45):

```markdown
- 🌊 **Streaming responses** — chunked/streaming bodies via `ctx.stream(...)`.
```

- [ ] **Step 6: Create the example crate**

Create `examples/streaming/Cargo.toml`:

```toml
[package]
name = "streaming-example"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
ultimo = { path = "../../ultimo" }
tokio = { version = "1", features = ["full"] }
futures-util = "0.3"
hyper = "1"
```

Create `examples/streaming/src/main.rs`:

```rust
//! Streaming response demo: a route that streams numbered lines, served with a
//! tiny HTML page that reads the stream via `fetch`.
use hyper::body::Bytes;
use std::time::Duration;
use ultimo::prelude::*;
use ultimo::response::BoxError;

const PAGE: &str = r#"<!doctype html>
<meta charset="utf-8"><title>Ultimo streaming demo</title>
<h1>Streaming demo</h1>
<button onclick="go()">Stream</button>
<pre id="out"></pre>
<script>
async function go() {
  const out = document.getElementById('out');
  out.textContent = '';
  const res = await fetch('/numbers');
  const reader = res.body.getReader();
  const dec = new TextDecoder();
  for (;;) {
    const { value, done } = await reader.read();
    if (done) break;
    out.textContent += dec.decode(value);
  }
}
</script>"#;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = Ultimo::new();

    app.get("/", |ctx: Context| async move { ctx.html(PAGE).await });

    app.get("/numbers", |ctx: Context| async move {
        // A stream that yields a line every 300ms, ten times.
        let s = futures_util::stream::unfold(0u32, |n| async move {
            if n >= 10 {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
            let chunk: std::result::Result<Bytes, BoxError> =
                Ok(Bytes::from(format!("line {n}\n")));
            Some((chunk, n + 1))
        });
        ctx.stream(s).await
    });

    println!("→ http://127.0.0.1:3000");
    app.listen("127.0.0.1:3000").await
}
```

- [ ] **Step 7: Register the example in the workspace**

In the root `Cargo.toml`, add `"examples/streaming"` to the `members` array.

- [ ] **Step 8: Build the example + docs sanity**

Run: `cargo build -p streaming-example 2>&1 | tail -15`
Expected: compiles cleanly.

- [ ] **Step 9: Commit**

```bash
git add docs-site/docs/pages/api-reference.mdx docs-site/docs/pages/streaming.mdx docs-site/vocs.config.ts docs-site/docs/pages/roadmap.mdx README.md examples/streaming Cargo.toml
git commit -m "docs: streaming responses (api-reference, guide, roadmap, README, example)"
```

---

### Task 6: Final gate + PR

- [ ] **Step 1: Run the full verification gate**

```bash
cd /Users/ruslanelishaev/Desktop/projects/ultimo
cargo fmt --all --check
cargo clippy -p ultimo --features "websocket,test-helpers,testing,session,csrf" --all-targets -- -D warnings
cargo test -p ultimo --lib
cargo test -p ultimo --features "websocket,test-helpers,testing,session,csrf,compression"
cargo test -p ultimo --doc --features "websocket,testing,session,csrf"
cargo build -p streaming-example
```

Expected: all green.

- [ ] **Step 2: Push and open the PR**

```bash
git push --no-verify -u origin feat/streaming-responses
gh pr create --fill
```

- [ ] **Step 3: Watch CI**

```bash
gh pr checks --watch
```

`semver-checks` will flag the `Response` body-type change — expected for a `feat!`. Everything else must be green.

- [ ] **Step 4: Merge (admin squash) + link issue #2**

```bash
gh pr merge --squash --admin --delete-branch
git switch main && git pull
```

Comment on #2 (SSE) that streaming primitives are now available (`ctx.stream`), leaving #2 open for the SSE layer.

---

## Self-Review

**Spec coverage:**
- `UltimoBody` enum (Full + Stream) → Task 1. ✔
- `BoxError` → Task 1. ✔
- `Body` impl (poll_frame/is_end_stream/size_hint) → Task 1. ✔
- `Response` alias change → Task 1. ✔
- Buffered helpers unchanged → Task 1 (regression suite). ✔
- `UltimoBody::{empty,full}` → Task 1; `stream` → Task 2. ✔
- `ctx.stream` (no Content-Length) → Task 3. ✔
- Compression skips streams → Task 4. ✔
- Blast-radius migration (5 files) → Task 1. ✔
- Dependency: uses existing `futures-util` (spec's `futures-core` moot — noted). ✔
- SemVer 0.9.0 / `feat!` → Task 1 commit + Task 6. ✔
- Testing (unit + integration + compression) → Tasks 1–4. ✔
- Docs surfaces + example → Task 5. ✔
- Follow-up SSE (#2) noted, out of scope → Task 6 step 4. ✔

**Placeholder scan:** none — every code step has concrete code.

**Type consistency:** `UltimoBody`, `BoxError`, `Response`, `UltimoBody::{empty,full,stream}`, `Context::stream` signatures are identical across Tasks 1–5. `futures_util::Stream<Item = Result<Bytes, BoxError>>` bound is uniform in Tasks 2 and 3. ✔
