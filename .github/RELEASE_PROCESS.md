# Release Process

This document outlines the release process for Ultimo to ensure version consistency across all platforms.

## Version Sources (Single Source of Truth)

**Primary:** `Cargo.toml` (workspace.package.version)

**Derived from primary:**
- `ultimo/Cargo.toml` (via workspace)
- `ultimo-cli/Cargo.toml` (via workspace)
- All example `Cargo.toml` files (via workspace)

**Must be manually synced:**
- `CHANGELOG.md`
- `docs-site/docs/pages/changelog.mdx`
- `docs-site/docs/pages/roadmap.mdx`
- `website/package.json`

## Release Checklist

### 1. Pre-Release (on feature branch)

- [ ] All features implemented and tested
- [ ] All tests passing (`cargo test`)
- [ ] Zero clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Examples updated and tested
- [ ] PR created and reviewed

### 2. Merge & Version Update (on main branch)

- [ ] Merge feature PR to main
- [ ] Update version in `Cargo.toml` (workspace.package.version)
- [ ] Update `CHANGELOG.md` with new version and date
- [ ] Update `docs-site/docs/pages/changelog.mdx` with full release notes
- [ ] Update `docs-site/docs/pages/roadmap.mdx` (move features from Next to Current)
- [ ] Update `website/package.json` version
- [ ] Commit with message: `chore: Bump version to vX.Y.Z`
- [ ] Push to main

### 3. Release to crates.io

```bash
./scripts/release.sh vX.Y.Z
```

This script will:
- Verify on main branch
- Check clean working directory
- Run core tests
- Run CLI tests
- Run clippy
- Check formatting
- Dry-run publish
- Publish `ultimo` to crates.io
- Wait 10 seconds
- Publish `ultimo-cli` to crates.io
- Create git tag
- Push tag to GitHub

### 4. GitHub Release

- [ ] Go to https://github.com/ultimo-rs/ultimo/releases/new
- [ ] Select the tag created by release script
- [ ] Title: `vX.Y.Z - Feature Name`
- [ ] Copy content from `CHANGELOG.md` for this version
- [ ] Mark as "latest release"
- [ ] Publish release

### 5. Update Project Board

- [ ] Go to https://github.com/orgs/ultimo-rs/projects/1
- [ ] Move completed issues from "In Review" to "Done"
- [ ] Close completed issues

### 6. Verify Deployments

- [ ] Check https://crates.io/crates/ultimo shows new version
- [ ] Check https://crates.io/crates/ultimo-cli shows new version
- [ ] Check https://ultimo.dev reflects new version (auto-deploy from main)
- [ ] Check https://docs.ultimo.dev/changelog shows new release (auto-deploy from main)
- [ ] Test installation: `cargo install ultimo-cli`

## Version Numbering (Semantic Versioning)

Given version `MAJOR.MINOR.PATCH`:

- **MAJOR** (0.x.y): Breaking API changes
- **MINOR** (x.Y.z): New features, backwards compatible
- **PATCH** (x.y.Z): Bug fixes, backwards compatible

**Pre-1.0 Guidelines:**
- Major features → MINOR bump (0.1.0 → 0.2.0)
- Bug fixes/small features → PATCH bump (0.1.0 → 0.1.1)
- Breaking changes → Document in CHANGELOG (breaking changes are expected pre-1.0)

## Version Sync Verification Script

```bash
#!/bin/bash
# scripts/check-versions.sh

WORKSPACE_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
WEBSITE_VERSION=$(grep '"version":' website/package.json | sed 's/.*"\(.*\)".*/\1/')
CHANGELOG_VERSION=$(grep -m 1 '## \[' CHANGELOG.md | sed 's/.*\[\(.*\)\].*/\1/')

echo "Workspace (Cargo.toml):        $WORKSPACE_VERSION"
echo "Website (package.json):        $WEBSITE_VERSION"
echo "Changelog (CHANGELOG.md):      $CHANGELOG_VERSION"

if [ "$WORKSPACE_VERSION" = "$WEBSITE_VERSION" ] && [ "$WORKSPACE_VERSION" = "$CHANGELOG_VERSION" ]; then
  echo "✅ All versions match!"
  exit 0
else
  echo "❌ Version mismatch detected!"
  exit 1
fi
```

Add to `.git/hooks/pre-commit`:
```bash
#!/bin/bash
./scripts/check-versions.sh || exit 1
```

## Emergency Hotfix Process

If a critical bug is found in production:

1. Create hotfix branch from main: `git checkout -b hotfix/vX.Y.Z`
2. Fix the issue with tests
3. Update CHANGELOG with `[X.Y.Z]` section
4. Bump PATCH version in `Cargo.toml`
5. Sync versions across files
6. Create PR, review, merge
7. Follow release process from step 3

## Rolling Back a Release

If a release has critical issues:

1. **On crates.io**: Cannot unpublish (except within 72 hours), must publish a new version
2. **On GitHub**: Delete the release and tag
3. **Action**: Publish a new fixed version immediately

## Documentation Deployments

- **Website** (ultimo.dev): Auto-deploys from `main` branch
- **Docs** (docs.ultimo.dev): Auto-deploys from `main` branch

Changes to `website/` or `docs-site/` will deploy automatically on merge to main.

## Communication

After release:

- [ ] Announce on Discord (if exists)
- [ ] Tweet release notes (if active)
- [ ] Update README.md if needed
- [ ] Close milestone if using GitHub milestones

## Post-Release

- [ ] Monitor GitHub issues for bug reports
- [ ] Monitor crates.io download stats
- [ ] Plan next version features
- [ ] Update roadmap if needed
