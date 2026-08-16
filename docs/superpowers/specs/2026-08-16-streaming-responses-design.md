# Streaming Responses — Design

**Date:** 2026-08-16
**Status:** Approved (design)
**Target release:** 0.9.0 (breaking — `Response` body type changes)

## Goal

Let handlers return **chunked / streaming response bodies** — data emitted
incrementally over the connection instead of buffered fully in memory before the
first byte is sent. This unlocks large downloads, proxy/pass-through bodies, and
is the required foundation for **typed SSE / subscriptions** (the next roadmap
item, built on top of the primitive introduced here).

Today every response is fully buffered: `pub type Response =
HyperResponse<Full<Bytes>>` (`ultimo/src/response.rs:12`). There is no way to
send a body the framework doesn't hold entirely in memory.

## Non-goals (v1)

- Typed SSE / event framing — separate follow-up feature, layered on `ctx.stream`.
- Streaming *file* serving (`static_files.rs` keeps buffering whole files for now).
- Trailers, custom backpressure tuning, HTTP/2 flow-control knobs.
- A channel/sink helper — may be added later; v1 exposes the `Stream` form only.

## Architecture

### The body type: `UltimoBody`

Replace the hardwired `Full<Bytes>` body with a small enum that keeps a
**zero-boxing fast path** for the common buffered case and adds a streaming
variant:

```rust
/// Error type carried by a streaming body. A stream item error aborts the
/// response connection.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub enum UltimoBody {
    /// Buffered — the hot path. No per-response allocation beyond the bytes.
    Full(Full<Bytes>),
    /// Streaming — a boxed `http_body::Body` produced from a `Stream`.
    Stream(BoxBody<Bytes, BoxError>),
}

pub type Response = HyperResponse<UltimoBody>;
```

`UltimoBody` implements `http_body::Body<Data = Bytes, Error = BoxError>`:

- `poll_frame`: delegates to the inner body. The `Full` arm maps its
  `Infallible` error into `BoxError`; the `Stream` arm passes through.
