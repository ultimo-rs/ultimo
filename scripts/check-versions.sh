#!/bin/bash
# Check that all version numbers are in sync

set -e

WORKSPACE_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
WEBSITE_VERSION=$(grep '"version":' website/package.json | sed 's/.*"\(.*\)".*/\1/')
CHANGELOG_VERSION=$(grep '## \[' CHANGELOG.md | grep -v Unreleased | head -1 | sed 's/.*\[\(.*\)\].*/\1/')
DOCS_CHANGELOG_VERSION=$(grep '## \[' docs-site/docs/pages/changelog.mdx | grep -v Unreleased | head -1 | sed 's/.*\[\(.*\)\].*/\1/')

echo "📦 Version Check"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Workspace (Cargo.toml):              $WORKSPACE_VERSION"
echo "Website (website/package.json):      $WEBSITE_VERSION"
echo "Changelog (CHANGELOG.md):            $CHANGELOG_VERSION"
echo "Docs Changelog (changelog.mdx):      $DOCS_CHANGELOG_VERSION"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

MISMATCH=0

if [ "$WORKSPACE_VERSION" != "$WEBSITE_VERSION" ]; then
  echo "❌ Website version ($WEBSITE_VERSION) doesn't match workspace ($WORKSPACE_VERSION)"
  MISMATCH=1
fi

if [ "$WORKSPACE_VERSION" != "$CHANGELOG_VERSION" ]; then
  echo "❌ Changelog version ($CHANGELOG_VERSION) doesn't match workspace ($WORKSPACE_VERSION)"
  MISMATCH=1
fi

if [ "$WORKSPACE_VERSION" != "$DOCS_CHANGELOG_VERSION" ]; then
  echo "❌ Docs changelog version ($DOCS_CHANGELOG_VERSION) doesn't match workspace ($WORKSPACE_VERSION)"
  MISMATCH=1
fi

if [ $MISMATCH -eq 0 ]; then
  echo "✅ All versions match: $WORKSPACE_VERSION"
  exit 0
else
  echo ""
  echo "Please sync all versions to match Cargo.toml"
  exit 1
fi
