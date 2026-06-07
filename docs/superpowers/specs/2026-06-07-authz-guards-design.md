# Authorization Guards — Design

**Date:** 2026-06-07
**Milestone:** v0.4.0 (Security & Performance — Wave 2: auth & abuse)
**Epic:** #53 (Security: secure-by-default)
**Status:** Approved, pre-implementation

## Goal

Make the shipped authentication methods (JWT, API-key) *actionable*: let handlers
declare per-route access requirements that are enforced uniformly, regardless of
which auth method established the caller's identity. Capstone of the Wave 2 auth
arc.

## Key decision: a unified `Principal`

Guards must read identity/scopes from either JWT or API-key. Rather than branch
on `#[cfg]` in the guards, both auth middlewares normalize their result into a
single `Principal` stored on the `Context`. Guards read only the `Principal`, so
they're decoupled from which method ran, a future custom auth can populate the
same slot, and roles map cleanly onto scopes (e.g. `role:admin`). Scopes is one
concept, not two.

```rust
pub struct Principal {
    pub id: Option<String>,   // subject / key id
    pub scopes: Vec<String>,
}
impl Principal { pub fn has_scope(&self, scope: &str) -> bool; }
```

Lives in `ultimo::auth` (the module is compiled when `any(feature = "jwt",
feature = "api-key")`). The `Context` gets a `principal` slot + guards gated the
same way. No new feature flag — guards exist whenever an auth method does.

## Population

- **API-key middleware** (on success): `Principal { id: Some(identity.id), scopes: identity.scopes }`.
- **JWT middleware** (on success): `Principal { id: <sub>, scopes: extract_scopes(claims) }`.
  `extract_scopes` parses the OAuth2-standard **`scope`** (space-delimited string)
  plus **`scopes`**/**`scp`** (array or string), de-duplicated. Custom claim
  shapes: read `ctx.jwt_claims()` directly (documented).

Both also keep their existing raw accessors (`ctx.jwt_claims()`, `ctx.api_key()`).

## Guard API (`Context` methods, async, compose with `?`)

| Method | Behavior |
|---|---|
| `principal() -> Option<Principal>` | The normalized caller identity, if authenticated. |
| `require_auth() -> Result<Principal>` | The principal, or **401** if unauthenticated. |
| `require_scope(s) -> Result<()>` | **401** if unauthenticated, **403** if authenticated but missing `s`. |
| `require_any_scope(&[s]) -> Result<()>` | 403 unless at least one scope matches. |
| `require_all_scopes(&[s]) -> Result<()>` | 403 unless all scopes are present. |

```rust
app.get("/admin", |ctx: Context| async move {
    ctx.require_scope("admin").await?;          // 401 or 403, else proceeds
    ctx.json(json!({ "ok": true })).await
});
```

401 → `UltimoError::Unauthorized`, 403 → `UltimoError::Forbidden` (existing variants).

## Non-goals (deferred)

- Per-route middleware / declarative attribute guards (Ultimo routing takes a
  single handler; `Context` guards are the idiomatic fit and more flexible).
- Role hierarchies / policy engines / RBAC tables — model roles as scopes for now.
- Configurable JWT scope-claim name — standard `scope`/`scopes`/`scp` covered;
  custom shapes read `jwt_claims()`.

## Testing

- Unit (`jwt.rs`): `extract_scopes` handles `scope` string, `scopes`/`scp` arrays,
  combined + de-dup.
- Integration `tests/authz.rs` (`#![cfg(feature = "jwt")]`): unauthenticated → 401;
  authenticated without scope → 403; with scope → 200; `require_any`/`require_all`.
- `tests/api_key.rs`: a guarded route verifying the API-key → `Principal` → guard
  path (scopes from `ApiKeyIdentity`).

## Surfaces

- Extend `examples/jwt-auth` with a scope-guarded `/api/admin` route (runnable demo).
- New `docs-site/docs/pages/authorization.mdx` under the Authentication group +
  sidebar; update `api-reference.mdx`, `README.md`, `roadmap.mdx`.
- CI: add `cargo test -p ultimo --features "jwt" --test authz` to the test job.
- Additive change → release-plz bumps from the `feat:` commit; `semver-checks` confirms.
