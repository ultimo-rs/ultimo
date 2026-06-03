# Testing Utilities Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship in-process testing utilities for Ultimo apps — `TestClient`, request builder, response assertions, macros, middleware helpers, DB rollback helpers, fixtures, and docs (issue #4).

**Architecture:** A new public seam `Ultimo::oneshot(Request<Full<Bytes>>) -> Response` runs the real middleware+routing pipeline in-process. The private `handle_request` is refactored to share a body-agnostic core (`dispatch_parts`) with `oneshot`. All ergonomic wrappers live in a new `ultimo::testing` module behind a `testing` Cargo feature. Everything is additive — no existing public signature changes.

**Tech Stack:** Rust, hyper 1.x (`http::Request`/`Response`, `http_body_util::Full`/`BodyExt`), tokio, serde/serde_json.

**Spec:** `docs/superpowers/specs/2026-06-03-testing-utilities-design.md`

---

## File structure

| File | Responsibility |
|---|---|
| `ultimo/src/context.rs` (modify) | Add `Request::from_parts` + `Context::from_parts`; refactor `new` to delegate |
| `ultimo/src/app.rs` (modify) | Add `Ultimo::oneshot`; extract `dispatch_parts` shared by `handle_request` |
| `ultimo/Cargo.toml` (modify) | Add `testing` feature |
| `ultimo/src/lib.rs` (modify) | `#[cfg(feature = "testing")] pub mod testing;` |
| `ultimo/src/testing/mod.rs` (create) | Module root; re-exports + `assert_json_eq!`/`assert_status!` macros |
| `ultimo/src/testing/client.rs` (create) | `TestClient` + `TestRequest` |
| `ultimo/src/testing/response.rs` (create) | `TestResponse` + assertions |
| `ultimo/src/testing/middleware.rs` (create) | `test_context()` builder + `run_middleware` |
| `ultimo/src/testing/database.rs` (create) | `with_test_transaction` (gated `sqlx`/`diesel`) |
| `ultimo/src/testing/fixtures.rs` (create) | `Fixture` trait + `load_fixture` |
| `ultimo/tests/testing_utils.rs` (create) | Integration tests exercising the public API |
| `docs/testing.md` (create) | "Testing Ultimo apps" guide |
| `.github/workflows/ci.yml` (modify) | Add `testing` feature to test invocations |
| `CHANGELOG.md` (modify) | v0.3.0 entry |

