# Session Management — Design Spec

- **Issue:** [#3 Add session management system](https://github.com/ultimo-rs/ultimo/issues/3)
- **Milestone:** v0.3.0
- **Date:** 2026-06-03
- **Status:** Approved (design), pending implementation plan
- **Scope:** Core slice only (see "Out of scope" for deferred work)

## Goal

Built-in, secure, cookie-based session management for Ultimo apps, structured
Hono-style: **cookies as a core helper**, **sessions as middleware over a
pluggable store**. This slice ships a complete, working session system on an
in-memory store; Redis/DB stores and CSRF are deferred and slot in behind the
same `SessionStore` trait without rework.

Target usage:

```rust
use ultimo::session::{session, MemoryStore, SessionConfig};

let mut app = Ultimo::new_without_defaults();
app.use_middleware(session(MemoryStore::new(), SessionConfig::default()));

app.get("/login", |ctx: Context| async move {
    ctx.session().set("user_id", &42u64).await?;
    ctx.text("logged in").await
});

app.get("/me", |ctx: Context| async move {
    let id: Option<u64> = ctx.session().get("user_id").await?;
    ctx.json(serde_json::json!({ "user_id": id })).await
});
```

## Approach

- **Cookies live in core** (no feature flag), hand-rolled with **no new
  dependency** — consistent with the zero-dependency websocket module. Cookie
  parsing (`Cookie:` header) and emission (`Set-Cookie`) are small and
  well-specified (RFC 6265).
- **Sessions live behind a `session` Cargo feature** (`default = []`). The only
  new dependency is **`getrandom`** (tiny) for cryptographically secure session
  IDs. Session data (de)serializes via `serde_json` (already a dependency).
- Sessions are delivered as **middleware over a `SessionStore` trait**, mirroring
  Hono's "session as middleware" model. The store is the single extension point.

## Module structure

```
ultimo/src/
  cookie.rs            # CORE: Cookie, CookieOptions, parse + Set-Cookie formatting
  session/
    mod.rs             # Session handle + re-exports (gated: feature = "session")
    store.rs           # SessionStore trait + MemoryStore
    middleware.rs      # session() middleware constructor
    config.rs          # SessionConfig
```

`lib.rs`: `pub mod cookie;` (core) and `#[cfg(feature = "session")] pub mod session;`.

## Components

### 1. Cookies (core, `ultimo::cookie` + Context methods)
- `struct Cookie { name, value, options: CookieOptions }`.
- `struct CookieOptions { http_only: bool, secure: bool, same_site: Option<SameSite>, max_age: Option<i64> /*seconds*/, path: Option<String>, domain: Option<String>, expires: Option<…> }` with a builder.
- `enum SameSite { Strict, Lax, None }`.
- Parsing: `parse_cookie_header(&str) -> HashMap<String, String>`.
- Formatting: `Cookie::to_set_cookie_string() -> String` (RFC 6265 attributes).
- Context API (additive):
  - `ctx.cookie(name) -> Option<String>` — read a request cookie.
  - `ctx.cookies() -> HashMap<String, String>` — all request cookies.
  - `ctx.set_cookie(Cookie)` — queue a `Set-Cookie` (stored on Context, flushed onto the response).
  - `ctx.remove_cookie(name)` — queue a deletion cookie (`Max-Age=0`).
- **Response flushing:** Context accumulates queued Set-Cookie headers; they are
  written onto the outgoing `Response` when the handler/middleware result is
  finalized. (Mechanism detailed in the plan — likely a `Vec<String>` slot on
  Context applied in `dispatch_parts` / by middleware.)

### 2. `SessionStore` trait + `MemoryStore` (`session/store.rs`)
```rust
#[async_trait-like or RPIT]
pub trait SessionStore: Send + Sync {
    async fn load(&self, id: &str) -> Option<SessionData>;
    async fn store(&self, id: &str, data: &SessionData, ttl: Duration);
    async fn destroy(&self, id: &str);
}
```
- `SessionData = HashMap<String, serde_json::Value>`.
- `MemoryStore`: `Arc<RwLock<HashMap<String, (SessionData, Instant /*expiry*/)>>>`,
  lazily evicting expired entries on access.

### 3. `Session` handle (`session/mod.rs`)
Accessed via `ctx.session()`. Holds the current id + an in-memory copy of the
data + a dirty flag.
- `id(&self) -> &str`
- `get::<T: DeserializeOwned>(&self, key) -> Result<Option<T>>`
- `set::<T: Serialize>(&self, key, &T) -> Result<()>` (marks dirty)
- `remove(&self, key)` / `clear(&self)` (mark dirty)
- `regenerate(&self)` — issue a new id (session fixation defense), keep data
- `destroy(&self)` — drop the session + expire the cookie

### 4. Session middleware (`session/middleware.rs`)
`session(store, config) -> BoxedMiddleware`. Flow per request:
1. Read the session-id cookie (`config.cookie_name`).
2. If present, `store.load(id)`; else (or on miss) create a fresh session with a
   new secure id.
3. Attach the `Session` to the Context (typed slot; `ctx.session()` returns it).
4. `next(ctx).await` → response.
5. If the session is dirty/new: `store.store(id, data, ttl)` and queue the
   `Set-Cookie` (HttpOnly/Secure/SameSite/Max-Age per config). If destroyed:
   `store.destroy(id)` + queue an expiry cookie.
6. Flush queued cookies onto the response.

### 5. `SessionConfig` (`session/config.rs`)
`{ cookie_name: String = "ultimo_sid", ttl: Duration = 24h, http_only: bool = true, secure: bool = true, same_site: SameSite = Lax, path: String = "/" }`, with a builder. Secure defaults.

### 6. Secure IDs
`getrandom` → 32 random bytes → URL-safe encoding (hex or base64url). No
predictable IDs.

## Context integration

`Context` gains two additive slots (interior-mutable, like existing state):
- `set_cookies: Arc<RwLock<Vec<String>>>` — queued Set-Cookie header values.
- `session: Arc<RwLock<Option<Session>>>` — attached by the middleware;
  `ctx.session()` reads it (panics with a clear message if the session
  middleware isn't installed, or returns a Result — decided in the plan).

`ctx.session()` returning a guard vs a cloneable handle is settled in the plan;
the data lives in the store, the handle mediates access.

## Testing strategy
- Cookie unit tests: parse round-trips, Set-Cookie formatting (all attributes,
  SameSite, Max-Age=0 deletion).
- Session unit tests: MemoryStore load/store/destroy + expiry eviction.
- Integration (via the `testing` feature's `TestClient`): login sets a cookie →
  subsequent request with that cookie sees the session value; destroy clears it;
  regenerate changes the id; expired session is not loaded.
- All under CI with the `session` feature added to the relevant jobs.

## Published-crate impact
Additive only: new `cookie` core module + `ctx.cookie/cookies/set_cookie/remove_cookie`,
new `session` feature + `ultimo::session` items + `ctx.session()`. No existing
signature changes; `cargo-semver-checks` confirms. Ships in **v0.3.0**; add a
`CHANGELOG.md` entry.

## Out of scope (deferred follow-up issues)
- Redis session store (implements `SessionStore`).
- SQLx / Diesel session store (implements `SessionStore`).
- CSRF protection (separate middleware, builds on sessions/cookies).
- MessagePack serialization option (JSON only for now).
- Signed/encrypted cookies (Hono's signed-cookie equivalent) — future.
