#!/bin/bash
# Coverage script for Ultimo framework using our own coverage tool
# Minimum coverage threshold: 60%

set -e

echo "🧪 Running tests with coverage analysis..."
echo "Using Ultimo's own coverage tool (no external dependencies)"
echo ""

# Run our coverage tool
cargo run --release -p ultimo-coverage || {
    echo "❌ Coverage analysis failed"
    exit 1
}

echo ""
echo "📊 Coverage report generated!"
echo "Open: target/coverage/html/index.html"
echo ""