**Verification commands** (this crate's conventions):
- Unit tests: `cargo test -p ultimo --features "testing" --lib`
- Integration: `cargo test -p ultimo --features "testing" --test testing_utils`
- DB path: `cargo test -p ultimo --features "testing,sqlx-sqlite,diesel-sqlite" --lib`
- Lint gate: `cargo clippy -p ultimo --all-targets --features "testing" -- -D warnings`
- Doctests: `cargo test -p ultimo --features "testing" --doc`

---

## Task 1: Body-agnostic Context constructors

**Files:**
- Modify: `ultimo/src/context.rs:30-49` (`Request::new`) and `:161-174` (`Context::new`)
- Test: `ultimo/src/context.rs` (inline `#[cfg(test)] mod` at end)

- [ ] **Step 1: Write the failing test** — append to `ultimo/src/context.rs`:

```rust
#[cfg(test)]
mod from_parts_tests {
    use super::*;

    #[tokio::test]
    async fn request_from_parts_exposes_method_path_query_body() {
        let req = HyperRequest::builder()
            .method("POST")
            .uri("/users?team=core")
            .body(())
            .unwrap();
        let (parts, ()) = req.into_parts();
        let body = Bytes::from_static(br#"{"name":"ada"}"#);

        let r = Request::from_parts(parts, body, Params::new());

        assert_eq!(r.method(), &hyper::Method::POST);
        assert_eq!(r.path(), "/users");
        assert_eq!(r.query("team").as_deref(), Some("core"));
        assert_eq!(r.text().await.unwrap(), r#"{"name":"ada"}"#);
    }
}
```

> Note: confirm `Params::new()` exists; router.rs shows `Params` is the param map. If its constructor differs (e.g. `Params::default()`), use that — check `ultimo/src/router.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ultimo --lib from_parts_tests`
Expected: FAIL — `no function or associated item named from_parts found for struct Request`.

- [ ] **Step 3: Add `Request::from_parts` and refactor `Request::new`** — replace `Request::new` (context.rs:31-49) with:

```rust
    /// Build a Request from already-parsed parts and a buffered body.
    pub(crate) fn from_parts(
        parts: hyper::http::request::Parts,
        body: Bytes,
        params: Params,
    ) -> Self {
        Self {
            method: parts.method,
            uri: parts.uri,
            headers: parts.headers,
            params,
            body: Arc::new(RwLock::new(Some(body))),
        }
    }

    /// Create a new Request from a Hyper request and path parameters
    pub async fn new(req: HyperRequest<Incoming>, params: Params) -> Result<Self> {
        let (parts, body) = req.into_parts();
        let collected = body
            .collect()
            .await
            .map_err(|e| UltimoError::Internal(format!("Failed to read body: {}", e)))?;
        Ok(Self::from_parts(parts, collected.to_bytes(), params))
    }
```

- [ ] **Step 4: Add `Context::from_parts` and refactor `Context::new`** — replace `Context::new` (context.rs:162-174) with:

```rust
    /// Build a Context from already-parsed parts and a buffered body.
    pub(crate) fn from_parts(
        parts: hyper::http::request::Parts,
        body: Bytes,
        params: Params,
    ) -> Self {
        Self {
            req: Request::from_parts(parts, body, params),
            state: Arc::new(RwLock::new(HashMap::new())),
            response_status: Arc::new(RwLock::new(None)),
            response_headers: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "database")]
            database: None,
        }
    }

    /// Create a new context from a request and params
    pub async fn new(req: HyperRequest<Incoming>, params: Params) -> Result<Self> {
        let (parts, body) = req.into_parts();
        let collected = body
            .collect()
            .await
            .map_err(|e| UltimoError::Internal(format!("Failed to read body: {}", e)))?;
        Ok(Self::from_parts(parts, collected.to_bytes(), params))
    }
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p ultimo --lib from_parts_tests`
Expected: PASS.

- [ ] **Step 6: Confirm no regression**

Run: `cargo test -p ultimo --lib`
Expected: all prior unit tests still pass (107+).

- [ ] **Step 7: Commit**

```bash
git add ultimo/src/context.rs
git commit -m "feat(context): add body-agnostic from_parts constructors"
```

---

## Task 2: `Ultimo::oneshot` + shared `dispatch_parts`

**Files:**
- Modify: `ultimo/src/app.rs` — refactor `handle_request` (:259-383), add `dispatch_parts` + `oneshot`
- Test: `ultimo/src/app.rs` (inline test module)

- [ ] **Step 1: Write the failing test** — append to `ultimo/src/app.rs` test module (or create one):

```rust
#[cfg(test)]
mod oneshot_tests {
    use super::*;
    use http_body_util::{BodyExt, Full};
    use hyper::Request as HyperRequest;

    async fn body_string(resp: Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn oneshot_routes_and_returns_response() {
        let mut app = Ultimo::new_without_defaults();
        app.get("/ping", |ctx: Context| async move { ctx.text("pong").await });

        let req = HyperRequest::builder()
            .method("GET")
            .uri("/ping")
            .body(Full::new(bytes::Bytes::new()))
            .unwrap();

        let resp = app.oneshot(req).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(body_string(resp).await, "pong");
    }

    #[tokio::test]
    async fn oneshot_unknown_route_is_404() {
        let app = Ultimo::new_without_defaults();
        let req = HyperRequest::builder()
            .uri("/nope")
            .body(Full::new(bytes::Bytes::new()))
            .unwrap();
        assert_eq!(app.oneshot(req).await.status(), 404);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ultimo --lib oneshot_tests`
Expected: FAIL — `no method named oneshot found`.

- [ ] **Step 3: Extract `dispatch_parts` from `handle_request`** — In `app.rs`, change `handle_request` so that, after the WebSocket-upgrade check (the `#[cfg(feature = "websocket")]` block at :264-280), it collects the body and delegates. Replace everything from `// Parse method` (:282) through the end of the method body (:382) by moving that logic into a new method `dispatch_parts`, and make `handle_request` call it:

```rust
    async fn handle_request(&self, req: HyperRequest<Incoming>) -> Response {
        let path = req.uri().path().to_string();

        // WebSocket upgrade must run on the live Incoming request.
        #[cfg(feature = "websocket")]
        {
            if let Some(ws_handler) = self.websocket_routes.get(&path) {
                if req
                    .headers()
                    .get(hyper::header::UPGRADE)
                    .and_then(|v| v.to_str().ok())
                    .map(|v| v.eq_ignore_ascii_case("websocket"))
                    .unwrap_or(false)
                {
                    let upgrade = WebSocketUpgrade::new(req);
                    return ws_handler(upgrade);
                }
            }
        }

        let (parts, body) = req.into_parts();
        let bytes = match body.collect().await {
            Ok(c) => c.to_bytes(),
            Err(e) => {
                error!("Failed to read body: {}", e);
                return response::helpers::text("Internal Error").unwrap();
            }
        };
        self.dispatch_parts(parts, bytes).await
    }

    /// Run routing + middleware + handler against an already-buffered request.
    /// Shared by the live server (`handle_request`) and `oneshot`.
    async fn dispatch_parts(
        &self,
        parts: hyper::http::request::Parts,
        body: Bytes,
    ) -> Response {
        let method_str = parts.method.clone();
        let path = parts.uri.path().to_string();

        let method = match Method::from_hyper(&method_str) {
            Some(m) => m,
            None => {
                return response::helpers::error_response(&UltimoError::BadRequest(format!(
                    "Unsupported HTTP method: {}",
                    method_str
                )))
                .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap());
            }
        };

        // OPTIONS goes through middleware before routing (CORS preflight).
        if method_str == hyper::Method::OPTIONS {
            let ctx = Context::from_parts(parts, body, Params::new());
            let mut chain = MiddlewareChain::new();
            for middleware in &self.middleware {
                chain.push(middleware.clone());
            }
            let result = chain
                .execute(ctx, |_ctx| async move {
                    Ok(response::helpers::not_found()
                        .unwrap_or_else(|_| response::helpers::text("Not Found").unwrap()))
                })
                .await;
            return match result {
                Ok(response) => response,
                Err(err) => {
                    error!("Middleware error: {}", err);
                    response::helpers::error_response(&err)
                        .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap())
                }
            };
        }

        let (handler_id, params) = match self.router.find_route(method, &path) {
            Some(m) => m,
            None => {
                return response::helpers::not_found()
                    .unwrap_or_else(|_| response::helpers::text("Not Found").unwrap());
            }
        };

        #[cfg_attr(not(feature = "database"), allow(unused_mut))]
        let mut ctx = Context::from_parts(parts, body, params);

        #[cfg(feature = "database")]
        if let Some(ref db) = self.database {
            ctx.attach_database(db.clone());
        }

        let mut chain = MiddlewareChain::new();
        for middleware in &self.middleware {
            chain.push(middleware.clone());
        }
        let handler = self.handlers[handler_id].clone();
        let result = chain
            .execute(ctx, move |ctx| async move { handler(ctx).await })
            .await;

        match result {
            Ok(response) => response,
            Err(err) => {
                error!("Handler error: {}", err);
                response::helpers::error_response(&err)
                    .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap())
            }
        }
    }

    /// Dispatch a fully-buffered request through the app in-process (no socket).
    /// The primary seam for testing and embedding.
    pub async fn oneshot(&self, req: HyperRequest<http_body_util::Full<Bytes>>) -> Response {
        let (parts, body) = req.into_parts();
        // Full<Bytes> is infallible.
        let bytes = body
            .collect()
            .await
            .map(|c| c.to_bytes())
            .unwrap_or_default();
        self.dispatch_parts(parts, bytes).await
    }
```

> Imports to ensure at the top of `app.rs`: `use bytes::Bytes;`, `use crate::context::Context;` (already present), `use crate::router::{Method, Params};` (confirm `Params` is imported — add if missing), `use http_body_util::BodyExt;`. The `Context::from_parts`/`Request::from_parts` are `pub(crate)` from Task 1.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p ultimo --lib oneshot_tests`
Expected: PASS (both cases).

- [ ] **Step 5: Confirm no regression across features**

Run: `cargo test -p ultimo --lib` then `cargo test -p ultimo --features "websocket,test-helpers" --tests`
Expected: all pass (WebSocket upgrade path unchanged).

- [ ] **Step 6: Commit**

```bash
git add ultimo/src/app.rs
git commit -m "feat(app): add Ultimo::oneshot in-process dispatch seam"
```

---

## Task 3: `testing` feature + module skeleton

**Files:**
- Modify: `ultimo/Cargo.toml:48-55` (features), `ultimo/src/lib.rs:43-44`
- Create: `ultimo/src/testing/mod.rs`

- [ ] **Step 1: Add the feature** — in `ultimo/Cargo.toml` under `[features]`, after `test-helpers = []`:

```toml
# Testing utilities for Ultimo apps (TestClient, assertions, fixtures)
testing = []
```

- [ ] **Step 2: Declare the module** — in `ultimo/src/lib.rs`, after the `validation` mod declaration (near line 38):

```rust
#[cfg(feature = "testing")]
pub mod testing;
```

- [ ] **Step 3: Create the module root** — `ultimo/src/testing/mod.rs`:

```rust
//! Testing utilities for Ultimo applications.
//!
//! Enable with `ultimo = { version = "…", features = ["testing"] }` under
//! `[dev-dependencies]`. The [`TestClient`] drives your app in-process — no
//! socket, fully deterministic.
//!
//! ```
//! # use ultimo::{Ultimo, Context};
//! # use ultimo::testing::TestClient;
//! # #[tokio::main] async fn main() {
//! let mut app = Ultimo::new_without_defaults();
//! app.get("/ping", |ctx: Context| async move { ctx.text("pong").await });
//!
//! let client = TestClient::new(app);
//! let res = client.get("/ping").send().await;
//! res.assert_ok();
//! assert_eq!(res.text(), "pong");
//! # }
//! ```

mod client;
mod response;
mod middleware;
mod fixtures;
#[cfg(any(feature = "sqlx", feature = "diesel"))]
mod database;

pub use client::{TestClient, TestRequest};
pub use fixtures::{load_fixture, Fixture};
pub use middleware::{run_middleware, test_context, TestContextBuilder};
pub use response::TestResponse;

#[cfg(any(feature = "sqlx", feature = "diesel"))]
pub use database::with_test_transaction;
```

> Macros `assert_json_eq!`/`assert_status!` are added in Task 6 (they use `#[macro_export]`, so they live at crate root regardless of this module).

- [ ] **Step 4: Verify it compiles (empty submodules will fail until later tasks)** — create empty stubs so the skeleton compiles now:

Create `ultimo/src/testing/client.rs`, `response.rs`, `middleware.rs`, `fixtures.rs` each containing only `//! stub` for now. (They are filled in Tasks 4-9.) Then:

Run: `cargo build -p ultimo --features testing`
Expected: FAILS on missing `TestClient` etc. — that's fine; the skeleton lands incrementally. To keep this task self-contained, instead make `mod.rs` re-exports conditional by commenting them out until each task adds the type. **Simpler:** defer the `pub use` lines — add each `pub use` in the task that creates the type. Replace Step 3's `pub use` block with just the `mod` declarations for now.

- [ ] **Step 5: Commit**

```bash
git add ultimo/Cargo.toml ultimo/src/lib.rs ultimo/src/testing/
git commit -m "feat(testing): add testing feature + module skeleton"
```

---

## Task 4: `TestClient` + `TestRequest`

**Files:**
- Create/replace: `ultimo/src/testing/client.rs`
- Add `pub use client::{TestClient, TestRequest};` to `mod.rs`
- Test: `ultimo/tests/testing_utils.rs`

- [ ] **Step 1: Write the failing integration test** — create `ultimo/tests/testing_utils.rs`:

```rust
#![cfg(feature = "testing")]

use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

fn app() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.get("/hello", |ctx: Context| async move { ctx.text("hi").await });
    app.post("/echo", |ctx: Context| async move {
        let body: serde_json::Value = ctx.req.json().await.unwrap_or_default();
        ctx.json(body).await
    });
    app
}

#[tokio::test]
async fn get_returns_handler_body() {
    let client = TestClient::new(app());
    let res = client.get("/hello").send().await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.text(), "hi");
}

#[tokio::test]
async fn post_json_round_trips() {
    let client = TestClient::new(app());
    let res = client
        .post("/echo")
        .json(&serde_json::json!({ "n": 1 }))
        .send()
        .await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.json::<serde_json::Value>(), serde_json::json!({ "n": 1 }));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p ultimo --features testing --test testing_utils`
Expected: FAIL — unresolved import `ultimo::testing::TestClient`.

- [ ] **Step 3: Implement `client.rs`**

```rust
//! In-process test client and request builder.

use crate::testing::response::TestResponse;
use crate::Ultimo;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{HeaderMap, Method, Request as HyperRequest};
use serde::Serialize;

/// Drives an [`Ultimo`] app in-process for testing.
pub struct TestClient {
    app: Ultimo,
}

impl TestClient {
    /// Wrap a built app.
    pub fn new(app: Ultimo) -> Self {
        Self { app }
    }

    /// Start building a request with an explicit method.
    pub fn request(&self, method: Method, path: &str) -> TestRequest<'_> {
        TestRequest {
            client: self,
            method,
            path: path.to_string(),
            headers: HeaderMap::new(),
            query: Vec::new(),
            body: Bytes::new(),
        }
    }

    pub fn get(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::GET, path)
    }
    pub fn post(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::POST, path)
    }
    pub fn put(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::PUT, path)
    }
    pub fn delete(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::DELETE, path)
    }
    pub fn patch(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::PATCH, path)
    }
    pub fn head(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::HEAD, path)
    }
    pub fn options(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::OPTIONS, path)
    }
}

/// A fluent request builder. Terminate with [`send`](TestRequest::send).
pub struct TestRequest<'a> {
    client: &'a TestClient,
    method: Method,
    path: String,
    headers: HeaderMap,
    query: Vec<(String, String)>,
    body: Bytes,
}

impl<'a> TestRequest<'a> {
    pub fn header(mut self, name: &str, value: &str) -> Self {
        let n = hyper::header::HeaderName::from_bytes(name.as_bytes()).expect("valid header name");
        let v = hyper::header::HeaderValue::from_str(value).expect("valid header value");
        self.headers.insert(n, v);
        self
    }

    pub fn bearer(self, token: &str) -> Self {
        self.header("authorization", &format!("Bearer {token}"))
    }

    pub fn query(mut self, pairs: &[(&str, &str)]) -> Self {
        for (k, v) in pairs {
            self.query.push((k.to_string(), v.to_string()));
        }
        self
    }

    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    pub fn text(self, text: &str) -> Self {
        self.body(Bytes::copy_from_slice(text.as_bytes()))
    }

    pub fn json<T: Serialize>(mut self, value: &T) -> Self {
        let bytes = serde_json::to_vec(value).expect("serializable JSON body");
        self.body = Bytes::from(bytes);
        self.header("content-type", "application/json")
    }

    /// Dispatch the request through the app in-process.
    pub async fn send(self) -> TestResponse {
        let uri = build_uri(&self.path, &self.query);
        let mut builder = HyperRequest::builder().method(self.method).uri(uri);
        if let Some(h) = builder.headers_mut() {
            *h = self.headers;
        }
        let req = builder
            .body(Full::new(self.body))
            .expect("valid test request");
        let resp = self.client.app.oneshot(req).await;
        TestResponse::from_response(resp).await
    }
}

fn build_uri(path: &str, query: &[(String, String)]) -> String {
    if query.is_empty() {
        return path.to_string();
    }
    let qs = query
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{path}?{qs}")
}

fn urlencode(s: &str) -> String {
    // Minimal: encode space and &/=; sufficient for test query values.
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
}
```

- [ ] **Step 4: Add re-export** — in `ultimo/src/testing/mod.rs` add `pub use client::{TestClient, TestRequest};` (and `pub use response::TestResponse;` if not already present from Task 5 ordering — if Task 5 not done yet, temporarily stub `TestResponse`; recommended to do Task 5 immediately after).

- [ ] **Step 5: Run (will fail until Task 5 provides `TestResponse`)**

Run: `cargo test -p ultimo --features testing --test testing_utils`
Expected: FAIL on missing `TestResponse` — proceed to Task 5, then both pass together.

- [ ] **Step 6: Commit** (after Task 5 makes it green)

```bash
git add ultimo/src/testing/client.rs ultimo/src/testing/mod.rs ultimo/tests/testing_utils.rs
git commit -m "feat(testing): TestClient + TestRequest builder"
```

---

## Task 5: `TestResponse` + assertions

**Files:**
- Create/replace: `ultimo/src/testing/response.rs`
- Add `pub use response::TestResponse;` to `mod.rs`
- Test: append to `ultimo/tests/testing_utils.rs`

- [ ] **Step 1: Write the failing test** — append to `ultimo/tests/testing_utils.rs`:

```rust
#[tokio::test]
async fn assertions_pass_for_ok_text() {
    let client = TestClient::new(app());
    let res = client.get("/hello").send().await;
    res.assert_ok().assert_status(200).assert_text("hi");
}

#[tokio::test]
#[should_panic(expected = "expected status 404")]
async fn assert_status_panics_on_mismatch() {
    let client = TestClient::new(app());
    client.get("/hello").send().await.assert_status(404);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p ultimo --features testing --test testing_utils assertions_pass_for_ok_text`
Expected: FAIL — no method `assert_ok`.

- [ ] **Step 3: Implement `response.rs`**

```rust
//! Buffered response wrapper with assertion helpers.

use crate::response::Response;
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{HeaderMap, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// A fully-buffered response. All accessors are synchronous.
pub struct TestResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl TestResponse {
    pub(crate) async fn from_response(resp: Response) -> Self {
        let (parts, body) = resp.into_parts();
        let body = body.collect().await.map(|c| c.to_bytes()).unwrap_or_default();
        Self {
            status: parts.status,
            headers: parts.headers,
            body,
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }
    pub fn bytes(&self) -> &Bytes {
        &self.body
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
    pub fn json<T: DeserializeOwned>(&self) -> T {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|e| panic!("response body is not valid JSON for the target type: {e}\nbody: {}", self.text()))
    }

    pub fn assert_status(&self, code: u16) -> &Self {
        assert_eq!(
            self.status.as_u16(),
            code,
            "expected status {code}, got {}",
            self.status.as_u16()
        );
        self
    }
    pub fn assert_ok(&self) -> &Self {
        self.assert_status(200)
    }
    pub fn assert_status_is_success(&self) -> &Self {
        assert!(
            self.status.is_success(),
            "expected 2xx status, got {}",
            self.status.as_u16()
        );
        self
    }
    pub fn assert_header(&self, name: &str, value: &str) -> &Self {
        assert_eq!(self.header(name), Some(value), "header {name} mismatch");
        self
    }
    pub fn assert_text(&self, expected: &str) -> &Self {
        assert_eq!(self.text(), expected, "response text mismatch");
        self
    }
    pub fn assert_json<T: Serialize>(&self, expected: &T) -> &Self {
        let got: serde_json::Value = serde_json::from_slice(&self.body)
            .unwrap_or_else(|e| panic!("response is not JSON: {e}"));
        let want = serde_json::to_value(expected).expect("expected is serializable");
        assert_eq!(got, want, "response JSON mismatch");
        self
    }
}
```

- [ ] **Step 4: Add re-export** — ensure `ultimo/src/testing/mod.rs` has `pub use response::TestResponse;`.

- [ ] **Step 5: Run to verify pass**

Run: `cargo test -p ultimo --features testing --test testing_utils`
Expected: PASS (Task 4 + Task 5 tests all green).

- [ ] **Step 6: Commit**

```bash
git add ultimo/src/testing/response.rs ultimo/src/testing/mod.rs ultimo/tests/testing_utils.rs
git commit -m "feat(testing): TestResponse with assertion helpers"
```

---

## Task 6: `assert_json_eq!` and `assert_status!` macros

**Files:**
- Modify: `ultimo/src/testing/mod.rs` (add `#[macro_export]` macros)
- Test: append to `ultimo/tests/testing_utils.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn macros_work() {
    let client = TestClient::new(app());
    let res = client
        .post("/echo")
        .json(&serde_json::json!({ "ok": true }))
        .send()
        .await;
    ultimo::assert_status!(res, 200);
    ultimo::assert_json_eq!(res.json::<serde_json::Value>(), serde_json::json!({ "ok": true }));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p ultimo --features testing --test testing_utils macros_work`
Expected: FAIL — cannot find macro `assert_status`.

- [ ] **Step 3: Implement the macros** — append to `ultimo/src/testing/mod.rs`:

```rust
/// Assert two values are equal as JSON, with a readable diff on failure.
#[macro_export]
macro_rules! assert_json_eq {
    ($actual:expr, $expected:expr $(,)?) => {{
        let actual = ::serde_json::to_value(&$actual).expect("actual is serializable");
        let expected = ::serde_json::to_value(&$expected).expect("expected is serializable");
        assert_eq!(
            actual, expected,
            "JSON mismatch\n  actual:   {}\n  expected: {}",
            actual, expected
        );
    }};
}

/// Assert a [`TestResponse`](crate::testing::TestResponse) has the given status.
#[macro_export]
macro_rules! assert_status {
    ($res:expr, $code:expr $(,)?) => {{
        $res.assert_status($code);
    }};
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p ultimo --features testing --test testing_utils macros_work`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add ultimo/src/testing/mod.rs ultimo/tests/testing_utils.rs
git commit -m "feat(testing): assert_json_eq! and assert_status! macros"
```

---

## Task 7: Middleware testing helpers

**Files:**
- Create/replace: `ultimo/src/testing/middleware.rs`
- Add re-exports to `mod.rs`
- Test: append to `ultimo/tests/testing_utils.rs`

- [ ] **Step 1: Write the failing test**

```rust
use ultimo::testing::{run_middleware, test_context};

#[tokio::test]
async fn middleware_can_short_circuit() {
    // A middleware that blocks unauthorized requests.
    let mw = |ctx: ultimo::Context, next: ultimo::middleware::Next<'_>| {
        Box::pin(async move {
            if ctx.req.header("authorization").is_none() {
                return ultimo::response::ResponseBuilder::new()
                    .status(401)
                    .text("unauthorized")
                    .build();
            }
            next(ctx).await
        }) as std::pin::Pin<Box<dyn std::future::Future<Output = ultimo::Result<ultimo::response::Response>> + Send>>
    };

    let ctx = test_context().method("GET").path("/private").build();
    let res = run_middleware(mw, ctx, |ctx: ultimo::Context| async move {
        ctx.text("secret").await
    })
    .await
    .unwrap();
    assert_eq!(res.status(), 401);
}
```

> Confirm the exact `Next` type and `ResponseBuilder` API against `ultimo/src/middleware.rs:14` and `ultimo/src/response.rs`. The middleware closure shape must match `IntoMiddleware`'s blanket impl.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p ultimo --features testing --test testing_utils middleware_can_short_circuit`
Expected: FAIL — unresolved `test_context`/`run_middleware`.