- `is_end_stream` / `size_hint`: delegate to the inner body (so buffered
  responses still report an exact size → `Content-Length`, streaming ones don't).

**Why an enum over erasing everything to `BoxBody`:** boxing every response —
including the overwhelmingly common buffered one — would add an allocation and a
dynamic dispatch to the hot path the benchmarks guard. The enum keeps buffered
responses allocation-identical to today while allowing streams.

### Constructors

```rust
impl UltimoBody {
    pub fn empty() -> Self;                              // Full(Full::new(Bytes::new()))
    pub fn full(bytes: impl Into<Bytes>) -> Self;        // buffered
    pub fn stream<S>(s: S) -> Self                        // streaming
    where
        S: Stream<Item = Result<Bytes, BoxError>> + Send + 'static;
}
```

`stream` wraps the caller's `Stream` in a tiny adapter that maps each
`Ok(bytes)` into `http_body::Frame::data(bytes)`, feeds it through
`http_body_util::StreamBody`, and boxes the result (`BodyExt::boxed`) into
`UltimoBody::Stream`. The adapter is ~15 lines and needs only the `Stream` trait
(new dep `futures-core`, see below) — it deliberately avoids pulling
`futures-util` into the runtime.

### Handler-facing API

```rust
impl Context {
    /// 200 OK, no Content-Length, body streamed from `s`.
    pub fn stream<S>(&self, s: S) -> Response
    where
        S: Stream<Item = Result<Bytes, BoxError>> + Send + 'static;
}
```

`ctx.stream(...)` builds a `Response` whose body is `UltimoBody::stream(s)`,
status `200`, and **no `Content-Length`** (hyper emits `Transfer-Encoding:
chunked` on HTTP/1.1). Headers can be adjusted on the returned `Response` before
returning it (e.g. `Content-Type`). The existing buffered helpers
(`text`/`json`/`html`, `ResponseBuilder::build`) are unchanged in behavior — they
now produce `UltimoBody::Full`.

## Compression interaction (must-handle)

The compression middleware currently buffers the whole body:
`middleware.rs:854` does `body.collect().await...`. For a streaming body that
would drain the entire stream into memory — defeating streaming and risking
unbounded memory. So compression **only compresses `UltimoBody::Full`; streaming
bodies pass through untouched**:

```rust
let (parts, body) = res.into_parts();
match body {
    UltimoBody::Stream(_) => return Ok(Response::from_parts(parts, body)), // pass through
    UltimoBody::Full(full) => { /* existing collect + gzip/brotli path */ }
}
```

`Vary: Accept-Encoding` handling is unchanged for the buffered path.

## Blast radius

`Full` appears across 9 files (~41 sites). Each buffered construction becomes
`UltimoBody::Full(...)` / `UltimoBody::full(...)` / `UltimoBody::empty()`:

| File | Sites | Change |
|---|---|---|
| `response.rs` | 3 | Define `UltimoBody`, `BoxError`; `Response` alias; `build()` → `UltimoBody::full` |
| `middleware.rs` | 10 | Error/early-return bodies → `UltimoBody::full`; compression skips `Stream` |
| `app.rs` | 5 | dispatch/`oneshot` body type; service returns `UltimoBody` |
| `websocket/upgrade.rs` | 12 | Upgrade (101) empty body → `UltimoBody::empty()`; handler fn type |
| `websocket/connection.rs` | 3 | Body construction → `UltimoBody` |
| `static_files.rs` | 3 | File/404 bodies → `UltimoBody::full` (still buffered) |
| `error.rs` | 2 | Error response bodies → `UltimoBody::full` |
| `testing/client.rs` | 2 | Response side reads `UltimoBody` (`.collect()` still works); request side stays `Full<Bytes>` |
| `websocket/pubsub.rs` | 1 | Body construction → `UltimoBody` |

**Request side is unchanged:** `oneshot(req: HyperRequest<Full<Bytes>>)` and the
testing client keep `Full<Bytes>` on the *request* body.

## Dependencies

- Add **`futures-core`** (runtime, tiny, `no_std`) — only for the `Stream` trait
  bound in the public API. No `futures-util` at runtime.
- `http_body_util` (already a dep) provides `StreamBody`, `BodyExt::boxed`,
  `BoxBody`, `Full`.
- Must pass `cargo-deny` / `cargo-audit` — `futures-core` is standard and audited.

## Backward compatibility & SemVer

Breaking: `Response`'s public body type changes from `Full<Bytes>` to
`UltimoBody`.

- `res.into_body()` now yields `UltimoBody` (still `impl Body`, so `.collect()`
  keeps working).
- User code that names `HyperResponse<Full<Bytes>>` or constructs a `Response`
  with a `Full<Bytes>` body directly breaks and must switch to `UltimoBody`.
- Pre-1.0 rule: breaking → **minor** bump → **0.9.0**. `semver-checks` will flag
  it; land with a `feat!` commit and the admin squash-merge override.

## Testing

- **Unit:** `UltimoBody::full` round-trips via `.collect()`; `UltimoBody::stream`
  over a `stream::iter` of chunks collects to the concatenation; `empty()` yields
  zero bytes and `is_end_stream`.
- **Integration:** a handler using `ctx.stream(...)` emits N chunks →
  `oneshot` collects the concatenated bytes; assert **no `Content-Length`**
  header on the streamed response.
- **Compression:** a streamed body passes through with **no `Content-Encoding`**;
  a buffered body over `min_size` is still compressed (guards the variant split).
- Gate integration tests behind the existing feature combo the CI gate uses.

## Open questions — resolved

- **Enum vs box-everything?** Enum (fast path). ✔ (approved)
- **v1 input type?** `Stream<Item = Result<Bytes, BoxError>>`; channel/SSE
  helpers later. ✔ (approved)
- **Feature-gate streaming?** No — the `Response` body-type change is core and
  can't be cleanly gated; `ctx.stream` only pulls `futures-core`, so it's always
  available.
- **Error semantics?** A stream item `Err` aborts the connection (documented on
  `ctx.stream`).

## Follow-ups (not this spec)

1. Typed SSE / subscriptions on top of `ctx.stream` (next roadmap item; closes #2).
2. Optional channel/sink helper (`ctx.stream_channel()`).
3. Streaming large static files.
