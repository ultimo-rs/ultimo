#!/bin/bash

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║    Web Framework Benchmark Runner      ║${NC}"
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo ""

# Check if oha is installed
if ! command -v oha &> /dev/null; then
    echo -e "${RED}Error: oha is not installed${NC}"
    echo "Install it with: cargo install oha"
    exit 1
fi

DURATION="30s"
CONCURRENCY="100"

echo -e "${YELLOW}Configuration:${NC}"
echo "  Duration: $DURATION"
echo "  Concurrency: $CONCURRENCY"
echo ""
echo -e "${YELLOW}Servers to test:${NC}"
echo "  • Ultimo (Rust) - http://localhost:3000"
echo "  • Axum (Rust) - http://localhost:3004"
echo "  • Hono (Node.js) - http://localhost:3001"
echo "  • Hono (Bun) - http://localhost:3002"
echo "  • FastAPI (Python) - http://localhost:3003"
echo ""

# Function to run benchmark
run_benchmark() {
    local name=$1
    local url=$2
    local endpoint=$3
    
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}Testing: $name - $endpoint${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    oha -z $DURATION -c $CONCURRENCY "$url$endpoint" 2>&1 | grep -E "(Success rate|Requests/sec|Average|Slowest|Fastest)"
    echo ""
}

echo -e "${BLUE}Starting benchmarks...${NC}"
echo ""

# Benchmark GET /api/users
echo -e "${YELLOW}═══════════════════════════════════════${NC}"
echo -e "${YELLOW}Endpoint: GET /api/users (list all)${NC}"
echo -e "${YELLOW}═══════════════════════════════════════${NC}"
echo ""

run_benchmark "Ultimo (Rust)" "http://localhost:3000" "/api/users"
run_benchmark "Axum (Rust)" "http://localhost:3004" "/api/users"
run_benchmark "Hono (Node.js)" "http://localhost:3001" "/api/users"
run_benchmark "Hono (Bun)" "http://localhost:3002" "/api/users"
run_benchmark "FastAPI (Python)" "http://localhost:3003" "/api/users"

# Benchmark GET /api/users/:id
echo -e "${YELLOW}═══════════════════════════════════════${NC}"
echo -e "${YELLOW}Endpoint: GET /api/users/1 (get by id)${NC}"
echo -e "${YELLOW}═══════════════════════════════════════${NC}"
echo ""

run_benchmark "Ultimo (Rust)" "http://localhost:3000" "/api/users/1"
run_benchmark "Axum (Rust)" "http://localhost:3004" "/api/users/1"
run_benchmark "Hono (Node.js)" "http://localhost:3001" "/api/users/1"
run_benchmark "Hono (Bun)" "http://localhost:3002" "/api/users/1"
run_benchmark "FastAPI (Python)" "http://localhost:3003" "/api/users/1"

echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         Benchmark Complete!            ║${NC}"
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
