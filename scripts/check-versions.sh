#!/bin/bash
# Check that all version numbers are in sync with the workspace version
# (Cargo.toml is the single source of truth). Run `scripts/sync-versions.sh`
# to fix the site versions. This is enforced as a CI gate.

set -e

WORKSPACE_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
# Install snippets pin major.minor only (e.g. `ultimo = "0.4"`), so they are
# compared against the workspace major.minor, not the full version.
WORKSPACE_MAJMIN=$(echo "$WORKSPACE_VERSION" | sed -E 's/^([0-9]+\.[0-9]+).*/\1/')
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

# ── Crate install snippets (README + every docs page) ─────────────────
# These pin major.minor (`ultimo = "0.4"` / `ultimo = { version = "0.4", … }`)
# and previously drifted freely (0.3 / 0.4 / 0.5 all coexisted) because nothing
# checked them. Extract every `ultimo` version string and assert major.minor.
echo "Install snippets (expect major.minor $WORKSPACE_MAJMIN):"
SNIPPET_MISMATCH=0
while IFS= read -r line; do
  file=${line%%:*}
  rest=${line#*:}
  # Pull the version string out of either snippet form.
  found=$(echo "$rest" | grep -oE 'ultimo = (\{ version = )?"[0-9][0-9.]*"' | grep -oE '"[0-9][0-9.]*"' | tr -d '"' | head -1)
  [ -z "$found" ] && continue
  found_majmin=$(echo "$found" | sed -E 's/^([0-9]+\.[0-9]+).*/\1/')
  if [ "$found_majmin" != "$WORKSPACE_MAJMIN" ]; then
    echo "  ❌ $file: ultimo = \"$found\" (major.minor $found_majmin ≠ $WORKSPACE_MAJMIN)"
    SNIPPET_MISMATCH=1
    MISMATCH=1
  fi
done < <(grep -rnE 'ultimo = (\{ version = )?"[0-9]' README.md docs-site/docs/pages/*.mdx 2>/dev/null)
if [ $SNIPPET_MISMATCH -eq 0 ]; then
  echo "  ✅ all install snippets pin $WORKSPACE_MAJMIN"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ $MISMATCH -eq 0 ]; then
  echo "✅ All versions match: $WORKSPACE_VERSION"
  exit 0
else
  echo ""
  echo "Run scripts/sync-versions.sh to sync the sites + install snippets, and update the changelogs."
  exit 1
fi
