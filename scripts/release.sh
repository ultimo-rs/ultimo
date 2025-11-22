#!/bin/bash
set -e

VERSION=$1
if [ -z "$VERSION" ]; then
  echo "❌ Usage: ./scripts/release.sh v0.1.0"
  exit 1
fi

echo "🚀 Preparing to release $VERSION"
echo ""

# Verify we're on main branch
BRANCH=$(git branch --show-current)
if [ "$BRANCH" != "main" ]; then
  echo "❌ Must be on main branch (currently on $BRANCH)"
  exit 1
fi

# Verify clean working directory
if [ -n "$(git status --porcelain)" ]; then
  echo "❌ Working directory is not clean. Commit or stash changes first."
  git status --short
  exit 1
fi

echo "✅ Working directory is clean"
echo ""

# Run tests
echo "🧪 Running tests..."
cargo test --all-features
echo "✅ All tests passed"
echo ""

# Run clippy
echo "📎 Running clippy..."
cargo clippy --all-features -- -D warnings
echo "✅ Clippy passed"
echo ""

# Check formatting
echo "✨ Checking formatting..."
cargo fmt --check
echo "✅ Formatting is correct"
echo ""

# Dry run publish
echo "🔍 Running dry-run publish for ultimo..."
cargo publish -p ultimo --dry-run
echo "✅ Dry-run successful for ultimo"
echo ""

echo "🔍 Running dry-run publish for ultimo-cli..."
cargo publish -p ultimo-cli --dry-run --allow-dirty
echo "✅ Dry-run successful for ultimo-cli"
echo ""

# Confirm before publishing
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Ready to publish $VERSION to crates.io"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
read -p "Continue? (y/N) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  echo "❌ Aborted"
  exit 1
fi

# Publish ultimo
echo ""
echo "📦 Publishing ultimo to crates.io..."
cargo publish -p ultimo
echo "✅ Published ultimo"
echo ""

# Wait for crates.io to update
echo "⏳ Waiting 10 seconds for crates.io to update..."
sleep 10

# Publish ultimo-cli
echo "📦 Publishing ultimo-cli to crates.io..."
cargo publish -p ultimo-cli
echo "✅ Published ultimo-cli"
echo ""

# Create and push git tag
echo "🏷️  Creating git tag $VERSION..."
git tag -a "$VERSION" -m "Release $VERSION"
git push origin "$VERSION"
echo "✅ Tagged and pushed $VERSION"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Successfully released $VERSION!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📝 Next steps:"
echo "  1. Create GitHub release: https://github.com/ultimo-rs/ultimo/releases/new?tag=$VERSION"
echo "  2. Verify on crates.io: https://crates.io/crates/ultimo"
echo "  3. Check docs.rs: https://docs.rs/ultimo"
echo ""
