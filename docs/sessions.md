# Sessions

Ultimo ships cookie-based session management behind the `session` feature. It's
structured Hono-style: cookies are a core helper, and sessions are middleware
over a pluggable store.

## Enable

```toml
[dependencies]
ultimo = { version = "0.3", features = ["session"] }
```

## Register the middleware

```rust
use ultimo::session::{session, MemoryStore, SessionConfig};

let mut app = Ultimo::new_without_defaults();
app.use_middleware(session(MemoryStore::new(), SessionConfig::default()));
```

## Read & write

Access the session from any handler via `ctx.session()`:

```rust
// write
app.post("/login", |ctx: Context| async move {
    ctx.session().await.set("user_id", &42u64).await?;
    ctx.text("logged in").await
});

// read
app.get("/me", |ctx: Context| async move {
    let id: Option<u64> = ctx.session().await.get("user_id").await?;
    ctx.json(serde_json::json!({ "user_id": id })).await
});
```

Values are typed (serde): `set(key, &value)` / `get::<T>(key)`. Other methods:
`remove(key)`, `clear()`, `id()`.

## Security model

Defaults are secure:

- **256-bit random ids** (`getrandom`); the session id is opaque and unguessable.
- **`HttpOnly` + `Secure` + `SameSite=Lax`** cookie by default.
- **Server-side data** — only the id is in the cookie, so there's no tampering or
  confidentiality risk on the client.
- **Anti session-fixation** — a client-supplied id is never adopted; unknown ids
  get a fresh server id. Call **`ctx.session().await.regenerate()`** on
  login/privilege change — the middleware issues a new id and drops the old one.
- **Anti-DoS** — empty/untouched sessions are not persisted and get no cookie.
- **Expiration** — enforced server-side (store TTL) *and* via the cookie `Max-Age`.

On logout, call `ctx.session().await.destroy()` — it removes the server-side
entry and expires the cookie.

> **CSRF:** `SameSite=Lax` is partial protection. Full CSRF tokens are a planned
> follow-up; don't rely on sessions alone for CSRF defense yet.

### Local development

The `Secure` cookie won't be sent over plain HTTP. For local dev:

```rust
SessionConfig::default().secure(false)  // dev only
```

## Configuration

```rust
use std::time::Duration;
use ultimo::cookie::SameSite;

SessionConfig::default()
    .cookie_name("my_sid")
    .ttl(Duration::from_secs(3600))
    .same_site(SameSite::Strict)
    .secure(true)
```

`SameSite::None` requires `secure(true)` (enforced at construction).

## Stores

`MemoryStore` (single-process) is built in. Implement the `SessionStore` trait
for other backends:

```rust
#[async_trait::async_trait]
impl SessionStore for MyStore {
    async fn load(&self, id: &str) -> Option<SessionData> { /* ... */ }
    async fn store(&self, id: &str, data: &SessionData, ttl: Duration) { /* ... */ }
    async fn destroy(&self, id: &str) { /* ... */ }
}
```

Redis and SQL stores are planned follow-ups.
