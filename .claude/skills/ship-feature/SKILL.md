---
name: ship-feature
description: >-
  The end-to-end workflow for shipping a change to the Ultimo framework — branch,
  TDD, update every documentation surface (api-reference, docs-site + sidebar,
  README, examples), run the verification gate, open a PR, watch CI go green, and
  admin-squash-merge. Use this whenever you're adding, changing, or removing
  anything in the Ultimo repo (a feature, a public API, a middleware, a Cargo
  feature, a bug fix) — even small changes — so nothing in the release checklist
  or the four doc-surface rules gets skipped. Trigger on "implement", "add",
  "ship", "fix", "let's build", "work on issue #N", or any request that ends in a
  PR to ultimo-rs/ultimo.
---

# Shipping a feature to Ultimo

This skill exists so feature work is **cheap, consistent, and never drops a doc
surface or a release-hygiene step.** Ultimo publishes two crates real users depend
on (`ultimo`, `ultimo-cli`), so every change is a potential breaking change and
every user-facing change must land its docs *in the same PR*. The cost of
forgetting is a follow-up PR, a stale docs site, or a broken downstream build —
all more expensive than doing it inline. Follow the steps in order.

> Read `CLAUDE.md` at the repo root for the authoritative rules — this skill is the
> operational loop *over* those rules, not a replacement. When the two disagree,
> CLAUDE.md wins.

## TodoWrite first

Create one todo per phase below (Branch → TDD → Docs → Gate → PR → Merge →
Cleanup). It keeps the doc surfaces from being forgotten once the code compiles —
the most common failure mode is stopping at "tests pass" and shipping without docs.

## 1. Branch

`main` is protected. Never commit to it directly.

```bash
git switch -c <type>/<short-slug>   # e.g. feat/security-headers, fix/router-precedence
```

Use the conventional-commit type as the prefix (`feat`, `fix`, `docs`, `chore`,
`refactor`, `perf`, `test`). This matters downstream: **release-plz derives the
next version and the CHANGELOG from your commit messages**, so the type you pick
here becomes the released changelog entry.

## 2. Implement with TDD

Invoke the TDD skill if you have it — write the failing test first, then the code.
Ultimo's whole value proposition is reliability; tests are the guardrail that lets
us move fast on a published crate.

Ultimo-specific implementation rules:

- **Gate new code behind a Cargo feature** when it's optional (`#[cfg(feature = "…")]`),
  and wire the feature in `ultimo/Cargo.toml`. Everything is `default = []` / opt-in.
- **Treat the public API as a contract.** Any new/renamed/removed `pub` item, feature
  name, MSRV, or dependency floor is a SemVer event. Pre-1.0 rule: breaking → bump
  **minor**, additive/fix → **patch**. CI's `semver-checks` job enforces this against
  the published crate — respect its verdict, don't override it.
- **Public items need doc comments**, and doctests must compile (`cargo test --doc`).
  docs.rs builds the public API; a missing doc or broken doctest breaks the build.
- **100% safe Rust** — the crate is `#![forbid(unsafe_code)]`. Don't reach for `unsafe`.
- **Integration tests need feature flags.** A bare `cargo test` won't compile the
  feature-gated `tests/*.rs` — that's expected, not a regression. Enable the combo
  (see the gate below).

## 3. Update every documentation surface (in this PR)

This is the step most likely to be skipped and the reason this skill exists. Walk
the four surfaces explicitly — skip one only after consciously deciding it doesn't
apply, not by forgetting:

1. **`docs-site/docs/pages/api-reference.mdx`** — update whenever the public surface
   changes: a new/renamed/removed `pub` method or type, a new middleware, new
   `Context`/`Ultimo`/`Request` methods, or a new/changed Cargo feature. This is the
   canonical API list; don't let it drift behind the code.
2. **`docs-site/docs/pages/<feature>.mdx` + `docs-site/vocs.config.ts` sidebar** — any
   *user-facing* feature needs a docs-site page (Vocs, deploys to docs.ultimo.dev on
   merge) **and** a sidebar entry. The top-level `docs/` dir is internal notes only —
   writing there does NOT surface a feature to users.
