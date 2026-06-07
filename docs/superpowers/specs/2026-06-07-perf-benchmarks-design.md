# Performance Benchmark Suite — Design

**Date:** 2026-06-07
**Milestone:** v0.4.0 (Security & Performance — Performance pillar)
**Epic:** #54 (Performance: prove & regression-guard the speed claim)
**Status:** Approved, pre-implementation

## Goal

Establish the foundation for Ultimo's performance pillar: reproducible benchmarks
that (a) measure the framework's own overhead with low enough noise to base a CI
regression gate on, and (b) define a credible methodology for the published
comparison numbers. This first deliverable is the *suite + methodology*; the CI
gate (#13) and the `/performance` page (#62) are follow-ups.

## Key decision: two benchmark tiers for two purposes

CI runners are noisy shared VMs (±20–50% throughput swings), so a single approach
can't serve both regression detection and headline numbers.

1. **Framework-overhead micro-benchmarks — criterion, in-process via `Ultimo::oneshot`.**
   Driving the app in-process isolates framework cost from network/OS noise, so
   these are low-variance and reproducible — the right basis for a regression gate.
   This is what this PR ships. Benches:
   - **routing** — radix-tree lookup across many registered routes (static + param).
   - **dispatch_text** — full request → minimal text handler.
   - **dispatch_json** — dispatch → serialized JSON response.
   - **middleware_chain** — dispatch through N pass-through middlewares (overhead).

2. **End-to-end load benchmarks — `oha` over real HTTP, on controlled hardware.**
   Real req/s vs axum/actix/etc. — the marketing figures. Only trustworthy off
   CI, so this PR ships a documented, reproducible *methodology + script*; the
   actual numbers are generated on a controlled machine for the `/performance`
   page.

This split avoids both flaky CI perf gates and dishonest cloud-VM "benchmarks."

## Deliverables (this PR)

- `ultimo/benches/http_bench.rs` — criterion (`harness = false`, no required
  features; uses `oneshot`). Uses the `async_tokio` criterion feature already in
  dev-deps.
- `[[bench]]` entry in `ultimo/Cargo.toml` for `http_bench`.
- `make bench` target (runs `cargo bench -p ultimo --features "websocket,test-helpers"`
  so all benches, incl. the existing websocket ones, run).
- `BENCHMARKS.md` — methodology: what each micro-bench measures, how to run
  locally, how to capture a criterion `--baseline` (sets up the #13 gate), and
  the controlled-hardware `oha` protocol for e2e/comparison numbers.
- Roadmap note.

## Non-goals (follow-ups)

- **CI regression gate (#13)** — criterion baseline compare with a generous (~10%)
  threshold, advisory before enforced. Separate PR.
- **`/performance` page (#62)** — published numbers + comparison, from controlled
  hardware. Separate PR.
- **Advertising (#63).**

## CI

No `ci.yml` change needed: the existing `test` job already builds all benches
(`cargo build -p ultimo --benches --features "websocket,test-helpers"`), which
will compile `http_bench` (it has no required features). Benches are **not run**
in CI here — running criterion is slow and the regression gate is #13.

## Notes

- Benchmarks measure *relative* framework overhead, not absolute machine speed —
  the numbers are only comparable within the same machine/run, which is exactly
  what regression detection needs.
- No SemVer impact (benches are not part of the public API).
