# Testing Utilities — Design Spec

- **Issue:** [#4 Create comprehensive testing utilities](https://github.com/ultimo-rs/ultimo/issues/4)
- **Milestone:** v0.3.0
- **Date:** 2026-06-03
- **Status:** Approved (design), pending implementation plan

## Goal

Make testing an Ultimo application first-class: an in-process test client, a
fluent request builder, response assertion helpers, middleware testing helpers,
database transaction/rollback helpers, fixture management, and documentation —
covering all acceptance criteria of issue #4.

Target API (from the issue):

```rust
let app = build_app();
let client = TestClient::new(app);

let res = client.get("/users").send().await;
res.assert_status(200);
assert_json_eq!(res.json::<Vec<User>>(), expected);
```

## Approach: in-process dispatch

The `TestClient` drives the app **in-process** — it runs the real middleware +
routing pipeline without binding a TCP socket. This is fast, fully deterministic,
and avoids the port/timing flakiness of a real ephemeral server (the pattern that
caused the `test_websocket_echo` flake fixed earlier this revival).

Enabling fact: `Context::new` already collects the request body to `Bytes`
immediately, and `Response` is `hyper::Response<Full<Bytes>>` (fully buffered) —
so both ends of a dispatch are buffered and easy to construct/inspect.

Rejected alternative — **ephemeral real server**: no core refactor and it also
exercises the hyper socket layer, but it is slower and flaky-prone. May be added
later as an opt-in mode; not the default.

## Packaging

- New Cargo feature **`testing`** (in `ultimo/Cargo.toml`, `default = []`).
- New module **`ultimo::testing`**, gated `#[cfg(feature = "testing")]`.
- Database helpers further gated by `testing` + the relevant backend
  (`sqlx` / `diesel`).
- Consumers enable it as a dev-dependency:
  `ultimo = { version = "…", features = ["testing"] }` under `[dev-dependencies]`.

## Components

Each component is an independently understandable, testable unit.

### 1. Core seam (the only change to existing code)

- Add `pub async fn Ultimo::oneshot(&self, req: http::Request<Full<Bytes>>) -> Response`.
  Runs the same middleware + routing as the live server path.
- Extract the body-agnostic core of the private `handle_request(Request<Incoming>)`
  so both the hyper path and `oneshot` share it (no logic duplication).
- Add internal `Context::from_parts(parts: http::request::Parts, body: Bytes, params: Params)`.
  Refactor `Context::new` to collect `Incoming → Bytes` then delegate to it.
  **The existing `pub Context::new` signature is unchanged** → no API breakage.
- `oneshot` is always available (not feature-gated), so it can also serve other
  embedders; `testing` only gates the ergonomic wrappers below.

### 2. `TestClient`

- `TestClient::new(app: Ultimo) -> Self` — takes ownership of a built app.
- Per-method constructors returning a `TestRequest`:
  `get`, `post`, `put`, `delete`, `patch`, `head`, `options`, and a generic
  `request(method, path)`.

### 3. `TestRequest` (builder)

- Fluent setters: `.header(name, value)`, `.headers(iter)`, `.bearer(token)`,
  `.content_type(ct)`, `.query(&[(k, v)])`, `.json(&T: Serialize)`,
  `.body(impl Into<Bytes>)`, `.text(s)`.
- Terminal: `.send(&self).await -> TestResponse` (dispatches through `oneshot`).
- `.json()` sets the `content-type: application/json` header automatically.

### 4. `TestResponse`

- Inspectors: `.status() -> StatusCode`, `.headers()`, `.header(name) -> Option<&str>`,
  `.bytes() -> &Bytes`, `.text() -> String`, `.json::<T: DeserializeOwned>() -> T`.
- Chainable assertions (panic with a clear message on failure, return `&Self`):
  `.assert_status(code)`, `.assert_ok()`, `.assert_status_is_success()`,
  `.assert_header(name, value)`, `.assert_json(&T: Serialize)`,
  `.assert_text(contains)`.
- Body is already buffered, so all inspectors are synchronous (no `.await`).

### 5. Macros

- `assert_json_eq!(actual, expected)` — compares two values as `serde_json::Value`,
  panicking with a readable diff on mismatch.
- `assert_status!(res, code)` — convenience over `assert_status`.

### 6. Middleware testing helpers

- `test_context()` / `TestContextBuilder` — construct a `Context` for unit tests
  (method, uri, headers, body, params) without a full request, reusing
  `Context::from_parts`.
- `run_middleware(mw, ctx, handler).await -> Result<Response>` — execute a single
  middleware in isolation against the real `Next` / `MiddlewareChain` types, so a
  middleware's behavior (short-circuit, header injection, etc.) can be asserted.

### 7. Database test helpers (feature-gated: `testing` + backend)

- sqlx: `with_test_transaction(&pool, |tx| async { … }).await` — opens a
  transaction, runs the closure, and **always rolls back** (never commits), so
  tests never mutate persistent state.
- diesel: thin wrapper over diesel's `Connection::test_transaction` providing the
  same always-rollback semantics.
- Scope: rollback-isolation only. No schema management / migrations in v0.3.0.

### 8. Fixtures (deliberately minimal — YAGNI)

- `trait Fixture { async fn setup(&self, …); async fn teardown(&self, …); }` —
  optional lifecycle hook for seeding/cleanup.
- `load_fixture::<T: DeserializeOwned>(path) -> T` — load a typed fixture from a
  JSON file under a conventional `tests/fixtures/` dir.
- No fixture registry / dependency graph in this iteration.

### 9. Documentation

- Module-level docs on `ultimo::testing` with **doctests** that mirror the public
  examples (doctests double as the worked examples and run in CI via `cargo test --doc`).
- A "Testing Ultimo apps" guide page (markdown under `docs/`, linkable from the
  docs site) covering: setting up `TestClient`, asserting responses, testing
  middleware, and DB rollback testing.

## Testing strategy (for the utilities themselves)

- Unit tests per component (request builder construction, response assertion
  success/failure paths, macro behavior).
- Integration tests: a small example app exercised end-to-end via `TestClient`
  (routing, params, query, JSON round-trip, status codes, middleware).
- DB helpers tested against the sqlite backend (no server needed), asserting
  rollback leaves no rows.
- Doctests for every public example.
- All run under the existing CI gates (clippy `-D warnings`, fmt, the `testing`
  feature added to the relevant CI test invocations).

## Published-crate impact

All changes are **additive**: a new `testing` feature, new `ultimo::testing`
public items, and a new `pub Ultimo::oneshot`. No existing public signatures
change. `cargo-semver-checks` (CI) confirms no breakage. Ships in **v0.3.0**;
add a `CHANGELOG.md` entry under the v0.3.0 section.

## Out of scope (future follow-ups)

- Opt-in real-server (socket-level) test mode.
- Fixture dependency graphs / registries.
- DB migration/schema management in tests.
- Snapshot/`insta`-style response snapshotting (could pair with `insta` later).
