# Session Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cookie-based session management for Ultimo — core cookie helper + a `session` feature providing `SessionStore`/`MemoryStore`, a `Session` handle on Context, and secure session middleware (issue #3, core slice).

**Architecture:** Cookies live in core (hand-rolled, no new dep) with a `Vec<Set-Cookie>` slot on Context flushed onto the final `Response` in `app.rs::dispatch_parts`. Sessions sit behind a `session` Cargo feature as middleware over an `#[async_trait] SessionStore` (the one extension point), with `MemoryStore` as the first backend. Security is designed in (see spec).

**Tech Stack:** Rust, hyper 1.x, tokio, serde_json (existing), `async-trait` + `base64` (existing deps), `getrandom` 0.3 (new, session-gated).

**Spec:** `docs/superpowers/specs/2026-06-03-session-management-design.md` (read its Security section before Task 7).

---

## File structure

| File | Responsibility |
|---|---|
| `ultimo/src/cookie.rs` (create) | `Cookie`, `CookieOptions`, `SameSite`, parse + Set-Cookie formatting (validated) |
| `ultimo/src/context.rs` (modify) | `set_cookies` slot + `cookie/cookies/set_cookie/remove_cookie`; `session` slot + `session()` (gated) |
| `ultimo/src/app.rs` (modify) | flush queued `Set-Cookie` onto the response in `dispatch_parts` |
| `ultimo/src/lib.rs` (modify) | `pub mod cookie;` + `#[cfg(feature="session")] pub mod session;` |
| `ultimo/Cargo.toml` (modify) | `session` feature + optional `getrandom` |
| `ultimo/src/session/mod.rs` (create) | `Session` handle + `SessionData` + re-exports |
| `ultimo/src/session/store.rs` (create) | `SessionStore` trait + `MemoryStore` |
| `ultimo/src/session/config.rs` (create) | `SessionConfig` (+ validation) |
| `ultimo/src/session/middleware.rs` (create) | `session(store, config)` middleware |
| `ultimo/tests/cookie.rs` (create) | cookie integration tests |
| `ultimo/tests/session.rs` (create) | session + security integration tests (uses `testing`) |

**Verification commands:**
- Cookies (core): `cargo test -p ultimo --lib cookie` / `cargo test -p ultimo --test cookie`
- Sessions: `cargo test -p ultimo --features "session" --lib`
- Session integration: `cargo test -p ultimo --features "session,testing" --test session`
- Lint: `cargo clippy -p ultimo --all-targets --features "session,testing,websocket,test-helpers" -- -D warnings`

---

## Task 1: Cookie primitives (core)

**Files:** Create `ultimo/src/cookie.rs`; modify `ultimo/src/lib.rs`.

- [ ] **Step 1: Declare the module** — in `ultimo/src/lib.rs`, near the other `pub mod` lines, add:

```rust
pub mod cookie;
```

- [ ] **Step 2: Write `ultimo/src/cookie.rs` with failing-test-first content**

