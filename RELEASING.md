# Releasing Ultimo

How we version and ship the `ultimo` and `ultimo-cli` crates. Releases are
**automated** — in normal operation you merge a single PR and everything else
(crates.io, GitHub Release, tags, changelog, docs/site versions) follows.

## 1. Versioning policy (SemVer)

We follow [Semantic Versioning](https://semver.org). The **single source of
truth** is the workspace version in the root `Cargo.toml`; both crates share it
and move together.

While we are pre-1.0 (`0.x`):

| Change | Bump | Example |
|---|---|---|
| Breaking change (removed/changed public API, **MSRV raised**, removed/renamed Cargo feature, raised a direct-dep floor that constrains consumers) | **minor** | `0.2.1 → 0.3.0` |
| New public API, new feature, bug fix (backward compatible) | **patch** | `0.2.1 → 0.2.2` |

> In `0.x`, the **minor** position is the breaking slot — there is no separate
> major bump until 1.0. After 1.0 we switch to standard major/minor/patch.

What counts as public API (and therefore breaking to change): any `pub` item +
the `prelude`, **Cargo feature names**, **MSRV** (`rust-version`), **direct-dep
version floors**, types re-exported from dependencies (e.g. `bytes::Bytes`), and
`ultimo-cli` flags. See the "Published crates" section of `CLAUDE.md`.

**Enforcement:** the CI `semver-checks` job (`cargo-semver-checks`) diffs the
public API against the published crate and fails on undeclared breakage. Respect
its verdict — if it flags a break, bump the minor version (don't override).

## 2. Commit convention (Conventional Commits)

Commits use [Conventional Commits](https://www.conventionalcommits.org). This is
what lets release-plz derive the next version and the changelog automatically.

| Prefix | Meaning | Changelog section | Version effect |
|---|---|---|---|
| `feat:` | new feature | Added | minor (in 0.x) |
| `fix:` | bug fix | Fixed | patch |
| `perf:` | performance | Performance | patch |
| `refactor:` | internal change | Changed | patch |
| `docs:` | docs | Documentation | none |
| `chore:`, `ci:`, `test:`, `style:` | housekeeping | (skipped) | none |

Mark a breaking change with a `!` (e.g. `feat!: …`) or a `BREAKING CHANGE:`
footer — release-plz will bump the minor accordingly.

## 3. The automated release flow (release-plz)

We use [release-plz](https://release-plz.dev) (`.github/workflows/release-plz.yml`,
config in `release-plz.toml`).

```
merge feature PRs (conventional commits) to main
        │
        ▼
release-plz opens / updates a "release PR"
   • bumps the workspace version per the commits since last release
   • updates CHANGELOG.md
        │
   (you run scripts/sync-versions.sh on that PR — see §4 — and review)
        │
        ▼
merge the release PR
        │
        ▼
release-plz: publishes ultimo → ultimo-cli to crates.io,
             creates the git tag vX.Y.Z, opens the GitHub Release
        │
        ▼
Vercel auto-redeploys website + docs-site (docs.rs rebuilds on publish)
```

You never hand-edit the version or changelog for a normal release — you review
and merge the release PR.

### Required repository secrets
- **`CARGO_REGISTRY_TOKEN`** — a crates.io API token with publish scope.
- **`RELEASE_PLZ_TOKEN`** — a Personal Access Token (or GitHub App token) so the
  release PR triggers CI. Without it the workflow falls back to `GITHUB_TOKEN`,
  which still opens the PR but **won't run CI on it** (GitHub blocks token-created
  events from triggering workflows). A PAT/App token is strongly recommended.

## 4. Keeping the sites/docs in sync

`Cargo.toml` is the source of truth. These places also display/declare a version
and are checked by `scripts/check-versions.sh` (CI gate `version-sync`):

- `website/package.json`
- `docs-site/package.json`
- `website/components/hero-section.tsx` (the "vX.Y.Z Now Available" badge)
- `CHANGELOG.md` (top entry) and `docs-site/docs/pages/changelog.mdx`

When the release PR bumps `Cargo.toml`, run:

```bash
scripts/sync-versions.sh   # propagate the version to the JS/TSX sites
```

and commit the result to the release PR branch. The `version-sync` CI gate will
**fail the release PR** until these match, so drift can't be merged. Update the
docs-site changelog (`changelog.mdx`) to mirror the `CHANGELOG.md` entry.

> Future enhancement: have the sites read the version at build time from a single
> generated file so even this step is automatic. Until then, `sync-versions.sh`
> + the CI gate keep it correct.

## 5. Manual / emergency release

If you must release without release-plz:

```bash
# 1. Bump the version in the root Cargo.toml ([workspace.package] version)
# 2. Move CHANGELOG.md [Unreleased] → [X.Y.Z] - <date>; mirror in changelog.mdx
scripts/sync-versions.sh
scripts/check-versions.sh          # must pass
cargo publish -p ultimo --dry-run
cargo publish -p ultimo-cli --dry-run --allow-dirty
# 3. Commit, open a PR, merge — release-plz's `release` job will publish + tag,
#    or publish manually with `cargo publish -p ultimo` then `-p ultimo-cli`.
```

## 6. After a release
- Verify both crates show the new version on crates.io.
- Verify [docs.rs](https://docs.rs/ultimo) built successfully.
- Verify the website + docs-site redeployed with the new version.
- If a published version has a serious bug or vulnerability: `cargo yank
  --version X.Y.Z` the affected crate and ship a fixed release.

## Pre-1.0 vs 1.0
The pre-1.0 rules above apply until we tag `1.0.0` (the roadmap's stable-API
milestone). At 1.0 we switch to standard SemVer (breaking → major).
