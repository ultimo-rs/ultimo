# Ultimo — Developer Reference (for Claude Code / coding agents)

Ultimo is a modern Rust web framework: REST + JSON-RPC in one app, automatic
**TypeScript client generation** from Rust API definitions, and RFC 6455
WebSocket support with built-in pub/sub. Built on **Hyper 1.0 + Tokio**.

- Homepage: https://ultimo.dev · Docs: https://docs.ultimo.dev · Roadmap: https://docs.ultimo.dev/roadmap
- Repo: https://github.com/ultimo-rs/ultimo · Issues: https://github.com/ultimo-rs/ultimo/issues · Board: https://github.com/orgs/ultimo-rs/projects/1
- Current version: **0.2.1** · Edition 2021 · MSRV **1.86.0** · License MIT

## ⚠️ THESE ARE PUBLISHED CRATES — read before changing any public code

Ultimo ships two crates to crates.io that real users depend on:
- **`ultimo`** — https://crates.io/crates/ultimo (the library)
- **`ultimo-cli`** — https://crates.io/crates/ultimo-cli (the binary; depends on `ultimo`)

Every change is a potential breaking change for downstream users. **The following
is part of the public API contract** — changing any of it can break consumers and
requires a deliberate version bump + CHANGELOG entry:

- Any `pub` item (types, fns, traits, enum variants, fields) and the `prelude`
- **Cargo feature names** (`websocket`, `database`, `sqlx-*`, `diesel-*`, `test-helpers`) — renaming/removing one breaks `features = [...]` downstream
- **MSRV** (`rust-version`) — raising it is a breaking change for users on older Rust (we are at 1.86.0)
- **Direct dependency version requirements** — raising a floor (e.g. `bytes`, `diesel`) constrains downstream resolution; types re-exported from a dep (e.g. `bytes::Bytes` in WebSocket messages) make that dep's version part of *our* API
- CLI subcommands / flags of `ultimo-cli`

### Always do, on EVERY change
1. **Decide the SemVer impact.** Pre-1.0 (0.x) rule: breaking change → bump **minor** (0.2.x → 0.3.0); additive/fix → bump **patch** (0.2.1 → 0.2.2). CI's `semver-checks` job will catch unintended breakage — respect its verdict, don't override it.
2. **Update `CHANGELOG.md`** in the same PR (Keep-a-Changelog style; the release workflow extracts the section).
3. **Keep `README.md` accurate** — it's the crates.io landing page.
4. **Don't break docs.rs** — public items need doc comments; doctests must compile (`cargo test --doc`). Gate doc-only feature needs via `[package.metadata.docs.rs]` if added.

### Always do, on a RELEASE
1. Bump `version` in root `Cargo.toml` (`[workspace.package]`) — both crates share it.
2. Finalize the `CHANGELOG.md` section for that version.
3. `cargo publish -p ultimo --dry-run` **and** `cargo publish -p ultimo-cli --dry-run` (the packaged tarball must build standalone — no path-dep leakage).
4. Publish order: **`ultimo` first, then `ultimo-cli`** (cli depends on the library). `release.yml` automates this on a version change to `main`.
5. If a published version ships a vulnerability or a serious bug → **`cargo yank`** that version and release a fixed patch.

> Heads-up: `main` currently carries an unreleased MSRV bump (1.75 → 1.86) and raised
> dep floors (`bytes`, `diesel`). These are breaking for old-Rust users, so the **next
> release should be `0.3.0`**, not a `0.2.x` patch.

## Workspace layout

Cargo workspace (`resolver = "2"`). Members:

| Crate / path | Role |
|---|---|
| `ultimo/` | **The framework.** Core library — everything below is a module here. |
| `ultimo-cli/` | `ultimo` CLI binary (clap). Subcommands: `generate` (TS client), `new` (scaffold project), `dev` (hot-reload server), `build`. |
| `coverage-tool/` | Custom coverage runner (`ultimo-coverage`), invoked by `make coverage`. |
| `examples/*` | Runnable example apps. NOTE: not all dirs under `examples/` are workspace members — `Cargo.toml` `members` is the source of truth. `react-app`, `benchmark`, `database-with-openapi` exist on disk but are **not** members (drift to reconcile). |

