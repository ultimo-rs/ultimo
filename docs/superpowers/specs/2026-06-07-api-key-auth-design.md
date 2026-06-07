# API-Key Auth Middleware — Design

**Date:** 2026-06-07
**Milestone:** v0.4.0 (Security & Performance — Wave 2: auth & abuse)
**Epic:** #53 (Security: secure-by-default)
**Status:** Approved, pre-implementation

## Goal

A second authentication method alongside JWT: an API-key middleware that
validates a presented key, resolves it to an **identity** (id + scopes), attaches
that identity to the request `Context`, and rejects unauthenticated requests with
`401`. Designed quality-first — extensible (pluggable store) and leak-resistant
(hashed, constant-time comparison) — so it is production-grade, not a toy, and so
it sets up the authorization-guards feature that follows.

## Design principles (why this shape)

1. **Pluggable store, not a static-only set.** Real deployments keep keys in a
   database with per-key metadata (owner, scopes, expiry, revocation). Matching
   the framework's existing `SessionStore` precedent, API-key validation goes
   through an `ApiKeyStore` trait, with a built-in `StaticKeys` store for quick
   start. This avoids a future breaking change to add DB-backed keys.
2. **Hashed, constant-time comparison.** Best-in-class systems store a *hash* of
   the key, never the raw secret, so a memory/config/DB disclosure does not leak
   usable credentials. API keys are **high-entropy random secrets, not
   passwords**, so the correct hash is **SHA-256** — *not* bcrypt/argon2 (those
   are for low-entropy human passwords and would only waste CPU here). Comparison
   is constant-time over the fixed-length digest.
3. **Resolve to an identity, not the raw key.** Handlers (and the upcoming
   authorization guards) receive an `ApiKeyIdentity { id, scopes }` — never the
   secret. This is the uniform thing guards will check (alongside JWT claims).

## Non-goals (deferred — YAGNI)

- Key generation/minting helpers (prefixes, checksums, secret-scanning formats).
- Built-in DB/Redis store impls — users implement `ApiKeyStore` for their backend.
- Rate-limiting / per-key quotas (separate abuse-primitives feature).
- A browser example: putting an API key in browser JS is an anti-pattern, so the
  docs demonstrate **`curl` / server-to-server** usage instead. (The
  frontend-example rule targets *frontend-relevant* features; this isn't one.)

## Public API

New module `ultimo::auth::api_key`, gated by `#[cfg(feature = "api-key")]`
(`api-key = ["dep:sha2"]`; `async-trait` is already a dependency).

```rust
use ultimo::auth::api_key::{ApiKey, StaticKeys};

let store = StaticKeys::new()
    .insert("key-abc", "service-a")
    .with_scopes("key-def", "service-b", ["read", "write"]);

let api = ApiKey::new(store)        // or ApiKey::new(MyDbStore) — any ApiKeyStore
    .header_name("x-api-key")       // default; or .from_query("api_key")
    .optional();                    // optional mode (pass through unauthenticated)

app.use_middleware(api.build());

// In a handler:
let who = ctx.api_key();            // Option<ApiKeyIdentity> { id, scopes }
```

### Types

```rust
#[async_trait]
pub trait ApiKeyStore: Send + Sync {
    /// Resolve a presented key to an identity, or None to reject.
    async fn validate(&self, presented_key: &str) -> Option<ApiKeyIdentity>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApiKeyIdentity {
    pub id: String,          // a label / key id — NEVER the raw secret
    pub scopes: Vec<String>, // optional scopes for authorization
}

pub struct StaticKeys { /* Vec<([u8; 32] sha256, ApiKeyIdentity)> */ }
impl StaticKeys {
    pub fn new() -> Self;                                   // + Default
    pub fn insert(self, key, id) -> Self;                  // builder
    pub fn with_scopes(self, key, id, scopes) -> Self;     // builder
}

pub struct ApiKey<S: ApiKeyStore> { /* store, source, optional */ }
impl<S: ApiKeyStore + 'static> ApiKey<S> {
    pub fn new(store: S) -> Self;                // header "x-api-key", required
    pub fn header_name(self, name) -> Self;
    pub fn from_query(self, name) -> Self;
    pub fn optional(self) -> Self;
    pub fn build(self) -> BoxedMiddleware;
}
```

### Middleware behavior

1. Read the key from the configured source (header `x-api-key` by default, or a
   query parameter).
2. `store.validate(key).await`. On `Some(identity)` → attach to `Context`, call
   `next`. On `None` → `401` (or pass through in `optional` mode).
3. Missing key → same as invalid (`401`, or pass through if optional).

No `WWW-Authenticate` header (ApiKey is not a registered HTTP auth scheme).

### `StaticKeys` comparison (constant-time)

Keys are hashed with SHA-256 at construction; only digests are stored. On
`validate`, the presented key is hashed and compared against every stored digest
with a constant-time equality check, **without early return**, so neither the
match position nor whether any key matched leaks via timing.

### Context integration

Feature-gated slot mirroring `session` / `jwt_claims`:
- `#[cfg(feature = "api-key")] pub async fn api_key(&self) -> Option<ApiKeyIdentity>`
- `pub(crate) async fn set_api_key(&self, identity: ApiKeyIdentity)`

## Module gating

`ultimo::auth` becomes available when **either** auth feature is on:
`#[cfg(any(feature = "jwt", feature = "api-key"))] pub mod auth;`, and
`auth/mod.rs` gates each submodule (`#[cfg(feature = "jwt")] pub mod jwt;`,
`#[cfg(feature = "api-key")] pub mod api_key;`).

## Testing

Unit (in-module): `StaticKeys` resolves a valid key to its identity (with scopes),
rejects an unknown key, and an unknown key never matches across a multi-key set.
Integration (`ultimo/tests/api_key.rs`, via `Ultimo::oneshot`): valid header →
200 + identity attached; invalid → 401; missing → 401; `optional()` passes
through; query-param source works; scopes surface on `ctx.api_key()`.

## CI coverage (fix existing gap)

`ci.yml` currently does not *run* the JWT tests (only `--all-features` clippy
compiles them). Add explicit runs in the `test` job and the non-DB clippy:
- lint: add `jwt,api-key` to the `clippy --all-targets --features "..."` list.
- test: `cargo test -p ultimo --features "jwt" --lib auth::jwt`,
  `--features "jwt" --test jwt`, `--features "jwt" --doc`,
  `--features "api-key" --lib auth::api_key`,
  `--features "api-key" --test api_key`.

## Docs & SemVer

- New `docs-site/docs/pages/api-keys.mdx` under the **Authentication** sidebar
  group; update `api-reference.mdx` (new `ApiKey`/`ApiKeyStore`/`ApiKeyIdentity`,
  `ctx.api_key()`, `api-key` feature), `README.md`, `roadmap.mdx`.
- Additive (new opt-in feature + new `pub` items) → release-plz bumps from the
  `feat:` commit; `semver-checks` confirms no breakage.