- [ ] **Step 3: Implement `middleware.rs`**

```rust
//! Helpers for unit-testing middleware in isolation.

use crate::context::Context;
use crate::error::Result;
use crate::middleware::{IntoMiddleware, MiddlewareChain};
use crate::response::Response;
use bytes::Bytes;
use hyper::Request as HyperRequest;
use std::future::Future;

/// Build a [`Context`] for tests without a live request.
pub fn test_context() -> TestContextBuilder {
    TestContextBuilder {
        method: "GET".to_string(),
        path: "/".to_string(),
        headers: Vec::new(),
        body: Bytes::new(),
    }
}

/// Builder produced by [`test_context`].
pub struct TestContextBuilder {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Bytes,
}

impl TestContextBuilder {
    pub fn method(mut self, m: &str) -> Self {
        self.method = m.to_string();
        self
    }
    pub fn path(mut self, p: &str) -> Self {
        self.path = p.to_string();
        self
    }
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    pub fn build(self) -> Context {
        let mut builder = HyperRequest::builder().method(self.method.as_str()).uri(&self.path);
        for (n, v) in &self.headers {
            builder = builder.header(n, v);
        }
        let req = builder.body(()).expect("valid test request parts");
        let (parts, ()) = req.into_parts();
        Context::from_parts(parts, self.body, crate::router::Params::new())
    }
}

/// Run a single middleware against a context and a terminal handler.
pub async fn run_middleware<M, F, Fut>(middleware: M, ctx: Context, handler: F) -> Result<Response>
where
    M: IntoMiddleware,
    F: FnOnce(Context) -> Fut + Send + 'static,
    Fut: Future<Output = Result<Response>> + Send + 'static,
{
    let mut chain = MiddlewareChain::new();
    chain.push(middleware.into_middleware());
    chain.execute(ctx, handler).await
}
```

