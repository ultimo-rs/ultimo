# Typed Server-Sent Events (SSE) — Design

**Date:** 2026-08-16
**Status:** Approved (design)
**Target release:** 0.9.1 (additive → patch, pre-1.0; release-plz computes the exact number)
**Closes:** most of #2 (SSE support) — the derived-TS-types item is deferred to layer B.

## Goal

A first-class **Server-Sent Events** API for server→client push, built entirely
on the shipped `ctx.stream` primitive (v0.9.0). Handlers emit **typed events** —
any `Serialize` payload serialized to the SSE `data:` field with a typed
`event:` name — over a long-lived `text/event-stream` response that the browser's
native `EventSource` consumes and auto-reconnects.

## Scope

**In (layer A — this spec):** the SSE transport primitive.
**Out (layer B — separate future spec):** ts-rs–derived event *union types* wired
into the RPC registry with generated TypeScript subscription methods (tRPC-style).
Layer B depends on layer A and is an independent subsystem.

Also out of v1: server-side event replay/buffering (that's application state, not
the transport's job).

## Non-goals

- Client reconnection logic — the browser `EventSource` reconnects natively; we
  only emit the `retry:` hint.
- A new Cargo feature — SSE needs no new dependencies (`serde_json` and tokio
  `mpsc` are already dependencies), so like streaming it is always available.

## Architecture

New module `ultimo/src/sse.rs`. Everything sits on top of `Context::stream`:
an SSE response is a `text/event-stream` body whose chunks are encoded
`SseEvent`s. No changes to the response body type; `ctx.sse` is a thin typed
layer over `ctx.stream`.

### `SseEvent`

A builder that encodes to the SSE wire format.

```rust
// Typed payload — any T: Serialize → JSON in the data: field.
SseEvent::new(&payload)?          // Result<SseEvent, UltimoError> (serialization)
    .event("message")             // event: <name>
    .id("42")                     // id: <id>
    .retry(Duration::from_secs(3))// retry: <ms>

// Raw text payload (already-serialized / plain string).
SseEvent::data("hello")

// Keep-alive / comment line.
SseEvent::comment("ping")
```

**Encoding rules** (`fn encode(&self) -> Bytes`):
- `event:`, `id:`, `retry:` lines emitted when set (retry as integer milliseconds).
- `data:` payload split on `\n` into one `data:` line per segment (SSE spec
  requires this; a bare `data:` with embedded newlines is invalid).
- A comment encodes as a single `: <text>` line.
- Each event terminated by a blank line (`\n\n`).
- Empty `data` with no fields other than a comment is allowed (heartbeat).

### Handler API (on `Context`)

```rust
// Core: stream of typed events → text/event-stream response.
pub async fn sse<S>(&self, events: S) -> Result<Response>
where S: Stream<Item = SseEvent> + Send + 'static;

// Same, with a periodic ": ping" comment merged in so idle connections and
// proxies don't drop the stream. Opt-in.
pub async fn sse_keep_alive<S>(&self, events: S, interval: Duration) -> Result<Response>
where S: Stream<Item = SseEvent> + Send + 'static;

// Read the Last-Event-ID request header for app-driven resume.
pub fn last_event_id(&self) -> Option<String>;
```

`sse` sets the SSE headers, then delegates to `ctx.stream(events.map(|e| Ok(e.encode())))`:
- `Content-Type: text/event-stream`
- `Cache-Control: no-cache`
- `X-Accel-Buffering: no` (defeats nginx proxy buffering)
- no `Content-Length` (inherited from `ctx.stream`'s chunked framing)

Queued `ctx.header(...)`/`ctx.status(...)` still apply (via the same
`response_meta` path `ctx.stream` uses), except the SSE content-type is forced.

### Push/broadcast helper (in `sse.rs`)

```rust
// Thin mpsc wrapper for the push case (broadcast, notifications).
pub fn sse_channel() -> (SseSender, impl Stream<Item = SseEvent> + Send + 'static);

impl SseSender {
    // Non-blocking send; Err if the receiver (client) has gone away.
    pub fn send(&self, event: SseEvent) -> Result<(), SseClosed>;
}
```

`sse_channel` wraps `tokio::sync::mpsc` and adapts the receiver to a `Stream`
(hand-rolled `futures_util::stream::unfold` over `recv()` — no new dep). A
handler keeps the `SseSender` (or clones it into other tasks) and returns
`ctx.sse(rx_stream)`. Dropping all senders ends the stream; the client's
`EventSource` then reconnects.

### Route sugar (on `Ultimo`)

```rust
// SSE is a GET; this is sugar over `get` for discoverability/intent.
pub fn sse<H, ...>(&mut self, path: &str, handler: H) -> &mut Self;
```

Registers the handler on `GET path` exactly as `get` does; exists so
`app.sse("/events", ...)` reads as intent and matches issue #2's sketch.

## Data flow

```
handler → Stream<SseEvent> ──ctx.sse──▶ map(encode)=Stream<Result<Bytes>> ──ctx.stream──▶ UltimoBody::Stream ─▶ hyper (chunked, text/event-stream)
                                   (+ SSE headers)                                                          ▲
sse_channel(): SseSender.send(ev) ─▶ mpsc ─▶ unfold(recv) stream ────────────────────────────────────────┘
```

## Error handling

- `SseEvent::new` returns `Result` (JSON serialization can fail); `data`/`comment`
  are infallible.
- Encoding is infallible → `ctx.sse` maps each event to `Ok(bytes)` for `ctx.stream`.
- `SseSender::send` returns `Err(SseClosed)` when the client has disconnected
  (receiver dropped); the handler can stop producing.
- A dropped sender / ended stream closes the response cleanly; no panic path.

## Testing

- **Unit (`sse.rs`):**
  - `SseEvent::new(&value)` encodes `data: {json}\n\n`.
  - `.event/.id/.retry` emit the right lines in order; `retry` is integer ms.
  - multi-line `data` splits into multiple `data:` lines.
  - `SseEvent::comment("ping")` encodes `: ping\n\n`.
- **Integration (`tests/sse.rs`, via `oneshot`):**
  - `app.sse` route → assert `Content-Type: text/event-stream`, `Cache-Control: no-cache`, **no `Content-Length`**; collected body equals the expected wire frames.
  - `sse_channel`: events pushed through `SseSender` appear in the response body in order.
  - `ctx.last_event_id()` returns the `Last-Event-ID` request header value.

## Documentation & example (ship-feature surfaces)

- `docs-site/docs/pages/sse.mdx` + `vocs.config.ts` sidebar entry.
- `docs-site/docs/pages/api-reference.mdx`: `SseEvent`, `Context::{sse,sse_keep_alive,last_event_id}`, `sse_channel`/`SseSender`, `Ultimo::sse`.
- `README.md`: add to feature list / Available Now.
- `docs-site/docs/pages/roadmap.mdx`: move the SSE row to ✅ Available.
- `examples/sse`: a live counter/clock feed consumed by a browser `EventSource`,
  including the documented ~10-line TypeScript `EventSource` wrapper (satisfies
  #2's "TS client helpers" without the layer-B derivation machinery). Add to
  workspace `members`.

## SemVer

Purely additive — new `pub` items, no changed signatures. Pre-1.0 rule:
additive → **patch** bump. `semver-checks` should pass; release-plz computes the
number (0.9.1 assuming 0.9.0 releases first).

## Follow-ups (not this spec)

1. **Layer B** — ts-rs–derived event union + RPC-registry subscriptions +
   generated TypeScript subscription client (tRPC-style).
2. Optional server-side replay buffer keyed by `Last-Event-ID`.
