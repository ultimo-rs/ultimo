#!/bin/bash
set -e

echo "🚀 Installing Ultimo CLI..."
echo ""

# Build the CLI
cargo build --release --manifest-path ultimo-cli/Cargo.toml

# Get the binary path
BINARY="ultimo-cli/target/release/ultimo"

if [ ! -f "$BINARY" ]; then
    echo "❌ Build failed - binary not found"
    exit 1
fi

# Install to cargo bin directory
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
mkdir -p "$CARGO_BIN"

cp "$BINARY" "$CARGO_BIN/ultimo"
chmod +x "$CARGO_BIN/ultimo"

echo "✅ Ultimo CLI installed successfully!"
echo ""
echo "📍 Installed to: $CARGO_BIN/ultimo"
echo ""
echo "Try running:"
echo "  ultimo --help"
echo ""