> `Context::from_parts` and `Params::new()` come from Task 1. Confirm `MiddlewareChain::new`/`push`/`execute` signatures (middleware.rs:44-79). If `Params` has no `new()`, use `Params::default()`.

- [ ] **Step 4: Add re-exports** — in `mod.rs`: `pub use middleware::{run_middleware, test_context, TestContextBuilder};`

- [ ] **Step 5: Run to verify pass**

Run: `cargo test -p ultimo --features testing --test testing_utils middleware_can_short_circuit`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add ultimo/src/testing/middleware.rs ultimo/src/testing/mod.rs ultimo/tests/testing_utils.rs
git commit -m "feat(testing): middleware testing helpers"
```

---

## Task 8: Database rollback helpers

**Files:**
- Create/replace: `ultimo/src/testing/database.rs`
- Add gated re-export to `mod.rs`
- Test: `ultimo/tests/testing_db.rs` (sqlite, no server)

- [ ] **Step 1: Write the failing test** — create `ultimo/tests/testing_db.rs`:

```rust
#![cfg(all(feature = "testing", feature = "sqlx-sqlite"))]

use ultimo::testing::with_test_transaction;

#[tokio::test]
async fn transaction_always_rolls_back() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();

    with_test_transaction(&pool, |tx| async move {
        sqlx::query("INSERT INTO t (id) VALUES (1)")
            .execute(&mut **tx)
            .await
            .unwrap();
        Ok(())
    })
    .await
    .unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM t")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "rows must not persist after rollback");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p ultimo --features "testing,sqlx-sqlite" --test testing_db`