```rust
//! HTTP cookie parsing and `Set-Cookie` formatting (RFC 6265).

use crate::error::{Result, UltimoError};
use std::collections::HashMap;

/// `SameSite` cookie attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl SameSite {
    fn as_str(&self) -> &'static str {
        match self {
            SameSite::Strict => "Strict",
            SameSite::Lax => "Lax",
            SameSite::None => "None",
        }
    }
}

/// Attributes for a `Set-Cookie`.
#[derive(Debug, Clone, Default)]
pub struct CookieOptions {
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<SameSite>,
    /// Max-Age in seconds.
    pub max_age: Option<i64>,
    pub path: Option<String>,
    pub domain: Option<String>,
}

/// A cookie to emit via `Set-Cookie`.
#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub options: CookieOptions,
}

impl Cookie {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            options: CookieOptions::default(),
        }
    }

    pub fn http_only(mut self, v: bool) -> Self {
        self.options.http_only = v;
        self
    }
    pub fn secure(mut self, v: bool) -> Self {
        self.options.secure = v;
        self
    }
    pub fn same_site(mut self, v: SameSite) -> Self {
        self.options.same_site = Some(v);
        self
    }
    pub fn max_age(mut self, secs: i64) -> Self {
        self.options.max_age = Some(secs);
        self
    }
    pub fn path(mut self, p: impl Into<String>) -> Self {
        self.options.path = Some(p.into());
        self
    }
    pub fn domain(mut self, d: impl Into<String>) -> Self {
        self.options.domain = Some(d.into());
        self
    }

    /// Format as a `Set-Cookie` header value. Rejects names/values containing
    /// control characters (header-injection guard).
    pub fn to_set_cookie_string(&self) -> Result<String> {
        validate_token(&self.name)?;
        validate_value(&self.value)?;
        let mut s = format!("{}={}", self.name, self.value);
        if let Some(p) = &self.options.path {
            validate_value(p)?;
            s.push_str(&format!("; Path={p}"));
        }
        if let Some(d) = &self.options.domain {
            validate_value(d)?;
            s.push_str(&format!("; Domain={d}"));
        }
        if let Some(m) = self.options.max_age {
            s.push_str(&format!("; Max-Age={m}"));
        }
        if let Some(ss) = self.options.same_site {
            s.push_str(&format!("; SameSite={}", ss.as_str()));
        }
        if self.options.secure {
            s.push_str("; Secure");
        }
        if self.options.http_only {
            s.push_str("; HttpOnly");
        }
        Ok(s)
    }
}

fn has_ctl(s: &str) -> bool {
    s.bytes().any(|b| b < 0x20 || b == 0x7f)
}

fn validate_token(name: &str) -> Result<()> {
    if name.is_empty() || has_ctl(name) || name.contains([';', '=', ' ', '\t']) {
        return Err(UltimoError::BadRequest(format!("invalid cookie name: {name:?}")));
    }
    Ok(())
}

fn validate_value(value: &str) -> Result<()> {
    if has_ctl(value) || value.contains([';', '\r', '\n']) {
        return Err(UltimoError::BadRequest("invalid cookie value".to_string()));
    }
    Ok(())
}

/// Parse a request `Cookie:` header into name → value pairs.
pub fn parse_cookie_header(header: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for pair in header.split(';') {
        let pair = pair.trim();
        if let Some((k, v)) = pair.split_once('=') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_cookies() {
        let m = parse_cookie_header("a=1; b=2;  c=3");
        assert_eq!(m.get("a").map(String::as_str), Some("1"));
        assert_eq!(m.get("b").map(String::as_str), Some("2"));
        assert_eq!(m.get("c").map(String::as_str), Some("3"));
    }

    #[test]
    fn formats_all_attributes() {
        let c = Cookie::new("sid", "abc")
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax)
            .max_age(3600)
            .path("/");
        let s = c.to_set_cookie_string().unwrap();
        assert!(s.starts_with("sid=abc"));
        assert!(s.contains("; Path=/"));
        assert!(s.contains("; Max-Age=3600"));
        assert!(s.contains("; SameSite=Lax"));
        assert!(s.contains("; Secure"));
        assert!(s.contains("; HttpOnly"));
    }

    #[test]
    fn rejects_header_injection() {
        let c = Cookie::new("sid", "abc\r\nSet-Cookie: evil=1");
        assert!(c.to_set_cookie_string().is_err());
    }
}
```

- [ ] **Step 3: Run tests** — `cargo test -p ultimo --lib cookie` → 3 pass.
- [ ] **Step 4: clippy + fmt** — `cargo clippy -p ultimo --lib -- -D warnings` clean; `cargo fmt --all`.
- [ ] **Step 5: Commit**

```bash
git add ultimo/src/cookie.rs ultimo/src/lib.rs
git commit -m "feat(cookie): RFC6265 cookie parsing + Set-Cookie formatting"
```

---

## Task 2: Context cookie API + response flush

**Files:** Modify `ultimo/src/context.rs` (add `set_cookies` slot + methods), `ultimo/src/app.rs` (flush in `dispatch_parts`).

- [ ] **Step 1: Add the slot to `Context`** — in `context.rs`, the `Context` struct (around line 150-159) add a field:

```rust
    set_cookies: Arc<RwLock<Vec<String>>>,
```

Initialize it in BOTH `Context::from_parts` (the constructor added earlier) — add `set_cookies: Arc::new(RwLock::new(Vec::new())),` alongside the other field inits.

- [ ] **Step 2: Add cookie methods** — in the `impl Context` block (near `set`/`get`), add:

