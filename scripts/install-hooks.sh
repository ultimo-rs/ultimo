#!/bin/sh
# Install git hooks for Ultimo development

HOOKS_DIR=".git/hooks"
GITHOOKS_DIR=".githooks"

echo "📦 Installing git hooks..."

# Copy pre-commit hook
if [ -f "$GITHOOKS_DIR/pre-commit" ]; then
    cp "$GITHOOKS_DIR/pre-commit" "$HOOKS_DIR/pre-commit"
    chmod +x "$HOOKS_DIR/pre-commit"
    echo "  ✅ Installed pre-commit hook"
fi

# Copy pre-push hook
if [ -f "$GITHOOKS_DIR/pre-push" ]; then
    cp "$GITHOOKS_DIR/pre-push" "$HOOKS_DIR/pre-push"
    chmod +x "$HOOKS_DIR/pre-push"
    echo "  ✅ Installed pre-push hook"
fi

echo ""
echo "✨ Git hooks installed successfully!"
echo ""
echo "Hooks configured:"
echo "  • pre-commit: Checks code formatting"
echo "  • pre-push: Runs tests, clippy, and coverage checks"
