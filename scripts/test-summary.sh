#!/bin/bash
# Test summary script - show test statistics

echo "📊 Ultimo Framework Test Summary"
echo "=================================="
echo ""

# Count tests
TOTAL_TESTS=$(cargo test --lib -- --list 2>/dev/null | grep -E "^test " | wc -l | tr -d ' ')
echo "Total Tests: $TOTAL_TESTS"

# Run tests
echo ""
echo "Running tests..."
cargo test --lib --quiet

# Count by module
echo ""
echo "Tests by module:"
echo "----------------"
cargo test --lib -- --list 2>/dev/null | grep -E "^test " | awk -F'::' '{print $1}' | sort | uniq -c | sort -rn

echo ""
echo "✅ Test run complete!"
