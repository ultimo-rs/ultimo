# Roadmap Revision — Design

**Date:** 2026-06-09
**Status:** Approved (brainstorm), pending spec review
**Artifact:** `docs-site/docs/pages/roadmap.mdx`

## Goal

Revise the Ultimo roadmap to **double down on the differentiator** — type-safe
end-to-end (Rust → typed TypeScript client) — with **1.0-focus discipline**.
Cut dated/vague/off-core items, keep only adoption-unblockers any real app
needs, add features that strengthen the moat, and reconcile the roadmap with
what actually shipped through 0.5.0.

## Lens / rationale

Ultimo's moat is the typed RPC + automatic TS client story; many Rust frameworks
(axum, actix) cover generic HTTP well. The roadmap should make the type-safe
story unbeatable and stop promising generic or speculative features that dilute
focus or that the project realistically won't build. Everything sequences toward
a credible 1.0.

## Reconciliation (roadmap is stale post-release)

- `0.4.1` shipped: static files + SPA fallback, response compression.
- `0.5.0` shipped: **client-gen** — TypeScript type derivation from Rust types
  (`ts-rs`), `ultimo::rpc::TS`, derived `query`/`mutation` (+ `*_with_types`),
  and `ultimo generate` now runs the project's `generate-client` bin. **This is
  not currently reflected on the roadmap and must be added** as a shipped 0.5.0
  entry (Feature Status row + Version Timeline section). The "(Current)" marker
  moves from 0.4.0 to 0.5.0.

## Cut (remove from roadmap)

| Item | Why |
|---|---|
| HTTP/2 Push | Deprecated; Chrome removed support. Dead end. |
| Long Polling | Niche; SSE (one-way) + WebSocket (bidirectional) already cover real-time. |
| Development Dashboard | A framework shouldn't ship a metrics-UI app; large, off-core. |
| Schema Introspection (AI) | Vague; overlaps with MCP server. |
| Smart Suggestions (AI) | Vague marketing; no concrete scope. |
| Go / Dart / Swift clients | Huge surface; dilutes TS-first focus. (Python kept as demand-driven, post-1.0.) |
| Mock Server | Testing utilities already shipped; fold in or drop. |

## Add (on-identity)

- **Typed RPC subscriptions / typed SSE** — server-push with derived event
  types. The standout differentiator (tRPC-style subscriptions, in Rust).
- **End-to-end typed errors** — RPC errors surfaced as typed results in the
  generated TS client.
- **`#[derive(UltimoType)]` wrapper** (codegen Phase 1.5) — removes the `ts-rs`
  direct-dependency papercut so the derive feels native.
- **HTTP graceful shutdown** + **request timeouts** — common production gaps
  (WebSocket graceful shutdown already exists).
- **Explicit 1.0 stabilization track** — public-API audit, doc completeness,
  SemVer freeze.

## Keep & sequence toward 1.0

- **0.6** — finish codegen: Phase 2b (scaffold `generate-client` into
  `ultimo new` templates) + Phase 3 (TypeScript docs rewrite, `generate
  --watch`); **deployment guides**; **advanced rate limiting**.
- **0.7** — **SSE (typed)**, **OAuth2**, **streaming responses**, HTTP graceful
  shutdown + request timeouts.
- **0.8** — **Redis sessions**, WebSocket compression (per-message deflate),
  hot reload (`ultimo dev`), `#[derive(UltimoType)]` wrapper.
- **0.9 → 1.0** — end-to-end typed errors, 1.0 stabilization (API freeze, docs
  completeness), **MCP server** *(if demand)*, **Python client** *(if demand)*.

## Integrations theme (added)

Same principle as everything else: a **generic capability in core + thin
presets/adapters + cookbook guides**, never bundled vendor SDKs.

- **Tier 1 (commit):** Frontend client adapters (TanStack Query / React hooks —
  pulled to 0.6 as the differentiator); Auth providers (OIDC/JWKS verification +
  presets for Clerk/Cognito/Auth0/Supabase, 0.7); Observability (OpenTelemetry +
  Prometheus, 0.7); Redis (sessions · cache · rate-limit, 0.8).
- **Tier 2 (demand-driven):** S3-compatible object storage; in-process
  background tasks.
- **Tier 3 (cookbook only, not framework code):** email (SMTP/Resend/Postmark),
  payments (Stripe webhooks), vendor message queues.

Per-vendor auth integrations (Clerk/Cognito/…) are intentionally NOT separate
features — they collapse to one OIDC/JWKS verifier + provider presets. Better
Auth is excluded (it's a TS-native backend, not a token issuer to verify).

## Out of scope

This is a documentation change to `roadmap.mdx` (plus reconciling the Feature
Status table and Version Timeline). No code changes. Each listed feature gets
its own brainstorm → spec → plan when picked up later.

## Testing / verification

- `version-sync` CI gate must stay green (roadmap edits don't touch versioned
  surfaces, but run `scripts/check-versions.sh` to be safe).
- Vocs docs-site build must succeed (valid MDX).