### `ultimo/src` modules (`lib.rs` is the map)
- `app.rs` — `Ultimo` app builder (main entry type)
- `context.rs` — `Context` (per-request state)
- `router.rs` — routing
- `handler.rs` — handler traits
- `middleware.rs` — middleware
- `rpc.rs` — JSON-RPC registry + **TypeScript client codegen** (`RpcRegistry`)
- `response.rs`, `error.rs` (`UltimoError`, `Result`), `validation.rs` (`validate`, uses `validator`)
- `openapi.rs` + `openapi/docs.rs` — OpenAPI spec + docs UI
- `database/` — `sqlx.rs` + `diesel.rs` (feature-gated, behind `database`)
- `websocket/` — `mod.rs`, `connection.rs`, `frame.rs`, `pubsub.rs`, `upgrade.rs` (behind `websocket` feature)

Public API surface (keep stable — pre-1.0 but watch breaking changes):
`Ultimo`, `Context`, `Result`, `UltimoError`, `RpcRegistry`, `RpcRequest`, `RpcResponse`, `validate`, plus `ultimo::prelude::*`.

## Cargo features (`ultimo/Cargo.toml`)

`default = []` — **everything is opt-in.** Key features:
- `websocket` — the websocket module
- `test-helpers` — exposes `ultimo::websocket::test_helpers` (`Frame`, `OpCode`) for integration tests
- `database` → enabled transitively by `sqlx-postgres|mysql|sqlite`, `diesel-postgres|mysql|sqlite`

## ⚠️ Test invocation gotchas (verified June 2026 — READ BEFORE TOUCHING TESTS)

The codebase is healthy, but the **test plumbing is the part that rotted**. Specifics:

1. **CI and `make test` only run `cargo test --lib`** (107 unit tests, all pass). The
   `ultimo/tests/*.rs` **integration tests are never run in CI.** Treat them as
   currently-unguarded until CI is fixed.
2. **Integration tests need feature flags.** The websocket integration tests use both
   the `websocket` module and the `test_helpers` module, so they only compile with:
   ```bash
   cargo test -p ultimo --features "websocket,test-helpers"
   ```
   A bare `cargo build --all-targets` or `cargo test` (no features) **fails to compile**
   these tests — that is expected, not a regression.
3. **`--all-features` requires system libraries.** `diesel-mysql`/`sqlx-mysql` link
   against `libmysqlclient`, and the postgres backends need `libpq`. On a machine
   without them, `cargo test --all-features` / `make check` **fails at the linker**
   (`ld: library 'mysqlclient' not found`) — an environment gap, not code rot.
   Install `mysql-client` + `libpq` (e.g. `brew install mysql-client libpq`) or scope
   features to what you need.

Verified-good invocations:
```bash
cargo fmt --all --check                                  # clean
cargo test -p ultimo --lib                               # 107 pass
cargo test -p ultimo --features "websocket,test-helpers" # integration tests pass
```

## Commands (Makefile is the canonical entry point)

```bash
make help            # list targets
make build           # cargo build --release
make test            # cargo test --lib --no-fail-fast   (UNIT ONLY — see gotchas)
make test-summary    # ./scripts/test-summary.sh
make check           # clippy --all-targets --all-features + fmt --check (needs system libs)
make fmt             # cargo fmt --all
make coverage        # cargo run --release -p ultimo-coverage  → target/coverage/html
make doc             # cargo doc --no-deps --open
make ci              # check + test + coverage
```

CLI usage (the product surface):
```bash
cargo run -p ultimo-cli -- new <name> --template <basic|fullstack|api-only|rpc|production>
cargo run -p ultimo-cli -- generate --path <dir> --output <dir> [--watch]   # TS client gen
cargo run -p ultimo-cli -- dev --port <n>
cargo run -p ultimo-cli -- build --profile <debug|release>
```

## Benchmarks
`ultimo/benches/`: `websocket_bench.rs`, `websocket_pubsub_bench.rs` (criterion, `harness = false`).
Perf claims (158k+ req/s) live or die by these — guard against regressions before changing hot paths.

