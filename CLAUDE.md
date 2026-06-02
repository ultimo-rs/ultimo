# Ultimo — Developer Reference (for Claude Code / coding agents)

Ultimo is a modern Rust web framework: REST + JSON-RPC in one app, automatic
**TypeScript client generation** from Rust API definitions, and RFC 6455
WebSocket support with built-in pub/sub. Built on **Hyper 1.0 + Tokio**.

- Homepage: https://ultimo.dev · Docs: https://docs.ultimo.dev · Roadmap: https://docs.ultimo.dev/roadmap
- Repo: https://github.com/ultimo-rs/ultimo · Issues: https://github.com/ultimo-rs/ultimo/issues · Board: https://github.com/orgs/ultimo-rs/projects/1
- Current version: **0.2.1** · Edition 2021 · MSRV **1.75.0** · License MIT

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

## CI / repo state (as of revival, June 2026 — last commit was 2026-01-04)
- `.github/workflows/`: `label-pr.yml`, `project-automation.yml`, `release.yml` only.
  **There is NO build/test/clippy/fmt CI workflow** — biggest guardrail gap. Nothing
  blocks a regression on PR. Adding one is a priority (build + `--lib` tests +
  feature-gated integration tests + clippy + fmt + ideally `cargo-semver-checks`).
- Release automation lives in `release.yml` + `scripts/release.sh` (runs `cargo test -p ultimo --lib` only).
- `.moon/` present (moonrepo config) and `node_modules/` (for TS-client / website tooling).

## Roadmap (next milestone = v0.3.0)
- **v0.3.0 (next):** session management, test utilities, enhanced error handling
- v0.4.0: static file serving, response compression, hot reload, dev dashboard
- v0.5.0: multi-language client gen (Python/Go/Dart/Swift), advanced security
- v0.6.0: **MCP server for AI-assisted development** (worth dogfooding once built)
- v1.0.0: stable API + complete docs

## Conventions
- Use the Makefile targets, not raw cargo, where one exists.
- `default = []` means: when adding code under a feature, gate it with `#[cfg(feature = "...")]`
  and remember integration tests must enable the feature combo explicitly.
- Pre-1.0: breaking public-API changes are allowed but should be deliberate and changelogged
  (`CHANGELOG.md`). Once `cargo-semver-checks` is wired into CI, respect its verdict.
- Always run `cargo fmt --all` and `cargo clippy` before committing (`.githooks/` may enforce this).
