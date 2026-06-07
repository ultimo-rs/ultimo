# JWT Auth Middleware — Design

**Date:** 2026-06-07
**Milestone:** v0.4.0 (Security & Performance — Wave 2: auth & abuse)
**Epic:** #53 (Security: secure-by-default)
**Status:** Approved, pre-implementation

## Goal

Ship Ultimo's flagship authentication primitive: a JWT middleware that verifies
signed bearer tokens, attaches the validated claims to the request `Context`, and
rejects unauthenticated requests with `401`. Include a token-signing helper so apps
can issue tokens (and so the example/login flow is self-contained). This is the
first Wave 2 deliverable and sets the pattern for API-key auth and authorization
guards that follow.

## Non-goals (explicitly deferred — YAGNI)

- **RS256 / EdDSA / asymmetric keys.** v1 is **HS256 only**. The config type is
  shaped so asymmetric algorithms can be added later without a breaking change.
- **API-key auth middleware** — next Wave 2 feature, separate PR.
- **Authorization guards (roles/permissions)** — next Wave 2 feature, separate PR.
- **Refresh tokens, token revocation/blocklist, JWKS endpoints** — out of scope.

## Key decision: depend on `jsonwebtoken`, do not hand-roll

We depend on the audited [`jsonwebtoken`](https://crates.io/crates/jsonwebtoken)
crate rather than implementing JWT verification ourselves.

**Why.** JWT has a well-known class of implementation footguns — `alg: none`
acceptance and HS/RS algorithm-confusion attacks — that a naive verifier gets
wrong. `jsonwebtoken` forces the caller to pin the expected algorithm(s) and
rejects `none`. For a framework whose headline pillar is security, relying on a
battle-tested implementation is the credible, defensible choice. The leaner
alternative (hand-rolled HS256 over `hmac`/`sha2`) would make *us* responsible for
those mitigations — exactly what a security-branded framework should not own.

**Cost & containment.** `jsonwebtoken` pulls `ring` (C/asm crypto). This is
acceptable: `#![forbid(unsafe_code)]` governs only *our* crate, not dependencies,
and the whole thing sits behind an **opt-in `jwt` Cargo feature**. With
`default = []`, users who don't enable `jwt` pull none of it.

## Public API

New module `ultimo::auth::jwt`, gated by `#[cfg(feature = "jwt")]`. A single config
type handles both signing and verification so the key is configured once.

```rust
use ultimo::auth::jwt::Jwt;

// Configure once (HS256 symmetric secret).
let jwt = Jwt::hs256(secret)        // secret: impl Into<Vec<u8>> / &[u8]
    .issuer("ultimo")               // optional: require `iss` to match
    .audience("web")                // optional: require `aud` to match
    .leeway(60)                     // optional: clock-skew tolerance (seconds)
    .from_bearer();                 // token source; default. Or .from_cookie("token")

// Verify on incoming requests + attach claims.
app.use_middleware(jwt.clone().build());

// Issue a token (e.g. in a /login handler).
let token: String = jwt.sign(&my_claims)?;
```

### Type sketch

- `pub struct Jwt` — holds the encoding/decoding key, the pinned algorithm
  (HS256 for v1), `Validation` settings (iss/aud/leeway/exp), token source
  (`Bearer` header vs named cookie), header name, and `required` vs `optional`.
- Builder methods (all `self -> Self`): `hs256(secret)`, `issuer(s)`,
  `audience(s)`, `leeway(secs)`, `from_bearer()`, `from_cookie(name)`,
  `header_name(name)`, `optional()`.
- `pub fn sign<T: Serialize>(&self, claims: &T) -> Result<String>` — encode a
  signed token. The caller owns the claims struct (must include `exp`, etc.).
- `pub fn build(self) -> BoxedMiddleware` — the verification middleware.

`Jwt` is `Clone` (so it can be both used as middleware and kept around for `sign`).

### Middleware behavior

1. Extract the token from the configured source (default: `Authorization: Bearer
   <token>`; alternatively a named cookie).
2. Verify signature + standard claims (`exp`, `nbf`, and `iss`/`aud` if configured)
   with the pinned algorithm. `alg: none` and algorithm confusion are rejected by
   `jsonwebtoken`.
3. **On success:** store the validated claims (as `serde_json::Value`) on the
   `Context`, then call `next`.
4. **On failure (missing/invalid/expired):**
   - `required` mode (default): return **401** with `WWW-Authenticate: Bearer`.
   - `optional` mode: attach nothing and call `next` (for mixed public/private
     routes; handlers decide what to do when claims are absent).

### Context integration

Claims are exposed on `Context` the same way sessions are — a feature-gated slot,
not a generic-over-claims middleware (the middleware lives in a
`Vec<BoxedMiddleware>`, so it can't be generic). New `#[cfg(feature = "jwt")]`
methods on `Context`:

- `pub fn jwt_claims(&self) -> Option<&serde_json::Value>` — raw validated claims.
- `pub fn jwt<T: DeserializeOwned>(&self) -> Result<T>` — deserialize claims into a
  typed struct; errors if absent or shape mismatch.

## Error handling

- Auth failures in `required` mode produce a `401 Unauthorized` response with a
  `WWW-Authenticate: Bearer` header and a short body. This is a normal response,
  not an `Err` (mirrors the CSRF middleware's `Ok(forbidden())` pattern).
- `sign()` and `jwt::<T>()` return `ultimo::Result` and surface `jsonwebtoken` /
  deserialization errors via `UltimoError`.

## Testing

Unit + integration (`Ultimo::oneshot`), gated on the `jwt` feature:

1. sign → verify roundtrip attaches expected claims.
2. expired token (`exp` in past, no leeway) → rejected (401).
3. tampered/bad-signature token → rejected.
4. `alg: none` token → rejected.
5. wrong `iss`/`aud` when configured → rejected.
6. missing token, `required` mode → 401 with `WWW-Authenticate`.
7. missing token, `optional` mode → passes through, `jwt_claims()` is `None`.
8. cookie source: token in configured cookie is read and verified.
9. typed `ctx.jwt::<T>()` deserializes; absent claims → error.

## Documentation & example surfaces (same PR)

Per the `ship-feature` doc-surface rules:

- **`examples/jwt-auth`** (new, added to workspace `members`): a Rust backend
  serving an HTML+JS page. `/login` signs a JWT and returns it; the page stores it
  and sends `Authorization: Bearer` on subsequent calls; `/me` is protected by the
  middleware. Runnable via `cargo run -p jwt-auth-example`. Models `examples/session-auth`.
- **`docs-site/docs/pages/authentication.mdx`** (new) + **sidebar** entry in
  `docs-site/vocs.config.ts` (under Security).
- **`docs-site/docs/pages/api-reference.mdx`**: new Auth/JWT section + the `jwt`
  feature flag.
- **`README.md`**: move JWT from Coming Soon → Available Now; note the `jwt` feature.
- **`docs-site/docs/pages/roadmap.mdx`**: mark JWT auth shipped.

## Cargo / SemVer

- Add `jwt = ["dep:jsonwebtoken"]` to `ultimo/Cargo.toml`; `jsonwebtoken` optional.
- Additive change (new opt-in feature + new `pub` items). release-plz computes the
  version bump from the `feat:` commit; `semver-checks` confirms no breakage.