```rust
    /// Read a request cookie by name.
    pub fn cookie(&self, name: &str) -> Option<String> {
        self.req
            .header("cookie")
            .and_then(|h| crate::cookie::parse_cookie_header(&h).remove(name))
    }

    /// All request cookies.
    pub fn cookies(&self) -> std::collections::HashMap<String, String> {
        self.req
            .header("cookie")
            .map(|h| crate::cookie::parse_cookie_header(&h))
            .unwrap_or_default()
    }

    /// Queue a `Set-Cookie` for the response. Errors if the cookie is invalid.
    pub async fn set_cookie(&self, cookie: crate::cookie::Cookie) -> Result<()> {
        let s = cookie.to_set_cookie_string()?;
        self.set_cookies.write().await.push(s);
        Ok(())
    }

    /// Queue a deletion of the named cookie (Max-Age=0).
    pub async fn remove_cookie(&self, name: &str) -> Result<()> {
        let c = crate::cookie::Cookie::new(name, "").max_age(0).path("/");
        self.set_cookie(c).await
    }

    /// Drain queued Set-Cookie header values (used by the dispatcher).
    pub(crate) async fn take_set_cookies(&self) -> Vec<String> {
        std::mem::take(&mut *self.set_cookies.write().await)
    }
```

> Note: `self.req.header(...)` returns `Option<String>` (see `Request::header`). Confirm the field name `req` on `Context` (it is `pub req: Request`).

- [ ] **Step 3: Flush cookies in `dispatch_parts`** — in `app.rs`, the `dispatch_parts` method builds a `Context` (`ctx`) then runs the chain producing a `Response`. The Context is moved into `chain.execute(ctx, ...)`, so capture a clone of the cookie slot BEFORE moving it. Simplest: after building `ctx` and before `chain.execute`, clone the Arc handle is not exposed — instead, change the flow to retrieve cookies via the returned context. Since `execute` consumes `ctx`, use this approach: wrap the response post-processing by having the handler-execution return both. **Concretely:** the `Context` is `Clone`? It is not. Instead, add a helper: keep a clone of the `set_cookies` Arc.

  Add, right after `let ctx = Context::from_parts(parts, body, params);` (and after any db attach), in BOTH the OPTIONS branch and the main branch:

```rust
        // Clone the cookie sink so we can flush it after the chain consumes ctx.
        let cookie_sink = ctx.set_cookies_handle();
```

  And add this accessor to `impl Context` (context.rs):

```rust
    pub(crate) fn set_cookies_handle(&self) -> Arc<RwLock<Vec<String>>> {
        self.set_cookies.clone()
    }
```

  Then, after obtaining the final `response` from the chain (replace the `match result { Ok(response) => response, ... }` tail so it binds to a `let mut response = ...;`), append cookies:

```rust
        let mut response = match result {
            Ok(response) => response,
            Err(err) => {
                error!("Handler error: {}", err);
                response::helpers::error_response(&err)
                    .unwrap_or_else(|_| response::helpers::text("Internal Error").unwrap())
            }
        };
        for value in cookie_sink.write().await.drain(..) {
            if let Ok(hv) = hyper::header::HeaderValue::from_str(&value) {
                response
                    .headers_mut()
                    .append(hyper::header::SET_COOKIE, hv);
            }
        }
        response
```

  Apply the same flush in the OPTIONS branch (it also runs middleware that may set cookies).

  Add the import if missing: `use std::sync::Arc;` and `use tokio::sync::RwLock;` are already used via context; in app.rs add `use std::sync::Arc;` if not present.

- [ ] **Step 4: Write the integration test** — create `ultimo/tests/cookie.rs`:

```rust
#![cfg(feature = "testing")]

use ultimo::cookie::{Cookie, SameSite};
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

#[tokio::test]
async fn set_cookie_appears_on_response() {
    let mut app = Ultimo::new_without_defaults();
    app.get("/set", |ctx: Context| async move {
        ctx.set_cookie(Cookie::new("sid", "xyz").http_only(true).same_site(SameSite::Lax))
            .await?;
        ctx.text("ok").await
    });

    let client = TestClient::new(app);
    let res = client.get("/set").send().await;
    let sc = res.header("set-cookie").unwrap();
    assert!(sc.contains("sid=xyz"));
    assert!(sc.contains("HttpOnly"));
    assert!(sc.contains("SameSite=Lax"));
}

#[tokio::test]
async fn reads_request_cookie() {
    let mut app = Ultimo::new_without_defaults();
    app.get("/read", |ctx: Context| async move {
        let v = ctx.cookie("token").unwrap_or_else(|| "none".into());
        ctx.text(v).await
    });

    let client = TestClient::new(app);
    let res = client.get("/read").header("cookie", "token=secret123").send().await;
    assert_eq!(res.text(), "secret123");
}
```

- [ ] **Step 5: Run** — `cargo test -p ultimo --features testing --test cookie` → 2 pass. Then `cargo test -p ultimo --lib` (no regressions) and `cargo test -p ultimo --features "websocket,test-helpers" --tests`.
- [ ] **Step 6: clippy + fmt** — `cargo clippy -p ultimo --all-targets --features "websocket,test-helpers,testing" -- -D warnings`; `cargo fmt --all`.
- [ ] **Step 7: Commit**

