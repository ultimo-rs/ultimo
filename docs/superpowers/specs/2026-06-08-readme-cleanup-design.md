# README cleanup — Design

**Date:** 2026-06-08
**Issue:** #94 (remaining half — README editorial cleanup)
**Status:** Approved, pre-implementation

## Goal

Turn the 1082-line `README.md` into a concise, **accurate** front door for the
crates.io + GitHub landing page. Today the bottom half is an internal build-spec
with stale/fictional content; the top half has API and version bugs. Trim it to
what a Rust developer actually wants, and make every retained claim and code
example true.

## Approach (chosen: A — trim to a front-door README)

Keep and refine the user-facing top; **delete the embedded build-spec**; fix API
bugs and stale strings; verify each retained claim against the code; lean on
docs.ultimo.dev for depth. (Rejected: full rewrite — discards good copy; minimal
delete-only — leaves redundancy/fluff.)

## Claim audit (verified against the codebase)

| Claim in current README | Reality | Action |
|---|---|---|
| "158k+ req/sec / 0.6ms / 15× faster" + perf table | already removed in #95 | (n/a — gone) |
| File uploads (multer; `upload.rs`; "is_image/is_pdf"; "### 8 File Upload Handling") | **not implemented** (no multipart in `ultimo/src`) | **remove all upload claims** |
| Rate limiting / `bearer_auth` / `c.res` / typed `c.get` | not implemented / fictional | already cut from docs; **remove from README** |
| `ctx.param("id")?.parse()?` (quick start) | real is `ctx.req.param`; `parse()?` needs explicit error mapping | **fix the example so it compiles** |
| MSRV "1.75.0" | real MSRV is **1.86.0** | **fix** |
| `version = "0.3"` install snippets | current release is **0.4.0** | **fix to "0.4"** |
| `Project Structure` lists `upload.rs`, `guard.rs` | those files don't exist | **delete the section** |
| Auth (JWT/API-key/guards), sessions, CSRF, security headers, WebSocket, OpenAPI, RPC, TS client gen, CORS, validation, SQLx/Diesel | real | **keep (refined)** |
| CLI `generate` / `new` / `build` | real; `dev` is a stub (hot-reload is #15, planned) | keep `generate`/`new`/`build`; don't claim hot-reload |

## Target structure

The rewritten README, in order:

1. **Header** — logo, tagline, badges (keep the existing badge row), nav links.
2. **Intro** — one paragraph: secure-by-default, fast (Hyper + Tokio), type-safe
   full-stack with automatic TypeScript client generation.
3. **Why Ultimo** — a short honest bullet list: auto TS clients, REST + JSON-RPC,
   WebSocket + pub/sub, auth (JWT/API-key/guards) + security hardening, O(1)
   routing, SQLx/Diesel. No throughput numbers (link to `/performance`).
4. **Quick start** — install (`ultimo = "0.4"`) + a minimal `main.rs` that
   **compiles** against the real API (`use ultimo::prelude::*;`, `ctx.req.param`,
   `ctx.req.json`, `ctx.json(...).await`). MSRV note: 1.86.
5. **Type-safe clients** — the headline feature: define in Rust → generated TS
   client, short illustration + `ultimo generate` command.
6. **Feature flags** — the real opt-in list: `websocket`, `session`, `jwt`,
   `api-key`, `csrf`, `testing`, `sqlx-postgres|mysql|sqlite`,
   `diesel-postgres|mysql|sqlite` (all `default = []`).
7. **CLI** — `ultimo new`, `ultimo generate`, `ultimo build` (note `dev` is
   experimental). Real flags only.
8. **Docs · Examples · Contributing · License** — links; depth lives at
   docs.ultimo.dev. Keep the `examples/` pointer.

**Deleted outright:** "Core Philosophy", "Tech Stack Requirements", "Minimum
Supported Rust Version" (as a standalone stale section — fold 1.86 into Quick
start), "Project Structure", the entire "Feature Requirements" build-spec
(### 1–11) and anything after it that is build-spec/duplicate. The real
architecture already lives in `docs-site` + `CLAUDE.md`.

## Constraints

- **version-sync CI gate**: the root `CHANGELOG.md` version is what the gate
  reads, not the README — but keep README install snippets at `0.4` for accuracy.
- Preserve the existing badge row and logo block (they're correct).
- Every code block must compile against the real 0.4.0 API. No fabricated numbers
  (the no-unsubstantiated-benchmarks rule from #63/#95 holds).

## Success criteria

- README is a concise front door (~200–250 lines), no build-spec.
- Every retained claim is true; every code example compiles against 0.4.0.
- No fabricated performance numbers; no fictional APIs (uploads, `bearer_auth`,
  rate limiting, `c.res`).
- `grep` for `1.75`, `version = "0.3"`, `ctx.param(`, `upload`, `bearer_auth`,
  `rate_limit` in README → none remain (except where legitimately part of prose).
- Vercel docs-site build unaffected (README isn't part of it); GitHub renders cleanly.
