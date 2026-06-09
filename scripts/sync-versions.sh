#!/bin/bash
# Propagate the workspace version (the single source of truth in Cargo.toml)
# to the JS/TSX sites that display or declare a version.
#
# Run this whenever the workspace version changes (e.g. while finalizing a
# release PR). `scripts/check-versions.sh` enforces that these stay in sync.
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
if [ -z "$VERSION" ]; then
  echo "❌ Could not read version from Cargo.toml"
  exit 1
fi

# Crate install snippets pin major.minor only (e.g. `ultimo = "0.4"` ≡ ^0.4),
# so patch releases never touch the docs. Derive it from the full version.
MAJMIN=$(echo "$VERSION" | sed -E 's/^([0-9]+\.[0-9]+).*/\1/')
if [ -z "$MAJMIN" ]; then
  echo "❌ Could not derive major.minor from version '$VERSION'"
  exit 1
fi

echo "Syncing all sites to version $VERSION (install snippets: $MAJMIN)"

# website/package.json + docs-site/package.json: "version": "X"
for pkg in website/package.json docs-site/package.json; do
  if [ -f "$pkg" ]; then
    sed -i.bak -E "s/(\"version\": *\")[^\"]*(\")/\1$VERSION\2/" "$pkg" && rm -f "$pkg.bak"
    echo "  updated $pkg"
  fi
done

# website hero "vX Now Available" badge
HERO="website/components/hero-section.tsx"
if [ -f "$HERO" ]; then
  sed -i.bak -E "s/v[0-9]+\.[0-9]+\.[0-9]+ Now Available/v$VERSION Now Available/" "$HERO" && rm -f "$HERO.bak"
  echo "  updated $HERO"
fi

# Crate install snippets in the README + every docs page. Two forms are
# rewritten to the major.minor version:
#   ultimo = "X.Y"
#   ultimo = { version = "X.Y", features = [...] }
# Only the `ultimo` crate is touched — sibling deps like `sqlx = { version =
# "0.8" }` and `tokio = "1.35"` are left alone because the regex is anchored on
# `ultimo`. The second form also matches commented snippets (e.g. `// ... ultimo
# = { version = "0.4" }`).
SNIPPET_FILES=(README.md docs-site/docs/pages/*.mdx)
for f in "${SNIPPET_FILES[@]}"; do
  [ -f "$f" ] || continue
  sed -i.bak -E \
    -e "s/(ultimo = \")[0-9][0-9.]*(\")/\1$MAJMIN\2/g" \
    -e "s/(ultimo = \{ version = \")[0-9][0-9.]*(\")/\1$MAJMIN\2/g" \
    "$f" && rm -f "$f.bak"
done
echo "  updated install snippets in README.md + docs-site/docs/pages/*.mdx → $MAJMIN"

echo "✅ Done. Review with: git diff"
