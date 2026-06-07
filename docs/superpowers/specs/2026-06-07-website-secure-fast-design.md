# Website: honest "Secure & Fast" homepage — Design

**Date:** 2026-06-07
**Milestone:** v0.4.0 (Security & Performance)
**Issue:** #63 (website: advertise "Secure & Fast" pillars + badges)
**Status:** Approved, pre-implementation

## Goal

Make the marketing site (`website/`, a Next.js + Tailwind + shadcn app) advertise
Ultimo's two headline pillars — **secure** and **fast** — using only claims we can
stand behind, and add the Security pillar (currently absent from the homepage).
This both delivers #63 and removes a credibility risk.

## Why this is needed (the core constraint)

The current homepage publishes **fabricated, unsubstantiated numbers** that directly
contradict the just-shipped `/performance` docs page (which deliberately refuses
head-to-head numbers without controlled hardware):

- `components/stats-section.tsx`: "158k+ req/sec", "0.6ms Latency", "15× Faster",
  and a req/s bar chart (Ultimo 158,247 beating Axum/Hono/FastAPI).
- `components/comparison-section.tsx`: a perf row "158k+ vs Axum 152k…".
- `components/features-section.tsx`, `app/features/page.tsx`: "158,000+ req/s".

There is **no benchmark data** behind any of these. Publishing them invites the
Rust community to reproduce-and-debunk, damaging both pillars. We replace them with
substantiated claims (decision: "Replace with honest claims").

## Accuracy audit (verified against the codebase)

| Claim on site | Reality | Action |
|---|---|---|
| 158k req/s · 0.6ms · 15× faster · req/s chart | no data; Ultimo runs on the same Hyper/Tokio as axum/actix — not faster | **remove**; replace with honest perf proof |
| Comparison "Performance" row (Ultimo > axum/actix) | false (same core stack) | **remove the row** |
| Rate Limiting ✓ | **not implemented** (`grep` finds no `rate_limit` in `ultimo/src`) | mark **Planned** |
| Dev Server ✓ Built-in | `ultimo dev` is a stub; real hot-reload is #15 (v0.5.0) | mark **Planned** |
| TS types (auto), OpenAPI, JSON-RPC, REST, validation, CORS, client gen | real | keep ✓ |
| Authentication | real now (sessions, JWT, API-key, guards) | keep ✓ |

## Approach

**Re-content the existing components in place** — preserve the current visual
language (colors, layout, shadcn components); change only the *content* and add one
new section. Lowest risk, no redesign. (Rejected alternatives: reorganizing section
order — unnecessary churn; deleting perf claims without adding the security pillar —
misses the #63 goal.)

## Section-by-section design

1. **Hero** (`hero-section.tsx`) — keep. Refine the subhead to name the pillars
   (secure · fast · type-safe full-stack). Add a **verifiable badge strip**:
   crates.io version, CI status, MIT license, "100% safe Rust", MSRV 1.86. (All
   real/verifiable; use shields.io or static badges.)

2. **Features grid** (`features-section.tsx`) — keep the card grid. Re-content the
   "Blazing Fast" card to honest bullets: O(1) routing · built on Hyper + Tokio ·
   regression-guarded benchmarks · zero-cost safe Rust (remove "158,000+ req/s").
   Add a **"Secure by Default"** card (safe Rust, sessions, JWT/API-key/guards,
   CSRF, security headers).

3. **Performance section** (`stats-section.tsx`) — remove 158k/0.6ms/15× and the
   req/s bar chart. Replace with:
   - Three honest claims: **O(1) routing** (constant-time regardless of route
     count), **Built on Hyper + Tokio**, **Regression-guarded** (every PR
     benchmarked).
   - An honest **relative** visual: routing lookup stays *flat* as routes grow
     (10 → 500), labeled "in-process micro-benchmark, relative." No absolute
     throughput, no competitor bars.
   - CTA: "Reproduce it yourself" → `/performance` (docs).

4. **NEW Security section** (`components/security-section.tsx`) — the second
   pillar, mirroring the performance section's treatment. Headline "Secure by
   default." Real capability list: `#![forbid(unsafe_code)]` (100% safe Rust),
   secure-by-default sessions & cookies, JWT + API-key auth, authorization guards
   (scopes), CSRF protection, security-headers middleware, request body-size
   limits, supply-chain CI (`cargo-audit` + `cargo-deny`). CTA → `/security` docs.
   Insert in `app/page.tsx` right after the performance section so the two pillars
   sit together.

5. **Type-Safe section** (`type-safe-section.tsx`) — keep unchanged (genuine
   differentiator: Rust → TS codegen).

6. **Comparison table** (`comparison-section.tsx`) — reframe thesis from "we're
   faster" to **"batteries-included full-stack DX"** (the real advantage over the
   intentionally-minimal axum/actix/etc.). Remove the Performance row. Fix Rate
   Limiting and Dev Server to "Planned". Keep the verified ✓ rows. Update the
   caption accordingly.

7. **`app/features/page.tsx`** — remove the "152k+ req/sec throughput" line; align
   with the honest perf claims.

8. **CTA + footer** — keep (footer already links Performance).

## Out of scope (follow-ups)

- README/roadmap also claim "Rate Limiting — Available 0.1.0"; correct in a small
  separate PR (keep this one to the website).
- Actually *implementing* rate limiting / hot-reload dev server (existing roadmap
  items).
- Published head-to-head benchmark numbers (need controlled hardware; recipe in
  `BENCHMARKS.md`).

## Success criteria

- No unsubstantiated performance number anywhere on the site.
- Both pillars (secure, fast) represented with true, specific claims.
- Comparison table accurate against the current codebase.
- `npm run build` (Next.js static export) succeeds; Vercel preview renders.
- Visual language unchanged (no design regression).
