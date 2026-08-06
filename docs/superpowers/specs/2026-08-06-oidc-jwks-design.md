# OIDC / JWKS Auth Providers — Design

**Date:** 2026-08-06
**Status:** Approved (brainstorm), pending spec review
**Roadmap:** 0.7 — "Auth Providers (OIDC/JWKS + presets)"
**Builds on:** the `jwt` feature (currently HS256-only)

## Goal

Verify **RS256/ES256** JWTs issued by an OIDC provider against its **remote JWKS
endpoint** (fetch + cache + key rotation, validating `iss`/`aud`/`exp`), so
Ultimo apps can accept tokens from Clerk, Auth0, Cognito, Supabase, and any other
standard OIDC provider — not just the symmetric HS256 tokens the framework signs
itself.

## Verification flow

1. Read the token's JOSE header (`jsonwebtoken::decode_header`) → `kid`.
2. Look up the key in the cached `JwkSet` (`JwkSet::find(kid)`).
3. Build a `DecodingKey` from the JWK (`DecodingKey::from_jwk` — no manual
   RSA/EC component handling).
4. `jsonwebtoken::decode::<Claims>` with a `Validation` accepting the asymmetric
   algorithms and enforcing `iss`/`aud`/`exp`.
5. If the `kid` is absent from the cache, refetch the JWKS once and retry
   (handles provider key rotation), then fail if still missing.

## Architecture

### Extend `Jwt` with a key source (Approach A)

`Jwt` currently holds a single `DecodingKey` + `Validation` (+ an `EncodingKey`
for `sign`). Introduce a key-source enum so the same builder serves both the
symmetric and JWKS cases:

```rust
enum KeySource {
    /// HS256 / fixed key — synchronous verify; supports `sign`.
    Static { encoding: Option<EncodingKey>, decoding: DecodingKey },
    /// Remote JWKS — asynchronous verify; verify-only.
    #[cfg(feature = "oidc")]
    Jwks(JwksClient),
}
```

- All existing builder options carry over unchanged: `issuer`, `audience`,
  `leeway`, `from_bearer`, `from_cookie`, `optional`.
- `sign()` stays available only for the `Static` (HS256) path; JWKS is
  verify-only (asymmetric public keys).
- The existing `hs256(secret)` constructor produces `KeySource::Static`.

New constructors (behind `oidc`):

```rust
// Verify against a known JWKS endpoint.
Jwt::jwks(jwks_url: impl Into<String>) -> Self;
// Verify against a provider discovered from its issuer
// ({issuer}/.well-known/openid-configuration -> jwks_uri).
Jwt::oidc(issuer: impl Into<String>) -> Self;
// Offline / air-gapped / tests: verify against an in-memory key set.
Jwt::jwks_from_set(keys: jsonwebtoken::jwk::JwkSet) -> Self;
```

### `JwksClient` (new, behind `oidc`)

```rust
pub struct JwksClient {
    source: JwksSource,               // Url(String) | Discover(issuer String)
    http: reqwest::Client,
    cache: Arc<RwLock<Option<CachedJwks>>>,   // set + expiry
}
struct CachedJwks { set: JwkSet, expires_at: Instant }
```

- `async fn decoding_key(&self, kid: &str) -> Result<(DecodingKey, Algorithm)>`:
  serve from cache when fresh and the `kid` is present; otherwise resolve the
  `jwks_uri` (discovery on first use if `source` is an issuer), fetch the JWKS,
  update the cache, and look up the `kid`. One forced refetch on cache-miss
  covers rotation.
- **Cache TTL:** honor the JWKS response `Cache-Control: max-age` when present;
  otherwise default to 1 hour. `jwks_from_set` seeds the cache with a far-future
  expiry so it never fetches.
- The verify path in the middleware is async for the `Jwks` source; the `Static`
  source keeps the existing synchronous `decode`.

### Packaging

- New feature: `oidc = ["jwt", "dep:reqwest"]`, off by default.
- `reqwest` added as an **optional** dependency with
  `default-features = false, features = ["json", "rustls-tls"]` — rustls (pure
  Rust), no OpenSSL/native-tls C dependency, consistent with the crate's ethos.
  (`reqwest` is already a dev-dependency; this promotes it to an optional
  runtime dep.)
- `jsonwebtoken`'s JWK support (`jwk::{Jwk, JwkSet}`, `DecodingKey::from_jwk`) is
  available in the pinned 10.4 — no extra jsonwebtoken feature needed.

## Providers

OIDC discovery covers Clerk / Auth0 / Cognito / Supabase generically (all serve
`.well-known/openid-configuration`), so v1 ships **discovery + explicit
`jwks(url)`** plus a **cookbook** documenting each provider's `issuer` — not four
coded presets (they would be one-line issuer strings, better as docs).

Cookbook issuer forms (docs): Clerk `https://<subdomain>.clerk.accounts.dev`,
Auth0 `https://<tenant>.auth0.com/`, Cognito
`https://cognito-idp.<region>.amazonaws.com/<pool-id>`, Supabase
`https://<project>.supabase.co/auth/v1`.

## Error handling

- Missing/invalid token (when not `optional`) → **401 Unauthorized**.
- JWKS fetch/discovery failure, or `kid` not found after refetch, or unsupported
  algorithm → **401** (log the cause via `tracing`; never leak provider internals
  to the client).
- `iss`/`aud`/`exp` mismatch → **401**.

## Testing

- **No network in tests.** A test generates an RSA keypair, builds a `JwkSet`
  from the public key (with a `kid`), signs a token with the private key, and
  constructs the verifier via `Jwt::jwks_from_set(set)`. Cases: valid token
  accepted + claims attached; wrong `kid` → 401; expired → 401; bad `aud`/`iss`
  → 401; unknown key → 401.
- An integration test dispatches through the middleware via `Ultimo::oneshot`
  with a `jwks_from_set` verifier and asserts a guarded route returns 200 for a
  good token and 401 otherwise.
- `jwks_from_set` doubles as a real offline/air-gapped feature, not just a test
  seam.

## Scope

- **v1:** RS256/ES256 (+ RS384/512, ES384) verification via remote JWKS,
  OIDC discovery, cache + rotation, `iss`/`aud`/`exp`, `jwks_from_set`, cookbook.
- **Out of scope:** OAuth2 login/redirect/token-exchange flows; opaque-token
  introspection; per-provider coded presets; JWE (encrypted tokens); automatic
  scope→`Principal` mapping beyond what the existing JWT path already does.
