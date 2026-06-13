# Ultimo — Developer Reference (for Claude Code / coding agents)

Ultimo is a modern Rust web framework: REST + JSON-RPC in one app, automatic
**TypeScript client generation** from Rust API definitions, and RFC 6455
WebSocket support with built-in pub/sub. Built on **Hyper 1.0 + Tokio**.

- Homepage: https://ultimo.dev · Docs: https://docs.ultimo.dev · Roadmap: https://docs.ultimo.dev/roadmap
- Repo: https://github.com/ultimo-rs/ultimo · Issues: https://github.com/ultimo-rs/ultimo/issues · Board: https://github.com/orgs/ultimo-rs/projects/1
- Current version: **0.3.0** · Edition 2021 · MSRV **1.86.0** · License MIT

## ⚠️ THESE ARE PUBLISHED CRATES — read before changing any public code

Ultimo ships two crates to crates.io that real users depend on: **`ultimo`**
(the library) and **`ultimo-cli`** (the binary; depends on `ultimo`). Every
change is a potential breaking change. **The public API contract** — changing
any of it requires a deliberate version bump:

- Any `pub` item (types, fns, traits, enum variants, fields) and the `prelude`
- **Cargo feature names** (`websocket`, `testing`, `session`, `csrf`, `database`, `sqlx-*`, `diesel-*`, `test-helpers`) — renaming/removing breaks `features = [...]` downstream
- **MSRV** (`rust-version`) — raising it is breaking (we're at 1.86.0)
- **Direct dependency floors** — raising one (e.g. `bytes`, `diesel`) constrains downstream resolution; a dep type re-exported in our API (e.g. `bytes::Bytes` in WebSocket messages) makes that dep's version part of _our_ API
- CLI subcommands / flags of `ultimo-cli`

### On EVERY change

1. **Decide SemVer impact.** Pre-1.0 (0.x): breaking → bump **minor** (0.3.x → 0.4.0); additive/fix → bump **patch**. CI's `semver-checks` enforces this against the published crate — respect its verdict.
2. **Don't hand-edit `CHANGELOG.md`** — release-plz regenerates it from your conventional commits. Write a good commit message; that _is_ the changelog entry.
3. **Keep `README.md` accurate** — it's the crates.io landing page.
4. **Don't break docs.rs** — public items need doc comments; doctests must compile (`cargo test --doc`).

### On a RELEASE (automated by release-plz — rarely done by hand)

release-plz opens a release PR that bumps `version` in root `Cargo.toml`
(`[workspace.package]`, shared by both crates) and writes the `CHANGELOG.md`
section from commits. Merging it publishes **`ultimo` then `ultimo-cli`** to
crates.io and tags a release. A published vuln/serious bug → **`cargo yank`** it
and ship a fixed patch.

## Workspace layout

Cargo workspace (`resolver = "2"`). Members:

| Crate / path     | Role                                                                                                                                                                   |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ultimo/`        | **The framework.** Core library — everything below is a module here.                                                                                                   |
| `ultimo-cli/`    | CLI binary (clap). Subcommands: `generate` (TS client), `new` (scaffold), `dev` (hot-reload server), `build`.                                                          |
| `coverage-tool/` | Custom coverage runner (`ultimo-coverage`), invoked by `make coverage`.                                                                                                |
| `examples/*`     | Runnable example apps. `Cargo.toml` `members` is the source of truth — some dirs (`react-app`, `benchmark`, `database-with-openapi`) exist on disk but aren't members. |

### `ultimo/src` modules (`lib.rs` is the map)

- `app.rs` (`Ultimo` builder), `context.rs` (`Context`), `router.rs`, `handler.rs`, `middleware.rs`
- `rpc.rs` — JSON-RPC registry + **TypeScript client codegen** (`RpcRegistry`)
- `response.rs`, `error.rs` (`UltimoError`/`Result`), `validation.rs` (`validate`), `openapi.rs` (+ `openapi/docs.rs`)
- `cookie.rs` (core), `session/` (`session`), `csrf.rs` (`csrf`), `testing/` (`testing`)
- `database/` = `sqlx.rs` + `diesel.rs` (behind `database`); `websocket/` (behind `websocket`)

Public API surface (keep stable): `Ultimo`, `Context`, `Result`, `UltimoError`,
`RpcRegistry`, `RpcRequest`, `RpcResponse`, `validate`, plus `ultimo::prelude::*`.
The crate is `#![forbid(unsafe_code)]` — 100% safe Rust, no `unsafe`.

## Cargo features (`ultimo/Cargo.toml`)

`default = []` — **everything is opt-in.** When adding code under a feature, gate
it with `#[cfg(feature = "...")]`. Features: `websocket`, `testing`
(TestClient/assertions/fixtures), `session`, `csrf`, `test-helpers` (exposes
`websocket::test_helpers` for integration tests), `database` (enabled
transitively by `sqlx-postgres|mysql|sqlite`, `diesel-postgres|mysql|sqlite`).

## ⚠️ Test invocation gotchas (READ BEFORE TOUCHING TESTS)

1. **Integration tests need feature flags.** The `tests/*.rs` files use
   feature-gated modules, so a bare `cargo test` (no features) **fails to
   compile** them — that's expected, not a regression. Enable the combo:
   ```bash
   cargo test -p ultimo --features "websocket,test-helpers,testing,session,csrf"
   ```
2. **`--all-features` needs system libs.** `diesel-mysql`/`sqlx-mysql` link
   `libmysqlclient`; postgres backends need `libpq`. Without them
   `--all-features` / `make check` **fails at the linker** — an env gap, not code
   rot. Install them (`brew install mysql-client libpq`) or scope features.

Verified-good invocations:

```bash
cargo fmt --all --check
cargo test -p ultimo --lib
cargo test -p ultimo --features "websocket,test-helpers,testing,session,csrf"
cargo test -p ultimo --features "testing,sqlx-sqlite,diesel-sqlite"   # DB-backed tests
```

## Commands (Makefile is the canonical entry point)

```bash
make help            # list targets
make build           # cargo build --release
make test            # cargo test --lib --no-fail-fast   (UNIT ONLY — see gotchas)
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

`ultimo/benches/` (criterion, `harness = false`): `websocket_bench.rs`,
`websocket_pubsub_bench.rs`. Perf claims (158k+ req/s) live or die by these —
guard against regressions before changing hot paths.

## CI / repo state

- **`ci.yml`** (push + PR to `main`): `fmt + clippy`, `test` (ubuntu + macOS:
  lib + feature-gated integration + CLI + benches), `test-db` (sqlite DB tests),
  `MSRV` (reads `rust-version` from Cargo.toml), `semver-checks` (public-API
  breakage vs published crate), `cargo-audit` (RUSTSEC), `cargo-deny`
  (advisories/bans/sources), `version-sync`.
- **`release-plz.yml`** automates releases (opt-in per crate); CodeQL + Vercel
  (docs-site + website) run as checks. **`Cargo.lock` is committed.** `main`
  branch protection requires 1 review → solo merges use `--admin` override.

## Shipping changes — use the `ship-feature` skill

`.claude/skills/ship-feature/SKILL.md` encodes the standard loop:
**branch → TDD → docs surfaces → verification gate → PR → CI green →
`gh pr merge --squash --admin --delete-branch` → sync.** Conventional commits
throughout. Invoke it for any change to this repo.

## Roadmap

- **v0.3.0 (current):** sessions + cookies, CSRF, security headers, testing utilities, `#![forbid(unsafe_code)]`, client-IP extraction — all shipped.
- v0.4.0: **Security & Performance** (Ultimo's two headline pillars) — auth (JWT/API-key), authz guards, rate-limit/timeouts, IP allow/deny, continuous benchmarking + `/performance` page.
- v0.5.0: static files + SPA fallback, compression, hot reload, dev dashboard, multi-language client gen, **deployment guides**.
- v0.6.0: MCP server for AI-assisted development. v1.0.0: stable API + complete docs.

## Documentation surfaces (MUST keep current, in the same PR as the change)

The `ship-feature` skill walks these — summary of the rules:

1. **`README.md`** — GitHub + crates.io landing page. Move shipped items "Coming Soon" → "Available Now"; keep install snippet (`ultimo = "0.x"`) + badges accurate. Never advertise a shipped feature as coming-soon (or vice-versa).
2. **`docs-site/docs/pages/api-reference.mdx`** — canonical public-API list. Update on any new/renamed/removed `pub` method/type, new middleware, new `Context`/`Ultimo`/`Request` method, or changed Cargo feature.
3. **`docs-site/docs/pages/<feature>.mdx` + `docs-site/vocs.config.ts` sidebar** — any user-facing feature needs a docs-site page (Vocs → docs.ultimo.dev, auto-deploys on merge) AND a sidebar entry. Top-level `docs/` is internal notes only; writing there does NOT surface a feature to users.
4. **`examples/`** — if usable from a frontend (routes/RPC, cookies, sessions, WebSocket, auth, SSE, file upload…), create/update a runnable example demonstrating it from a client and add it to workspace `members` so CI builds it. Pattern: `examples/session-auth` (Rust backend serving HTML+JS, `cargo run -p <example>`). Same PR — don't defer.
5. Keep **`docs-site/docs/pages/roadmap.mdx`** honest — move shipped features to the right version section.

## Conventions

- **Published crates** — see the PUBLISHED CRATES section before changing public code, features, MSRV, or dep floors.
- Use Makefile targets, not raw cargo, where one exists.
- Breaking public-API changes must be deliberate and version-bumped; `cargo-semver-checks` runs in CI — respect its verdict.
- Run `cargo fmt --all` and `cargo clippy` before committing (`.githooks/` may enforce this).

## Website SEO conventions (`website/`)

Every new page on ultimo.dev MUST include:

1. **Canonical URL** — `alternates: { canonical: "https://ultimo.dev/<path>" }` in the page's `metadata` export.
2. **Structured data (JSON-LD)** — pick the appropriate schema:
   - Blog post → `BlogPosting` + `BreadcrumbList` (already automatic via `app/blog/[slug]/page.tsx`)
   - Tutorial/guide → add `HowTo` with steps
   - Feature/product page → `FAQPage` if it has Q&A, otherwise `WebPage`
   - Landing page → `Organization` / `SoftwareApplication` (already in root layout)
3. **Open Graph + Twitter meta** — title, description, `og:type`, image. Use the `metadata` export.
4. **Internal links** — link to at least 2-3 other pages on the site from new content.
5. **External links** — link to authoritative sources (docs.rs, rust-lang.org, tokio.rs, etc.) where relevant.
6. **Sitemap entry** — new routes are auto-included via `app/sitemap.ts`; blog posts via `getAllPosts()`. Static pages must be added manually to the sitemap array.

Validate schemas after deploy: https://search.google.com/test/rich-results