3. **`README.md`** — it's both the GitHub landing page and the crates.io front page.
   Move shipped items from "Coming Soon" → "Available Now", keep the install snippet
   (`ultimo = "0.x"`) and badges accurate. Never advertise a shipped feature as
   "coming soon" or vice-versa.
4. **`examples/`** — if the feature is usable from a frontend (routes/RPC, cookies,
   sessions, WebSocket, auth, SSE, file upload…), create or update a runnable example
   that demonstrates it from a client, and add it to the workspace `members` so CI
   builds it. Pattern to copy: `examples/session-auth` (a Rust backend serving an
   HTML+JS page, `cargo run -p <example>`). Update it in *this* PR, don't defer.

Also keep the **roadmap** (`docs-site/docs/pages/roadmap.mdx`) honest — move the
feature from Planned to the shipped version section if it was listed.

**Do not hand-edit `CHANGELOG.md`.** release-plz regenerates it from your
conventional commits on release. Get the changelog right by writing good commit
messages, not by editing the file. (CLAUDE.md's "update CHANGELOG" rule predates
release-plz automation — the commit *is* the changelog entry now.)

## 4. Run the verification gate

Run all of these and get them green before opening the PR. This mirrors what CI
runs, so a clean gate locally means a green PR.

```bash
cd /Users/ruslanelishaev/Desktop/projects/ultimo

# Formatting
cargo fmt --all --check

# Clippy across the feature surface, warnings = errors
cargo clippy -p ultimo --features "websocket,test-helpers,testing,session,csrf" \
  --all-targets -- -D warnings

# Library unit tests
cargo test -p ultimo --lib

# Feature-gated integration tests (won't compile without the features — expected)
cargo test -p ultimo --features "websocket,test-helpers,testing,session,csrf"

# Doctests (public API examples)
cargo test -p ultimo --doc --features "websocket,testing,session,csrf"
```

If you touched DB code, also run the sqlite-backed tests:
`cargo test -p ultimo --features "testing,sqlx-sqlite,diesel-sqlite"`.
(`--all-features` needs `libpq`/`libmysqlclient` system libs — scope features to
what you changed instead.)

If you touched a hot path, run the benches (`ultimo/benches/`) — perf claims are
guarded there.

## 5. Commit and open the PR

```bash
git add -A
git commit -m "feat(scope): concise summary"   # conventional commits — drives release-plz
git push --no-verify -u origin <branch>          # --no-verify: the gate already ran
gh pr create --fill
```

Write the commit body to explain the *why*. End the PR body with the Claude Code
generated-with line. Multiple commits are fine — they get squashed on merge, so the
**PR title** must be a clean conventional-commit line (that's what release-plz reads).

## 6. Watch CI to green

```bash
gh pr checks --watch
```

Don't merge on red. Fix forward (new commit on the same branch) and re-watch. The
jobs that gate the merge: `fmt + clippy`, `test` (ubuntu + macOS), `test-db`, `MSRV`,
`semver-checks`, `cargo-audit`, `cargo-deny`, `version-sync`.

## 7. Merge (admin override)

`main` requires 1 review and a solo dev can't approve their own PR, so use the admin
squash-merge:

```bash
gh pr merge --squash --admin --delete-branch
```

Squash keeps one clean conventional commit on `main` per feature — exactly what
release-plz needs to compute the next version and changelog.

## 8. Sync and clean up

```bash
git switch main && git pull
git branch -D <branch> 2>/dev/null || true
```

## Releasing is separate — and automated

You do **not** cut a release as part of shipping a feature. After merges land on
`main`, **release-plz** opens (or updates) a release PR that bumps the version and
writes the CHANGELOG from the accumulated conventional commits. Merging *that* PR
publishes `ultimo` then `ultimo-cli` to crates.io and tags a GitHub release. Your
job per feature ends at step 8; just make sure your commit messages are accurate
because they become the release notes.

## Quick recap

branch → TDD → **docs (api-reference · docs-site+sidebar · README · examples)** →
gate → PR → CI green → `gh pr merge --squash --admin` → sync. Conventional commits
throughout; never hand-edit CHANGELOG; release-plz handles the release.
