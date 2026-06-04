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
- Formatting: `Cookie::to_set_cookie_string() -> Result<String>` (RFC 6265
  attributes). **Header-injection defense:** reject cookie names/values
  containing control characters (CR, LF, NUL) or other characters illegal per
  RFC 6265 — never emit unvalidated, attacker-influenced bytes into a response
  header. (Generated session ids are already safe; this guards `ctx.set_cookie`
  with app-supplied values.)
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
- `regenerate(&self)` — issue a **new** secure id and **destroy the old store
  entry** (session-fixation defense), keeping the data
- `destroy(&self)` — drop the session server-side + expire the cookie

### 4. Session middleware (`session/middleware.rs`)
`session(store, config) -> BoxedMiddleware`. Flow per request:
1. Read the session-id cookie (`config.cookie_name`).
2. If a cookie id is present **and found in the store**, load it. **If the id is
   absent OR not found in the store, generate a fresh server-side id — never
   adopt a client-supplied id** (anti session-fixation). Do not yet persist.
3. Attach the `Session` to the Context (typed slot; `ctx.session()` returns it).
4. `next(ctx).await` → response.
5. Persistence (security-critical rules):
   - **Only persist when the session is dirty AND non-empty.** An untouched /
     empty session is **not** stored and **gets no cookie** — this prevents an
     unbounded-session DoS (one stored entry + Set-Cookie per anonymous hit) and
     avoids needless cookies.
   - On dirty+non-empty: `store.store(id, data, ttl)` and queue the `Set-Cookie`
     (HttpOnly/Secure/SameSite/Max-Age per config).
   - On `destroy()`: `store.destroy(id)` + queue an expiry cookie (`Max-Age=0`).
   - On `regenerate()`: `store.destroy(old_id)`, `store.store(new_id, …)`, cookie
     carries the new id.
6. Flush queued cookies onto the response.

### 5. `SessionConfig` (`session/config.rs`)
`{ cookie_name: String = "ultimo_sid", ttl: Duration = 24h, http_only: bool = true, secure: bool = true, same_site: SameSite = Lax, path: String = "/" }`, with a builder. **Secure-by-default.**
Validation: if `same_site == None`, `secure` **must** be `true` (browsers reject
`SameSite=None` without `Secure`) — enforce at construction. The `secure` default
is `true`; document that local HTTP dev needs `.secure(false)` explicitly (a
deliberate opt-out, not a silent default).

### 6. Secure IDs
`getrandom` → 32 random bytes (256 bits) → URL-safe base64 (no padding). IDs are
unguessable. **`getrandom` failure is a hard error** — never fall back to a
weaker RNG. The high entropy also makes store-lookup timing irrelevant.

## Context integration

`Context` gains two additive slots (interior-mutable, like existing state):
- `set_cookies: Arc<RwLock<Vec<String>>>` — queued Set-Cookie header values.
- `session: Arc<RwLock<Option<Session>>>` — attached by the middleware;
  `ctx.session()` reads it (panics with a clear message if the session
  middleware isn't installed, or returns a Result — decided in the plan).

`ctx.session()` returning a guard vs a cloneable handle is settled in the plan;
the data lives in the store, the handle mediates access.

## Security (threat model + mitigations)

Sessions are security-critical; these are designed in, not bolted on.

| Threat | Mitigation (in this design) |
|---|---|
| **Session prediction/brute force** | 256-bit CSPRNG ids (`getrandom`); hard error on RNG failure (no weak fallback). |
| **Session fixation** | Never adopt a client-supplied id — unknown/absent id → fresh server id. `regenerate()` (apps must call it on login/privilege change) issues a new id and **destroys the old store entry**. |
| **XSS cookie theft** | `HttpOnly` default `true` — JS can't read the session cookie. |
| **Cookie interception (MITM)** | `Secure` default `true` — cookie only sent over HTTPS. |
| **CSRF** | `SameSite=Lax` default gives partial protection. **Full CSRF protection is a deferred follow-up (known gap)** — documented clearly so users don't assume coverage. `SameSite=None` requires `Secure` (enforced). |
| **Header injection via cookies** | Cookie name/value validated; control chars (CR/LF/NUL) rejected before emitting `Set-Cookie`. |
| **Session-store DoS (unbounded growth)** | Empty/untouched sessions are never persisted and get no cookie; only dirty, non-empty sessions are stored, with TTL eviction. |
| **Stale session reuse** | Server-side TTL eviction in the store **and** cookie `Max-Age` — expiry enforced server-side, not just client-side. |
| **Data tampering/confidentiality** | Session data lives **server-side** (store); only the opaque id is in the cookie — no signed/encrypted-cookie attack surface. |
| **Logout not ending the session** | `destroy()` removes the entry server-side (not just the cookie) + expires the cookie. |

**Known limitations (documented, acceptable for this slice):**
- Concurrent requests sharing a session are last-write-wins (read-modify-write
  race) — acceptable for v1; revisit if atomic updates are needed.
- No CSRF tokens yet (see deferred follow-ups); `SameSite=Lax` is the interim
  defense.

**Future hardening (deferred):** `__Host-`/`__Secure-` cookie name prefixes,
signed/encrypted cookies, sliding-expiration option, idle vs absolute timeout.

## Testing strategy
- Cookie unit tests: parse round-trips, Set-Cookie formatting (all attributes,
  SameSite, Max-Age=0 deletion).
- Session unit tests: MemoryStore load/store/destroy + expiry eviction.
- Integration (via the `testing` feature's `TestClient`): login sets a cookie →
  subsequent request with that cookie sees the session value; destroy clears it;
  regenerate changes the id; expired session is not loaded.
- **Security tests (required):**
  - A client-supplied unknown id is **not** adopted — the server issues a
    different id (anti-fixation).
  - `regenerate()` changes the id AND the old id no longer resolves in the store.
  - An empty/untouched session is **not** persisted and **no** `Set-Cookie` is
    emitted (anti-DoS).
  - Default cookie carries `HttpOnly`, `Secure`, and `SameSite=Lax`.
  - A cookie value with `\r`/`\n` is rejected (header-injection guard).
  - `SameSite=None` without `Secure` is rejected at config construction.
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