Expected: FAIL — unresolved `with_test_transaction`.

- [ ] **Step 3: Implement `database.rs`** (sqlx path; diesel path analogous)

```rust
//! Database test helpers — always roll back so tests never mutate state.

use crate::error::Result;
use std::future::Future;

/// Run `f` inside a transaction that is **always rolled back**.
#[cfg(feature = "sqlx")]
pub async fn with_test_transaction<'a, DB, F, Fut, T>(
    pool: &sqlx::Pool<DB>,
    f: F,
) -> Result<T>
where
    DB: sqlx::Database,
    F: FnOnce(&mut sqlx::Transaction<'a, DB>) -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| crate::error::UltimoError::Internal(format!("begin tx: {e}")))?;
    let result = f(&mut tx).await;
    // Always roll back, regardless of result.
    let _ = tx.rollback().await;
    result
}
```

> Diesel variant (gate `#[cfg(feature = "diesel")]`): wrap `conn.test_transaction(|| { … })`, which never commits. Add it alongside if `diesel` is enabled. Verify the exact `sqlx::Transaction` deref pattern (`&mut **tx`) against the sqlx version in `Cargo.lock` (0.7.x).

- [ ] **Step 4: Add gated re-export** — in `mod.rs` (already shown): `#[cfg(any(feature = "sqlx", feature = "diesel"))] pub use database::with_test_transaction;`