```bash
git add ultimo/src/context.rs ultimo/src/app.rs ultimo/tests/cookie.rs
git commit -m "feat(context): cookie read + Set-Cookie flush on response"
```

---

## Task 3: `session` feature + module skeleton + getrandom

**Files:** Modify `ultimo/Cargo.toml`, `ultimo/src/lib.rs`; create `ultimo/src/session/mod.rs`.

- [ ] **Step 1: Cargo.toml** — under `[features]` after `testing = []`:

```toml
# Session management (cookie-based, pluggable store)
session = ["dep:getrandom"]
```

And under `[dependencies]` (near base64):

```toml
getrandom = { version = "0.3", optional = true }
```

- [ ] **Step 2: lib.rs** — add:

```rust
#[cfg(feature = "session")]
pub mod session;
```

- [ ] **Step 3: Create `ultimo/src/session/mod.rs`** (minimal skeleton that compiles):

```rust
//! Cookie-based session management.
//!
//! Enable with the `session` feature. Register the middleware with a store:
//! `app.use_middleware(session(MemoryStore::new(), SessionConfig::default()))`.

mod config;
mod middleware;
mod store;

pub use config::SessionConfig;
pub use middleware::session;
pub use store::{MemoryStore, SessionStore};

use std::collections::HashMap;

/// Session payload: arbitrary JSON values keyed by string.
pub type SessionData = HashMap<String, serde_json::Value>;
```

> The `Session` type is added in Task 5; `config`/`store`/`middleware` submodules are filled in Tasks 4-7. To keep this task compiling on its own, create empty stub files now: `ultimo/src/session/{config,store,middleware}.rs` each `//! stub`, and temporarily comment the three `pub use` lines until their tasks add the types. (Each later task uncomments its own re-export.)

- [ ] **Step 4: Verify** — `cargo build -p ultimo --features session` compiles. `cargo clippy -p ultimo --features session -- -D warnings` clean.
- [ ] **Step 5: Commit**

```bash
git add ultimo/Cargo.toml ultimo/src/lib.rs ultimo/src/session/
git commit -m "feat(session): feature flag + module skeleton + getrandom dep"
```

---

## Task 4: `SessionStore` trait + `MemoryStore`

**Files:** Replace `ultimo/src/session/store.rs`; uncomment store re-export in `mod.rs`.

- [ ] **Step 1: Write `store.rs`**

```rust
//! Session store trait and in-memory implementation.

use super::SessionData;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Backing store for session data. Implement this for Redis/SQL/etc.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Load session data by id, or `None` if absent/expired.
    async fn load(&self, id: &str) -> Option<SessionData>;
    /// Persist session data under id with a time-to-live.
    async fn store(&self, id: &str, data: &SessionData, ttl: Duration);
    /// Remove a session.
    async fn destroy(&self, id: &str);
}

/// In-memory store. Suitable for single-process apps and tests.
#[derive(Clone, Default)]
pub struct MemoryStore {
    inner: Arc<RwLock<HashMap<String, (SessionData, Instant)>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SessionStore for MemoryStore {
    async fn load(&self, id: &str) -> Option<SessionData> {
        let map = self.inner.read().await;
        match map.get(id) {
            Some((data, expiry)) if *expiry > Instant::now() => Some(data.clone()),
            _ => None,
        }
    }

    async fn store(&self, id: &str, data: &SessionData, ttl: Duration) {
        let mut map = self.inner.write().await;
        // Opportunistic eviction of expired entries.
        let now = Instant::now();
        map.retain(|_, (_, exp)| *exp > now);
        map.insert(id.to_string(), (data.clone(), now + ttl));
    }

    async fn destroy(&self, id: &str) {
        self.inner.write().await.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_load_destroy_roundtrip() {
        let s = MemoryStore::new();
        let mut data = SessionData::new();
        data.insert("k".into(), serde_json::json!("v"));
        s.store("id1", &data, Duration::from_secs(60)).await;
        assert_eq!(s.load("id1").await.unwrap().get("k").unwrap(), &serde_json::json!("v"));
        s.destroy("id1").await;
        assert!(s.load("id1").await.is_none());
    }

    #[tokio::test]
    async fn expired_entries_are_not_loaded() {
        let s = MemoryStore::new();
        s.store("id2", &SessionData::new(), Duration::from_millis(1)).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(s.load("id2").await.is_none());
    }
}
```

