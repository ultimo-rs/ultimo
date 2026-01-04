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

### 2. Prepare Version Update (on feature branch or separate PR)

- [ ] Update version in `Cargo.toml` (workspace.package.version)
- [ ] Update `CHANGELOG.md` with new version and date
- [ ] Update `docs-site/docs/pages/changelog.mdx` with full release notes
- [ ] Update `docs-site/docs/pages/roadmap.mdx` (move features from Next to Current)
- [ ] Update `website/package.json` version
- [ ] Run `./scripts/check-versions.sh` to verify all versions match
- [ ] Commit with message: `chore: Prepare vX.Y.Z release`
- [ ] Push changes

### 3. Merge to Main

- [ ] Merge feature/version PR to main
- [ ] Wait for CI to pass
- [ ] **Release triggers automatically!** 🎉

### 4. Automated Release (Triggers on Version Change)

**When the PR merges to `main`:**

The GitHub Actions workflow automatically detects the version change in `Cargo.toml` and:

- ✅ Verifies all versions are in sync
- ✅ Verifies on main branch
- ✅ Runs core tests
- ✅ Runs CLI tests
- ✅ Runs clippy checks
- ✅ Checks code formatting
- ✅ Dry-run publish for both crates
- ✅ Publishes `ultimo` to crates.io
- ✅ Publishes `ultimo-cli` to crates.io
- ✅ Creates git tag `vX.Y.Z`
- ✅ Pushes tag to GitHub
- ✅ Creates GitHub release with changelog notes

**Manual Override (Optional):**

If you need to manually trigger a release:

1. Go to https://github.com/ultimo-rs/ultimo/actions/workflows/release.yml
2. Click "Run workflow"
3. Select branch: `main`
4. Enter version: `vX.Y.Z` (or leave empty to auto-detect)
5. Click "Run workflow"

### 5. Update Project Board

- [ ] Go to https://github.com/orgs/ultimo-rs/projects/1
- [ ] Move completed issues from "In Review" to "Done"
- [ ] Close related issues with "Closes #XX" comment

### 6. Verify Deployments

- [ ] Check https://crates.io/crates/ultimo shows new version
- [ ] Check https://crates.io/crates/ultimo-cli shows new version
- [ ] Check https://github.com/ultimo-rs/ultimo/releases shows new release
- [ ] Check https://ultimo.dev reflects new version (auto-deploy from main)
- [ ] Check https://docs.ultimo.dev/changelog shows new release (auto-deploy from main)
- [ ] Test installation: `cargo install ultimo-cli`

## Manual Release (Fallback)

If GitHub Actions is unavailable, use the manual script:

```bash
./scripts/release.sh vX.Y.Z
```

This requires:

- CARGO_REGISTRY_TOKEN environment variable set
- Write access to GitHub repository
- Clean main branch checked out

## Version Numbering (Semantic Versioning)

Given version `MAJOR.MINOR.PATCH`:

- **MAJOR** (0.x.y): Breaking API changes
- **MINOR** (x.Y.z): New features, backwards compatible
- **PATCH** (x.y.Z): Bug fixes, backwards compatible

**Pre-1.0 Guidelines:**

- Major features → MINOR bump (0.1.0 → 0.2.0)
- Bug fixes/small features → PATCH bump (0.1.0 → 0.1.1)
- Breaking changes → Document in CHANGELOG (breaking changes are expected pre-1.0)

## How Automated Release Works

### The Workflow

```
Developer Workflow:
├─ Feature Branch
│  ├─ Update Cargo.toml (0.1.2 -> 0.2.0)
│  ├─ Update CHANGELOG.md with release notes
│  ├─ Update docs-site changelogs
│  ├─ Update website/package.json
│  ├─ Run ./scripts/check-versions.sh
│  └─ Commit: "chore: Prepare v0.2.0 release"
│
├─ Create PR & Review
│
└─ Merge to main
   └─> 🤖 GitHub Actions (Automatic!)
       ├─ Detects version change in Cargo.toml
       ├─ Verifies all versions synced
       ├─ Runs tests & checks
       ├─ Publishes to crates.io
       ├─ Creates git tag v0.2.0
       └─ Creates GitHub release
           └─> ✅ Done!
```

### Why Manual Version Bumping?

Version bumping is **intentionally manual** to ensure:

1. **Semantic versioning correctness** - Human decides if it's major/minor/patch
2. **CHANGELOG quality** - Developer writes meaningful release notes
3. **Documentation sync** - All docs updated before release
4. **Human review** - What's being released is understood and approved
5. **No surprises** - Release happens when you expect it

### What's Automated?

- ✅ **Publishing to crates.io** - No manual cargo publish needed
- ✅ **Git tagging** - Tag created and pushed automatically
- ✅ **GitHub Release** - Created with changelog excerpt
- ✅ **Version validation** - Ensures all files are in sync
- ✅ **Quality checks** - Tests, clippy, formatting before publish

### What's Manual?

- 📝 **Version decision** - You choose the version number
- 📝 **CHANGELOG writing** - You write the release notes
- 📝 **Documentation updates** - You update docs-site

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

## Setup Requirements

### One-Time Setup

**For automated releases to work, add these secrets to GitHub:**

1. Go to: https://github.com/ultimo-rs/ultimo/settings/secrets/actions
2. Add secret: `CARGO_REGISTRY_TOKEN`
   - Get token from: https://crates.io/me
   - Click "New Token" → Name it "GitHub Actions" → Copy token
   - Paste into GitHub secret

**Verify GitHub Actions permissions:**

1. Go to: https://github.com/ultimo-rs/ultimo/settings/actions
2. Under "Workflow permissions", ensure:
   - ✅ "Read and write permissions" is enabled
   - ✅ "Allow GitHub Actions to create and approve pull requests" is enabled

## Emergency Hotfix Process

If a critical bug is found in production:

1. Create hotfix branch from main: `git checkout -b hotfix/vX.Y.Z`
2. Fix the issue with tests
3. Update CHANGELOG with `[X.Y.Z]` section
4. Bump PATCH version in `Cargo.toml`
5. Sync versions across files with `./scripts/check-versions.sh`
6. Create PR, review, merge to main
7. Trigger release workflow with new version

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
