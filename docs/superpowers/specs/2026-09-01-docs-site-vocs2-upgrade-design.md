# Docs Site Modernization — Vocs 1.4 → 2.x Upgrade + Feature Adoption

**Date:** 2026-09-01
**Status:** Approved (design) — **not yet implemented** (deferred: parent session out of context budget)
**Scope:** `docs-site/` only (Vocs, deploys to docs.ultimo.dev via Vercel). Does **not** touch the `ultimo`/`ultimo-cli` crates or the marketing `website/`.

## Goal

Bring the docs site up to the richness of vocs.dev (which runs Vocs 2.8.5): version
dropdown, "Copy page for AI" / Ask AI, agent support, generated changelog with a
Versions view, and richer Markdown (badges, callouts, cards, file trees, steps,
benchmarks tables, code-snippet includes). The docs are **already Vocs** — just on
**1.4.1** — so the unlock is a major-version upgrade plus adopting 2.x features.

## Decisions (locked with the user)

- **Visual style:** clean Vocs 2.x **default** look (like vocs.dev) — minimal custom
  theming. No Ultimo-brand restyle in this effort.
- **Scope:** everything — upgrade + adopt features + a content-enrichment pass using
  the new directives.

## Why a major upgrade (not incremental)

Vocs 2.x is a Vite 7 + Waku + React 19 rewrite with a new config surface
(`import { defineConfig, Changelog } from 'vocs/config'`), MDX directives
(`:badge`, `:::benchmarks`, `::changelog`, cards, file-tree, steps), code-include
syntax (`// [!include path]`), built-in agent support (auto Markdown serving,
`/llms.txt`, `.md` URLs, "Copy page for AI", optional MCP), and a version dropdown.
None of these exist in 1.4.1. So the features the user wants require the upgrade.

## Architecture / phasing

Three phases, **each its own docs-site PR** (Phase 1 must land and deploy green
before 2 and 3). This is decomposed because the upgrade is breaking and must be
verified end-to-end (local build **and** Vercel deploy to docs.ultimo.dev) before
layering features on top.

### Phase 1 — Upgrade to Vocs 2.x (foundation)

- Bump `docs-site/package.json`: `vocs` `^1.4.1 → ^2` (+ React 19 / Vite 7 / peer
  deps as the 2.x package requires). Follow the official Vocs 2.x getting-started /
  config reference at execution time (fetch it fresh — the API moved to `vocs/config`).
- Migrate `docs-site/vocs.config.ts` to the 2.x shape: config import path, `sidebar`
  / `topNav` (verify shapes), `theme`, and any renamed options. Keep the same nav +
  sidebar structure (61 pages).
- Confirm the build output path Vercel expects (`docs/.vocs/dist` vs `docs/dist` —
  it changed in the past; see `docs-site/DEPLOYMENT.md`, which is stale) and update
  the Vercel project settings/`vercel.json` if needed.
- Verify: `pnpm --dir docs-site build` succeeds; `vocs preview` renders every page;
  `/llms.txt` + `/llms-full.txt` still emit; no MDX/directive breakage.
- **Deploy gate:** the Vercel docs-site preview must build green before merge — a
  broken docs.ultimo.dev is the main risk.

### Phase 2 — Adopt 2.x features (config)

- **Changelog generation:** `changelog: Changelog.github({ repo: 'ultimo-rs/ultimo' })`
  and replace `changelog.mdx`'s hand-maintained body with the `::changelog` directive
  (reads GitHub Releases — Ultimo already publishes them via release-plz). Note: this
  **removes** `changelog.mdx` from the `version-sync` gate's changelog check — update
  `scripts/check-versions.sh` so it no longer requires that file's version (the root
  `CHANGELOG.md` check stays). Coordinate with the version-sync rule.
- **Agent support:** enable the 2.x agent features (Copy page for AI, `.md` serving,
  optional MCP). This supersedes the WS2 hand-rolled `robots.txt`/llms.txt notes —
  reconcile with `docs-site/docs/public/robots.txt`.
- **Version dropdown / Versions view:** enable per the 2.x docs (ties into the
  changelog/releases). Decide whether to surface multi-version docs or just the
  version selector + changelog Versions list.
- **Code-snippet includes:** adopt `// [!include ...]` where docs currently inline
  long snippets.

### Phase 3 — Content enrichment pass

Rewrite/enrich the existing pages with the new directives where they add value:
`:::steps` for getting-started/tutorials, `::callout` for the many admonition
blocks, cards on the index, `:badge` for feature/stability tags, file-tree for
project-structure/assets pages, and a `:::benchmarks` table on the performance page
(Ultimo already has bench data). Keep every code block self-contained and compiling
(agents copy them verbatim — the WS2 rule).

## Risks & notes

- **Vercel deploy is the top risk** — verify the preview build before merging Phase 1;
  don't merge on a red docs-site Vercel check.
- **`version-sync` coupling:** the `changelog.mdx` → `::changelog` switch in Phase 2
  requires a `check-versions.sh` update, or version-sync will fail every release.
- **`ai-agents.mdx` (WS2) overlap:** parts (Copy for AI, llms.txt) become native in
  2.x; trim/reconcile that page rather than duplicating.
- Vocs 2.x maturity: vocs.dev itself dogfoods 2.8.5, so it's production-viable, but
  pin an exact version and read its changelog for any beta caveats.

## Verification (per phase)

- Phase 1: local `vocs build` + `vocs preview` all pages; Vercel preview green; `/llms.txt` present.
- Phase 2: changelog page renders GitHub releases; version dropdown works; Copy-for-AI works; `check-versions.sh` still green.
- Phase 3: every page builds; spot-check the new directives render; no broken links.

## Handoff

Parent session ran out of context budget before implementing. Recommended: execute
**Phase 1 first** in a fresh session via the ship workflow (branch → migrate →
local build + Vercel preview green → PR → merge), then Phases 2 and 3. Each phase is
a self-contained docs-site PR.