- [ ] **Step 2: Uncomment** `pub use store::{MemoryStore, SessionStore};` in `mod.rs`.
- [ ] **Step 3: Run** — `cargo test -p ultimo --features session --lib session::store` → 2 pass.
- [ ] **Step 4: clippy + fmt + commit**

```bash
git add ultimo/src/session/store.rs ultimo/src/session/mod.rs
git commit -m "feat(session): SessionStore trait + MemoryStore"
```

---

## Task 5: `Session` handle + Context integration

**Files:** Modify `ultimo/src/session/mod.rs` (add `Session`); modify `ultimo/src/context.rs` (add gated `session` slot + `session()`).

- [ ] **Step 1: Add `Session` to `mod.rs`** (append after `SessionData`):

```rust
use crate::error::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

/// A handle to the current session. Cheap to clone (shares inner state).
#[derive(Clone)]
pub struct Session {
    inner: std::sync::Arc<SessionInner>,
}

struct SessionInner {
    id: RwLock<String>,
    data: RwLock<SessionData>,
    dirty: AtomicBool,
    destroyed: AtomicBool,
    new_id: RwLock<Option<String>>, // set by regenerate()
}

impl Session {
    pub(crate) fn new(id: String, data: SessionData) -> Self {
        Self {
            inner: std::sync::Arc::new(SessionInner {
                id: RwLock::new(id),
                data: RwLock::new(data),
                dirty: AtomicBool::new(false),
                destroyed: AtomicBool::new(false),
                new_id: RwLock::new(None),
            }),
        }
    }

    /// Current session id.
    pub async fn id(&self) -> String {
        self.inner.id.read().await.clone()
    }

    /// Get a typed value by key.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let data = self.inner.data.read().await;
        match data.get(key) {
            Some(v) => Ok(Some(serde_json::from_value(v.clone())?)),
            None => Ok(None),
        }
    }

    /// Set a typed value (marks the session dirty).
    pub async fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let v = serde_json::to_value(value)?;
        self.inner.data.write().await.insert(key.to_string(), v);
        self.inner.dirty.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Remove a key (marks dirty).
    pub async fn remove(&self, key: &str) {
        self.inner.data.write().await.remove(key);
        self.inner.dirty.store(true, Ordering::SeqCst);
    }

    /// Clear all data (marks dirty).
    pub async fn clear(&self) {
        self.inner.data.write().await.clear();
        self.inner.dirty.store(true, Ordering::SeqCst);
    }

    /// Issue a new id on the next persist (session-fixation defense). The old
    /// store entry is destroyed by the middleware.
    pub async fn regenerate(&self, new_id: String) {
        *self.inner.new_id.write().await = Some(new_id);
        self.inner.dirty.store(true, Ordering::SeqCst);
    }

    /// Destroy the session (server-side + cookie).
    pub fn destroy(&self) {
        self.inner.destroyed.store(true, Ordering::SeqCst);
    }

    // --- internal accessors for the middleware ---
    pub(crate) fn is_dirty(&self) -> bool {
        self.inner.dirty.load(Ordering::SeqCst)
    }
    pub(crate) fn is_destroyed(&self) -> bool {
        self.inner.destroyed.load(Ordering::SeqCst)
    }
    pub(crate) async fn snapshot(&self) -> SessionData {
        self.inner.data.read().await.clone()
    }
    pub(crate) async fn take_new_id(&self) -> Option<String> {
        self.inner.new_id.write().await.take()
    }
    pub(crate) async fn is_empty(&self) -> bool {
        self.inner.data.read().await.is_empty()
    }
}
```

- [ ] **Step 2: Add the Context slot (gated)** — in `context.rs`, the `Context` struct, add:

```rust
    #[cfg(feature = "session")]
    session: Arc<RwLock<Option<crate::session::Session>>>,
```

Initialize in `Context::from_parts`: `#[cfg(feature = "session")] session: Arc::new(RwLock::new(None)),`.

- [ ] **Step 3: Add Context session methods (gated)** — in `impl Context`:

```rust
    /// The current session. Panics if the session middleware isn't installed.
    #[cfg(feature = "session")]
    pub async fn session(&self) -> crate::session::Session {
        self.session
            .read()
            .await
            .clone()
            .expect("session middleware not installed (add `session(store, config)`)")
    }

    #[cfg(feature = "session")]
    pub(crate) async fn set_session(&self, s: crate::session::Session) {
        *self.session.write().await = Some(s);
    }
```

