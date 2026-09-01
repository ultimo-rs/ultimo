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

# CLAUDE.md: Current version: **X.X.X**
if [ -f CLAUDE.md ]; then
  sed -i.bak -E "s/(Current version: \*\*)[0-9]+\.[0-9]+\.[0-9]+(\*\*)/\1$VERSION\2/" CLAUDE.md && rm -f CLAUDE.md.bak
  echo "  updated CLAUDE.md"
fi

# AGENTS.md: Current version: **X.X.X**
if [ -f AGENTS.md ]; then
  sed -i.bak -E "s/(Current version: \*\*)[0-9]+\.[0-9]+\.[0-9]+(\*\*)/\1$VERSION\2/" AGENTS.md && rm -f AGENTS.md.bak
  echo "  updated AGENTS.md"
fi

# Blog comparison post: "Current version" table cell
BLOG_COMPARE="website/content/posts/ultimo-vs-axum-comparison.mdx"
if [ -f "$BLOG_COMPARE" ]; then
  sed -i.bak -E "s/(\*\*Current version\*\* *\| *\[)[0-9]+\.[0-9]+\.[0-9]+/\1$VERSION/" "$BLOG_COMPARE" && rm -f "$BLOG_COMPARE.bak"
  echo "  updated $BLOG_COMPARE"
fi

# Roadmap: move (Current) marker to the matching version header
ROADMAP="docs-site/docs/pages/roadmap.mdx"
if [ -f "$ROADMAP" ]; then
  # Strip (Current) from all version headers
  sed -i.bak -E 's/(### v[0-9]+\.[0-9]+\.[0-9]+) \(Current\)/\1/' "$ROADMAP" && rm -f "$ROADMAP.bak"
  # Add (Current) to the header matching the workspace version
  sed -i.bak -E "s/### v${VERSION}$/### v${VERSION} (Current)/" "$ROADMAP"
  sed -i.bak -E "s/### v${VERSION} —/### v${VERSION} (Current) —/" "$ROADMAP" && rm -f "$ROADMAP.bak"
  echo "  updated $ROADMAP — (Current) → v$VERSION"
fi

# Promote the hand-maintained root changelogs (CHANGELOG.md + the docs-site
# changelog page) to the release version. release-plz maintains the per-crate
# `ultimo/CHANGELOG.md` from Conventional Commits but never touches these two
# files, yet `check-versions.sh` requires their newest released heading to match
# the workspace version. Mirror the newest per-crate section into them so a
# release PR passes the version-sync gate without hand editing.
# A release may bump the library, the CLI, or both, so mirror the `## [VERSION]`
# section from whichever per-crate changelog actually carries it (prefer the
# library's when both do).
pick_crate_changelog() {
  local f
  for f in ultimo/CHANGELOG.md ultimo-cli/CHANGELOG.md; do
    [ -f "$f" ] && grep -q "^## \[$VERSION\]" "$f" && {
      echo "$f"
      return
    }
  done
}

promote_changelog() {
  local file="$1"
  [ -f "$file" ] || return 0
  # Idempotent: skip if this version is already the newest released entry.
  if grep -q "^## \[$VERSION\]" "$file"; then
    return 0
  fi

  local src
  src=$(pick_crate_changelog)
  if [ -z "$src" ]; then
    echo "  ⚠️  no per-crate changelog has a v$VERSION section; skipping $file"
    return 0
  fi

  # Extract that exact `## [VERSION]` section (up to the next `## [` heading) and
  # rewrite the heading `## [x.y.z](compare-url) - DATE` → `## [x.y.z] - DATE`.
  local block
  block=$(awk -v ver="$VERSION" '
    index($0, "## [" ver "]") == 1 { grab = 1; print; next }
    grab && /^## \[/ { grab = 0 }
    grab { print }
  ' "$src" | sed -E "s/^## \[$VERSION\]\([^)]*\)/## [$VERSION]/")

  if [ -z "$block" ] || ! printf '%s' "$block" | grep -q "^## \[$VERSION\]"; then
    echo "  ⚠️  could not extract v$VERSION section from $src; skipping $file"
    return 0
  fi

  # Insert the block right after the `## [Unreleased]` heading.
  local tmp
  tmp=$(mktemp)
  printf '%s\n' "$block" > "$tmp"
  awk -v blockfile="$tmp" '
    /^## \[Unreleased\]/ && !done {
      print
      print ""
      while ((getline line < blockfile) > 0) print line
      done = 1
      next
    }
    { print }
  ' "$file" > "$file.new" && mv "$file.new" "$file"
  rm -f "$tmp"
  echo "  promoted $file — [Unreleased] → [$VERSION]"
}

promote_changelog CHANGELOG.md
promote_changelog docs-site/docs/pages/changelog.mdx

echo "✅ Done. Review with: git diff"