- [ ] **Step 5: Run to verify pass**

Run: `cargo test -p ultimo --features "testing,sqlx-sqlite" --test testing_db`
Expected: PASS (count == 0).

- [ ] **Step 6: Commit**

```bash
git add ultimo/src/testing/database.rs ultimo/src/testing/mod.rs ultimo/tests/testing_db.rs
git commit -m "feat(testing): DB transaction/rollback helper"
```

---

## Task 9: Fixtures

**Files:**
- Create/replace: `ultimo/src/testing/fixtures.rs`
- Add re-exports to `mod.rs`
- Test: append to `ultimo/tests/testing_utils.rs` + a fixture file

- [ ] **Step 1: Create a fixture file** — `ultimo/tests/fixtures/user.json`:

```json
{ "id": 1, "name": "Ada" }
```

- [ ] **Step 2: Write the failing test** — append to `testing_utils.rs`:

```rust
#[derive(serde::Deserialize, PartialEq, Debug)]
struct UserFixture {
    id: u32,
    name: String,
}

#[test]
fn load_fixture_parses_json() {
    let u: UserFixture = ultimo::testing::load_fixture("tests/fixtures/user.json");
    assert_eq!(u, UserFixture { id: 1, name: "Ada".into() });
}
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p ultimo --features testing --test testing_utils load_fixture_parses_json`
Expected: FAIL — unresolved `load_fixture`.