- [ ] **Step 4: Add a unit test in `mod.rs`** verifying the handle:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn set_get_and_dirty() {
        let s = Session::new("id".into(), SessionData::new());
        assert!(!s.is_dirty());
        s.set("n", &7u32).await.unwrap();
        assert!(s.is_dirty());
        assert_eq!(s.get::<u32>("n").await.unwrap(), Some(7));
    }
}
```

- [ ] **Step 5: Run** — `cargo test -p ultimo --features session --lib session` → store + handle tests pass. `cargo build -p ultimo` (no session) still compiles.
- [ ] **Step 6: clippy + fmt + commit**

```bash
git add ultimo/src/session/mod.rs ultimo/src/context.rs
git commit -m "feat(session): Session handle + ctx.session() integration"
```

---

## Task 6: `SessionConfig`

**Files:** Replace `ultimo/src/session/config.rs`; uncomment config re-export in `mod.rs`.

- [ ] **Step 1: Write `config.rs`**

```rust
//! Session configuration.

use crate::cookie::SameSite;
use std::time::Duration;

/// Session middleware configuration. Secure-by-default.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub cookie_name: String,
    pub ttl: Duration,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: SameSite,
    pub path: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cookie_name: "ultimo_sid".to_string(),
            ttl: Duration::from_secs(60 * 60 * 24),
            http_only: true,
            secure: true,
            same_site: SameSite::Lax,
            path: "/".to_string(),
        }
    }
}