## CI / repo state (June 2026)
- **`ci.yml`** is the guardrail (push + PR to `main`): jobs `fmt + clippy`, `test`
  (ubuntu + macOS: lib + feature-gated integration + CLI tests + benches), `test-db`
  (`--all-features` + sqlite db tests), `MSRV` (reads `rust-version` from Cargo.toml),
  `semver-checks` (public-API breakage vs the published crate), and `cargo-audit`
  (RUSTSEC; fails on new advisories, ignores 6 known sqlx-0.8 ones).
- Other workflows: `release.yml` (auto-publish on version bump to `main`),
  `project-automation.yml` (org-project board; `continue-on-error` — needs a
  project-scoped PAT to actually function), `label-pr.yml`.
- **`Cargo.lock` is committed** (workspace ships a binary). `main` branch protection
  requires 1 review; solo merges use admin override.
- `.moon/` present (moonrepo config) and `node_modules/` (for TS-client / website tooling).

## Roadmap (next milestone = v0.3.0)
- **v0.3.0 (next):** session management, test utilities, enhanced error handling
- v0.4.0: static file serving, response compression, hot reload, dev dashboard
- v0.5.0: multi-language client gen (Python/Go/Dart/Swift), advanced security
- v0.6.0: **MCP server for AI-assisted development** (worth dogfooding once built)
- v1.0.0: stable API + complete docs

## Frontend examples (MUST follow)
**When a feature can be used from the frontend, ship a frontend example for it.**
Ultimo's whole pitch is full-stack DX (typed TS clients, etc.), so any
frontend-relevant feature (routes/RPC, cookies, sessions, WebSocket, auth, SSE,
file upload…) must **create or update an example under `examples/`** that
demonstrates it from a client. Prefer a self-contained example (a Rust backend
serving an HTML+JS page, runnable with `cargo run -p <example>`, added to the
workspace `members` so CI builds it) unless a React example is the better fit.
The session feature's example is `examples/session-auth`. Update the relevant
example in the same PR as the feature — don't defer it.

**User-facing docs go to the docs SITE, not just `docs/`.** The published site
(docs.ultimo.dev) is **Vocs** under `docs-site/`: feature pages live in
`docs-site/docs/pages/*.mdx` and the sidebar is in `docs-site/vocs.config.ts`.
A new user-facing feature must add/update its `docs-site` page **and** the
sidebar entry in the same PR. The repo's top-level `docs/` is internal notes;
writing only there does NOT surface a feature to users. (Vercel auto-deploys the
site on merge to `main`.)

**Keep `api-reference.mdx` current with the public API.** Any change to the
public surface — a new/renamed/removed `pub` method or type, a new middleware,
new `Context`/`Ultimo` methods, or a new/changed Cargo feature — must update
`docs-site/docs/pages/api-reference.mdx` in the same PR (the relevant section:
`Ultimo` methods, Context/Request, Middleware, Feature Flags, etc.). It's the
canonical API list; don't let it drift behind the code.

**Keep `README.md` current too.** It's the GitHub landing page **and** the
crates.io front page. When a user-facing feature ships or status/version changes,
update `README.md` in the same PR: move items from "Coming Soon" → "Available
Now", add the feature to the list, and keep the install snippet (`ultimo = "0.x"`)
and badges accurate. Don't let it advertise shipped features as "coming soon"
(or unshipped ones as available).

## Conventions
- **These are published crates — see the "PUBLISHED CRATES" section at the top before
  changing any public code, features, MSRV, or dep floors.**
- Use the Makefile targets, not raw cargo, where one exists.
- `default = []` means: when adding code under a feature, gate it with `#[cfg(feature = "...")]`
  and remember integration tests must enable the feature combo explicitly.
- Breaking public-API changes must be deliberate, changelogged (`CHANGELOG.md`), and
  version-bumped. `cargo-semver-checks` runs in CI — respect its verdict.
- Always run `cargo fmt --all` and `cargo clippy` before committing (`.githooks/` may enforce this).
