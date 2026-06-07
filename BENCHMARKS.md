# Benchmarks

Ultimo's performance work has two tiers, for two different purposes. Understanding
the split matters: **don't compare numbers across tiers or across machines.**

| Tier | Tool | Measures | Where to run | Purpose |
|---|---|---|---|---|
| Framework overhead | criterion (in-process) | Ultimo's own cost (routing, dispatch, JSON, middleware) | anywhere, incl. CI | regression detection |
| End-to-end | `oha` (real HTTP) | req/s & latency over a socket | **controlled hardware only** | published / comparison numbers |

## Tier 1 — framework-overhead micro-benchmarks (criterion)

These drive the app **in-process** via `Ultimo::oneshot`, so they isolate
framework cost from network and OS scheduling noise. That makes them low-variance
and reproducible — the right basis for a regression gate. The numbers are
*relative* (compare routing vs dispatch, or this commit vs a baseline); the
absolute microseconds are only meaningful within one machine/run.

Source: [`ultimo/benches/http_bench.rs`](ultimo/benches/http_bench.rs). Benches:

- **dispatch_text** — full request → minimal text handler.
- **dispatch_json** — dispatch → JSON-serialized response.
- **routing/{static,param}/{10,100,500}** — radix-tree lookup as the table grows.
- **middleware_chain/{0,1,5,10}** — per-layer pass-through overhead.

### Running

```bash
make bench                              # all benches (incl. websocket)
cargo bench -p ultimo --bench http_bench   # just the HTTP-overhead suite
```

### Detecting regressions (the basis for the CI gate, #13)

criterion compares against a saved baseline:

```bash
# On main / a known-good commit:
cargo bench -p ultimo --bench http_bench -- --save-baseline main

# On your change — criterion reports % change vs the baseline:
cargo bench -p ultimo --bench http_bench -- --baseline main
```

### CI regression check (advisory)

`.github/workflows/bench.yml` runs on PRs that touch `ultimo/src`, the benches, or
`Cargo.toml`. It benchmarks the PR's **base commit** and the **PR head** on the
same runner and publishes the `critcmp` delta to the job's Step Summary. It is
**advisory** — it never fails the build, because shared CI runners are noisy
(treat regressions beyond ~10% as worth a look). Once the real noise floor is
known, flip it to enforcing by parsing `critcmp` output and failing the step
above a calibrated threshold.

## Tier 2 — end-to-end load benchmarks (`oha`)

Real requests-per-second over a socket, for the published numbers and the
framework comparison on the `/performance` page. **These are only trustworthy on
controlled hardware** — never a shared CI runner. Always report the machine,
Rust version, build profile, and tool version alongside the numbers.

### Protocol

1. Build the target server in release mode: `cargo run --release -p <example>`
   (e.g. a minimal JSON endpoint). Pin it to dedicated cores if possible.
2. Run the load from a **separate machine** (or at least a separate core set) to
   avoid the client starving the server:

   ```bash
   # https://github.com/hatoo/oha
   oha -z 30s -c 100 --no-tui http://<server>:3000/
   ```

   - `-z 30s` — sustained 30-second run (ignore the first few seconds of warm-up).
   - `-c 100` — concurrent connections; sweep (e.g. 50/100/200/500) to find the knee.
3. Record: req/s (mean), latency p50/p95/p99, and error count (must be 0).
4. For comparisons, run the **identical** endpoint + load profile against each
   framework (axum, actix, hono, express, fastapi) on the **same** hardware in the
   same session. Publish the methodology, not just the bar chart.

### Why not in CI

Cloud CI VMs are shared and noisy (±20–50% throughput swings), so absolute req/s
there is meaningless and would make any comparison dishonest. CI is for Tier-1
regression detection only.

## Findings & fixes

- **Routing was O(N) in route count — fixed (#89).** The suite's first run showed
  `routing/static` growing with the table size (≈1.7µs @10 → ≈26µs @500), i.e. a
  linear scan over every registered route. Fixed by indexing fully-static routes
  in an O(1) hash map keyed by `(method, path)` and scanning only parameterized
  routes. After: `routing/static` is **flat at ≈1.2µs** across 10/100/500 routes
  (~21× faster at 500), and `routing/param` dropped too (the scan no longer wades
  through static routes). A textbook example of the benchmark suite earning its keep.
