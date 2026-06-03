#!/bin/bash
# Check that all version numbers are in sync with the workspace version
# (Cargo.toml is the single source of truth). Run `scripts/sync-versions.sh`
# to fix the site versions. This is enforced as a CI gate.

set -e

WORKSPACE_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
WEBSITE_VERSION=$(grep '"version":' website/package.json | head -1 | sed 's/.*"\(.*\)".*/\1/')
DOCS_SITE_PKG_VERSION=$(grep '"version":' docs-site/package.json | head -1 | sed 's/.*"\(.*\)".*/\1/')
HERO_VERSION=$(grep -oE 'v[0-9]+\.[0-9]+\.[0-9]+ Now Available' website/components/hero-section.tsx | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
CHANGELOG_VERSION=$(grep '## \[' CHANGELOG.md | grep -v Unreleased | head -1 | sed 's/.*\[\(.*\)\].*/\1/')
DOCS_CHANGELOG_VERSION=$(grep '## \[' docs-site/docs/pages/changelog.mdx | grep -v Unreleased | head -1 | sed 's/.*\[\(.*\)\].*/\1/')

echo "📦 Version Check (source of truth: Cargo.toml)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Workspace (Cargo.toml):               $WORKSPACE_VERSION"
echo "Website (website/package.json):       $WEBSITE_VERSION"
echo "Docs-site (docs-site/package.json):   $DOCS_SITE_PKG_VERSION"
echo "Website hero badge (hero-section):    $HERO_VERSION"
echo "Changelog (CHANGELOG.md):             $CHANGELOG_VERSION"
echo "Docs Changelog (changelog.mdx):       $DOCS_CHANGELOG_VERSION"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

MISMATCH=0
check() {
  if [ "$WORKSPACE_VERSION" != "$2" ]; then
    echo "❌ $1 ($2) doesn't match workspace ($WORKSPACE_VERSION)"
    MISMATCH=1
  fi
}

check "Website version" "$WEBSITE_VERSION"
check "Docs-site package version" "$DOCS_SITE_PKG_VERSION"
check "Website hero badge" "$HERO_VERSION"
check "Changelog version" "$CHANGELOG_VERSION"
check "Docs changelog version" "$DOCS_CHANGELOG_VERSION"

if [ $MISMATCH -eq 0 ]; then
  echo "✅ All versions match: $WORKSPACE_VERSION"
  exit 0
else
  echo ""
  echo "Run scripts/sync-versions.sh to sync the sites, and update the changelogs."
  exit 1
fi