impl SessionConfig {
    pub fn cookie_name(mut self, n: impl Into<String>) -> Self {
        self.cookie_name = n.into();
        self
    }
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }
    pub fn secure(mut self, v: bool) -> Self {
        self.secure = v;
        self
    }
    pub fn same_site(mut self, v: SameSite) -> Self {
        self.same_site = v;
        self
    }

    /// Validate invariants. `SameSite=None` requires `Secure`.
    pub fn validated(self) -> Self {
        if self.same_site == SameSite::None && !self.secure {
            panic!("SessionConfig: SameSite=None requires secure=true");
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_defaults() {
        let c = SessionConfig::default();
        assert!(c.http_only && c.secure);
        assert_eq!(c.same_site, SameSite::Lax);
    }

    #[test]
    #[should_panic(expected = "SameSite=None requires secure")]
    fn samesite_none_requires_secure() {
        SessionConfig::default().same_site(SameSite::None).secure(false).validated();
    }
}
```

- [ ] **Step 2: Uncomment** `pub use config::SessionConfig;` in `mod.rs`.
- [ ] **Step 3: Run** — `cargo test -p ultimo --features session --lib session::config` → 2 pass.
- [ ] **Step 4: clippy + fmt + commit**

```bash
git add ultimo/src/session/config.rs ultimo/src/session/mod.rs
git commit -m "feat(session): SessionConfig with secure defaults + validation"
```

---

## Task 7: Session middleware (security-critical — see spec Security section)

**Files:** Replace `ultimo/src/session/middleware.rs`; uncomment middleware re-export in `mod.rs`; create `ultimo/tests/session.rs`.

- [ ] **Step 1: Write `middleware.rs`**

```rust
//! Session middleware.

use super::{Session, SessionConfig, SessionStore};
use crate::context::Context;
use crate::cookie::Cookie;
use crate::middleware::{BoxedMiddleware, Next};
use crate::response::Response;
use base64::Engine;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Generate a 256-bit URL-safe session id. Panics if the OS RNG fails (we must
/// never fall back to weak randomness for a security token).
fn generate_id() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS RNG failure generating session id");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Build session middleware over `store`. Construct like the built-ins.
pub fn session<S>(store: S, config: SessionConfig) -> BoxedMiddleware
where
    S: SessionStore + 'static,
{
    let store = Arc::new(store);
    let config = Arc::new(config.validated());

    Arc::new(move |ctx: Context, next: Next| {
        let store = store.clone();
        let config = config.clone();
        Box::pin(async move {
            // 1-2. Load existing session ONLY if the cookie id is known to the
            // store. Never adopt a client-supplied id (anti session-fixation).
            let cookie_id = ctx.cookie(&config.cookie_name);
            let (id, data) = match &cookie_id {
                Some(cid) => match store.load(cid).await {
                    Some(d) => (cid.clone(), d),
                    None => (generate_id(), super::SessionData::new()),
                },
                None => (generate_id(), super::SessionData::new()),
            };

            let session = Session::new(id.clone(), data);
            ctx.set_session(session.clone()).await;

            // 4. Run the handler.
            let response: Response = next(ctx_with_session(&ctx)).await?;

            // 5. Persist per the security rules.
            if session.is_destroyed() {
                store.destroy(&id).await;
                let _ = queue_cookie(&ctx, expiry_cookie(&config)).await;
            } else if session.is_dirty() && !session.is_empty().await {
                let final_id = match session.take_new_id().await {
                    Some(new_id) => {
                        store.destroy(&id).await; // fixation: drop old entry
                        new_id
                    }
                    None => id.clone(),
                };
                store.store(&final_id, &session.snapshot().await, config.ttl).await;
                let _ = queue_cookie(&ctx, id_cookie(&config, &final_id)).await;
            }
            // empty/untouched session: persist nothing, no cookie (anti-DoS).

            Ok(response)
        }) as Pin<Box<dyn Future<Output = crate::error::Result<Response>> + Send>>
    })
}

// `ctx` is shared via interior mutability; the same Context flows through.
fn ctx_with_session(ctx: &Context) -> Context {
    ctx.clone_handle()
}

async fn queue_cookie(ctx: &Context, cookie: Cookie) -> crate::error::Result<()> {
    ctx.set_cookie(cookie).await
}

fn id_cookie(config: &SessionConfig, id: &str) -> Cookie {
    Cookie::new(config.cookie_name.clone(), id.to_string())
        .http_only(config.http_only)
        .secure(config.secure)
        .same_site(config.same_site)
        .path(config.path.clone())
        .max_age(config.ttl.as_secs() as i64)
}

fn expiry_cookie(config: &SessionConfig) -> Cookie {
    Cookie::new(config.cookie_name.clone(), "")
        .http_only(config.http_only)
        .secure(config.secure)
        .same_site(config.same_site)
        .path(config.path.clone())
        .max_age(0)
}
```

> **Context sharing note:** the middleware needs to set the session on the same
> Context the handler sees, then still reference `ctx` after `next` consumes it.
> `Next` takes `Context` by value. Add a `pub(crate) fn clone_handle(&self) ->
> Context` to `Context` that clones all the `Arc` slots (so both copies share the
> same interior-mutable state — `state`, `response_*`, `set_cookies`, `session`,
> and `req` must be shareable). If `Request` is not `Clone`, wrap the shared
> bits; simplest is to make `Context` hold its fields as `Arc`s already (most are)
> and derive a manual `clone_handle`. **Implementer:** verify `Context`'s fields
> are all `Arc`-shared; `req: Request` holds `Arc<RwLock<..>>` for the body but
> `method/uri/headers/params` are owned — make `clone_handle` clone those owned
> values and share the `Arc`s. Add a focused unit test that a value set on one
> handle is visible on the clone.

- [ ] **Step 2: Uncomment** `pub use middleware::session;` in `mod.rs`.

- [ ] **Step 3: Write security integration tests** — create `ultimo/tests/session.rs`:

```rust
#![cfg(all(feature = "session", feature = "testing"))]

use ultimo::session::{session, MemoryStore, SessionConfig};
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

fn app() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(session(MemoryStore::new(), SessionConfig::default().secure(false)));
    app.get("/login", |ctx: Context| async move {
        ctx.session().await.set("uid", &42u64).await?;
        ctx.text("ok").await
    });
    app.get("/me", |ctx: Context| async move {
        let uid: Option<u64> = ctx.session().await.get("uid").await?;
        ctx.json(serde_json::json!({ "uid": uid })).await
    });
    app.get("/anon", |ctx: Context| async move { ctx.text("hi").await });
    app
}

fn sid(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_string() // "ultimo_sid=VALUE"
}

#[tokio::test]
async fn session_persists_across_requests() {
    let client = TestClient::new(app());
    let login = client.get("/login").send().await;
    let cookie = sid(login.header("set-cookie").unwrap());

    let me = client.get("/me").header("cookie", &cookie).send().await;
    assert_eq!(me.json::<serde_json::Value>(), serde_json::json!({ "uid": 42 }));
}

#[tokio::test]
async fn empty_session_sets_no_cookie() {
    // anti-DoS: a request that never touches the session gets no Set-Cookie.
    let client = TestClient::new(app());
    let res = client.get("/anon").send().await;
    assert!(res.header("set-cookie").is_none());
}

#[tokio::test]
async fn unknown_client_id_is_not_adopted() {
    // anti-fixation: a forged cookie id must not become the session id.
    let client = TestClient::new(app());
    let res = client
        .get("/login")
        .header("cookie", "ultimo_sid=attacker_fixed_id")
        .send()
        .await;
    let issued = sid(res.header("set-cookie").unwrap());
    assert_ne!(issued, "ultimo_sid=attacker_fixed_id");
}

#[tokio::test]
async fn default_cookie_is_httponly_lax() {
    let mut app = Ultimo::new_without_defaults();
    // secure(true) default would also add Secure; here assert HttpOnly + SameSite.
    app.use_middleware(session(MemoryStore::new(), SessionConfig::default().secure(false)));
    app.get("/login", |ctx: Context| async move {
        ctx.session().await.set("x", &1u8).await?;
        ctx.text("ok").await
    });
    let res = TestClient::new(app).get("/login").send().await;
    let sc = res.header("set-cookie").unwrap();
    assert!(sc.contains("HttpOnly"));
    assert!(sc.contains("SameSite=Lax"));
}
```

- [ ] **Step 4: Run** — `cargo test -p ultimo --features "session,testing" --test session` → 4 pass. Fix the `clone_handle`/Context-sharing detail until the persistence test passes (this is the crux).
- [ ] **Step 5: clippy + fmt** — `cargo clippy -p ultimo --all-targets --features "session,testing,websocket,test-helpers" -- -D warnings`; `cargo fmt --all`.
- [ ] **Step 6: Commit**

```bash
git add ultimo/src/session/middleware.rs ultimo/src/session/mod.rs ultimo/src/context.rs ultimo/tests/session.rs
git commit -m "feat(session): secure session middleware (fixation/DoS/cookie hardening)"
```

---

## Task 8: Docs + CHANGELOG + CI wiring

**Files:** create `docs/sessions.md`; modify `CHANGELOG.md`, `.github/workflows/ci.yml`.

- [ ] **Step 1: Module doctest** — ensure `ultimo/src/session/mod.rs` has a runnable doc example (the login/me snippet from the spec, gated/`no_run` as needed). Run `cargo test -p ultimo --features "session" --doc`.

- [ ] **Step 2: Guide** — create `docs/sessions.md` covering: enabling the feature, registering `session(store, config)`, `ctx.session().get/set`, `regenerate()` on login (security note), `destroy()` on logout, cookie/security defaults, and that Redis/DB stores + CSRF are forthcoming. Use the real API from Tasks 4-7.

- [ ] **Step 3: CHANGELOG** — under `## [Unreleased]` → `### Added`:

```markdown
- Cookie helper (`ultimo::cookie`) + `ctx.cookie`/`set_cookie`/`remove_cookie`.
- Session management (`session` feature): `SessionStore` + `MemoryStore`,
  `Session` via `ctx.session()`, secure session middleware (HttpOnly/Secure/
  SameSite, anti-fixation, anti-DoS), `SessionConfig`. (#3)
```

- [ ] **Step 4: CI** — in `.github/workflows/ci.yml`:
  - `test` job, after the testing-utilities step, add:
    ```yaml
          - name: Session + cookie tests
            run: |
              cargo test -p ultimo --features "testing" --test cookie
              cargo test -p ultimo --features "session,testing" --test session
              cargo test -p ultimo --features "session" --lib
    ```
  - `lint` job: extend the feature clippy to include `session`:
    `cargo clippy -p ultimo --all-targets --features "websocket,test-helpers,testing,session" -- -D warnings`

- [ ] **Step 5: Validate + full verify**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml')); print('ok')"
cargo fmt --all --check
cargo clippy -p ultimo --all-targets --features "websocket,test-helpers,testing,session" -- -D warnings
cargo test -p ultimo --lib
cargo test -p ultimo --features "testing" --test cookie
cargo test -p ultimo --features "session,testing" --test session
cargo test -p ultimo --features "session" --doc
```

- [ ] **Step 6: Commit**

```bash
git add docs/sessions.md CHANGELOG.md .github/workflows/ci.yml ultimo/src/session/mod.rs
git commit -m "docs+ci: session guide, changelog, CI wiring (#3)"
```

---

## Notes for the implementer
- **The crux is Task 7's Context sharing** (`clone_handle`): the session middleware sets the session, passes the Context to `next`, then reads the same session back after. All Context state is `Arc`-shared interior-mutable, so cloning the handle (sharing the `Arc`s) makes this work. Verify with the persistence test before moving on.
- `getrandom` 0.3 uses `getrandom::fill(&mut buf)`. If the resolved version is 0.2, the call is `getrandom::getrandom(&mut buf)` — check `Cargo.lock`.
- `base64` 0.21 is already a dep: `base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(..)` with `use base64::Engine;`.
- `async-trait` is already a dep — used for `SessionStore` (keeps store futures `Send` + dyn-safe).
- Keep every new public item documented (docs.rs).