- [ ] **Step 4: Implement `fixtures.rs`**

```rust
//! Minimal fixture support.

use serde::de::DeserializeOwned;
use std::future::Future;
use std::path::Path;

/// Load a typed fixture from a JSON file. Panics with a clear message on failure
/// (fixtures are test inputs — a bad path is a test bug).
pub fn load_fixture<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    let path = path.as_ref();
    let data = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&data)
        .unwrap_or_else(|e| panic!("failed to parse fixture {}: {e}", path.display()))
}

/// Optional setup/teardown lifecycle for seeding test data.
pub trait Fixture {
    /// Seed data before a test.
    fn setup(&self) -> impl Future<Output = ()> + Send;
    /// Clean up after a test.
    fn teardown(&self) -> impl Future<Output = ()> + Send;
}
```

- [ ] **Step 5: Add re-exports** — in `mod.rs`: `pub use fixtures::{load_fixture, Fixture};`

- [ ] **Step 6: Run to verify pass**

Run: `cargo test -p ultimo --features testing --test testing_utils load_fixture_parses_json`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add ultimo/src/testing/fixtures.rs ultimo/src/testing/mod.rs ultimo/tests/testing_utils.rs ultimo/tests/fixtures/
git commit -m "feat(testing): minimal fixture loader + Fixture trait"
```

---

## Task 10: Documentation + guide

**Files:**
- Modify: `ultimo/src/testing/mod.rs` (already has a doctest from Task 3; expand)
- Create: `docs/testing.md`

- [ ] **Step 1: Verify doctests run**

Run: `cargo test -p ultimo --features testing --doc`
Expected: the module doctest from Task 3 compiles and passes. If it fails, fix the example to match the final API.

- [ ] **Step 2: Write the guide** — create `docs/testing.md` covering: enabling the `testing` feature (dev-dependency), `TestClient` setup, request building (`json`/`query`/`header`/`bearer`), response assertions, `assert_json_eq!`, middleware testing via `test_context`/`run_middleware`, and DB rollback testing via `with_test_transaction`. Use the exact APIs from Tasks 4-9. Include at least one full runnable example per section.

- [ ] **Step 3: Add doctests for the main entry points** — ensure `TestClient`, `TestResponse`, and `with_test_transaction` each have a `///` doc example. Re-run:

