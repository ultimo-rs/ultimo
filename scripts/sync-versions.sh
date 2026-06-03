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
echo "Syncing all sites to version $VERSION"

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

echo "✅ Done. Review with: git diff"
