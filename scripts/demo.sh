#!/bin/bash
# Ultimo Framework Demo Script

set -e

echo "🚀 Ultimo Framework Demo"
echo "====================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}Step 1: Building Ultimo CLI${NC}"
cargo build --release --manifest-path ultimo-cli/Cargo.toml
echo ""

echo -e "${BLUE}Step 2: Starting Backend Server${NC}"
echo "Backend will auto-generate TypeScript client..."
cd examples/react-backend

# Kill any existing process on port 3000
lsof -ti:3000 | xargs kill -9 2>/dev/null || true

# Start backend in background
cargo run --release > /tmp/ultimo-backend.log 2>&1 &
BACKEND_PID=$!
echo "Backend PID: $BACKEND_PID"

# Wait for server to start
sleep 3

cd ../..
echo ""

echo -e "${BLUE}Step 3: Testing RPC Endpoint${NC}"
echo "Calling listUsers RPC..."
curl -s -X POST http://localhost:3000/rpc \
  -H "Content-Type: application/json" \
  -d '{"method":"listUsers","params":null}' | jq '.'
echo ""

echo -e "${BLUE}Step 4: Verifying Generated TypeScript Client${NC}"
echo "Client location: examples/react-app/src/lib/ultimo-client.ts"
echo ""
echo "First 15 lines:"
head -15 examples/react-app/src/lib/ultimo-client.ts
echo "..."
echo ""

echo -e "${BLUE}Step 5: Using CLI to Generate Client${NC}"
./ultimo-cli/target/release/ultimo generate \
  --project examples/react-backend \
  --output /tmp/demo-client.ts
echo ""

echo -e "${GREEN}✅ Demo Complete!${NC}"
echo ""
echo "Backend is running on http://localhost:3000"
echo "Backend PID: $BACKEND_PID"
echo ""
echo "Try these commands:"
echo "  # List users via RPC"
echo "  curl -X POST http://localhost:3000/rpc -H 'Content-Type: application/json' -d '{\"method\":\"listUsers\",\"params\":null}'"
echo ""
echo "  # Create user via RPC"
echo "  curl -X POST http://localhost:3000/rpc -H 'Content-Type: application/json' -d '{\"method\":\"createUser\",\"params\":{\"name\":\"Charlie\",\"email\":\"charlie@example.com\"}}'"
echo ""
echo "  # List users via REST"
echo "  curl http://localhost:3000/users"
echo ""
echo "To stop the backend:"
echo "  kill $BACKEND_PID"
echo ""