Run: `cargo test -p ultimo --features testing --doc`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add ultimo/src/testing/ docs/testing.md
git commit -m "docs(testing): module docs, doctests, and testing guide"
```

---

## Task 11: CI wiring + CHANGELOG

**Files:**
- Modify: `.github/workflows/ci.yml` (the `test` and `test-db` jobs)
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add the `testing` feature to CI test runs** — in `ci.yml`, the `test` job, after the integration-tests step, add:

```yaml
      - name: Testing-utilities tests
        run: cargo test -p ultimo --features "testing" --test testing_utils --doc
```

And in `test-db`, after the sqlite step:

```yaml
      - name: Testing-utilities DB tests (sqlite)
        run: cargo test -p ultimo --features "testing,sqlx-sqlite" --test testing_db
```

Also extend the clippy invocation in the `lint` job's feature build to include `testing`:
`cargo clippy -p ultimo --all-targets --features "websocket,test-helpers,testing" -- -D warnings`

- [ ] **Step 2: Validate YAML**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml')); print('ok')"`
Expected: `ok`.

- [ ] **Step 3: Add CHANGELOG entry** — under a new `## [0.3.0] - Unreleased` section in `CHANGELOG.md`:

```markdown
### Added
- Testing utilities (`testing` feature): in-process `TestClient`, request builder,
  `TestResponse` assertions, `assert_json_eq!`/`assert_status!` macros, middleware
  test helpers, database transaction/rollback helper, and fixture loading. (#4)
- `Ultimo::oneshot` — dispatch a buffered request through the app in-process.
```

- [ ] **Step 4: Final full verification**

Run:
```bash
cargo fmt --all --check
cargo clippy -p ultimo --all-targets --features "websocket,test-helpers,testing" -- -D warnings
cargo test -p ultimo --lib
cargo test -p ultimo --features "testing" --test testing_utils --doc
cargo test -p ultimo --features "testing,sqlx-sqlite" --test testing_db
```
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml CHANGELOG.md
git commit -m "ci+docs: run testing-utilities tests; changelog for v0.3.0"
```

---

## Notes for the implementer

- `Params::new()` vs `Params::default()` — verify in `ultimo/src/router.rs` and use whichever exists (it's used in Tasks 1, 7).
- The middleware closure shape in Task 7 must satisfy the `IntoMiddleware` blanket impl (`middleware.rs:32-42`). If the inline closure type inference fights you, define the middleware as a named `async fn`-style boxed closure as that impl expects.
- sqlx `Transaction` deref (`&mut **tx`) is version-sensitive; this repo is on sqlx 0.7.x (see `Cargo.lock`).
- Keep every new public item documented — `missing_docs` may be enforced and docs.rs renders them.
