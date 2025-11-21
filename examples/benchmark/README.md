# Web Framework Benchmark

Performance comparison of Ultimo (Rust) vs Axum (Rust) vs Hono.js (Node.js/Bun) vs FastAPI (Python).

## Quick Start

```bash
# Run automated benchmark
./run-benchmarks.sh
```

This script will test all frameworks and display results.

## Manual Setup

### Terminal 1 - Ultimo Server (Rust)

```bash
cd ultimo-server
cargo build --release
cargo run --release
# Server runs on http://localhost:3000
```

### Terminal 2 - Axum Server (Rust)

```bash
cd axum-server
cargo build --release
cargo run --release
# Server runs on http://localhost:3004
```

### Terminal 3 - Hono (Node.js)

```bash
cd hono-server
npm install
npm start
# Server runs on http://localhost:3001
```

### Terminal 4 - Hono (Bun)

```bash
cd hono-server
bun install
bun run start:bun
# Server runs on http://localhost:3002
```

### Terminal 5 - FastAPI (Python)

```bash
cd fastapi-server
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
python server.py
# Server runs on http://localhost:3003
```

## Running Benchmarks

Using [oha](https://github.com/hatoo/oha) (fast HTTP load generator):

```bash
# Install oha
cargo install oha

# Benchmark Ultimo
oha -z 30s -c 100 http://localhost:3000/api/users
oha -z 30s -c 100 http://localhost:3000/api/users/1

# Benchmark Hono (Node)
oha -z 30s -c 100 http://localhost:3001/api/users
oha -z 30s -c 100 http://localhost:3001/api/users/1

# Benchmark Hono (Bun)
oha -z 30s -c 100 http://localhost:3002/api/users
oha -z 30s -c 100 http://localhost:3002/api/users/1
```

## Endpoints

All servers implement the same API:

- `GET /api/users` - List all users (JSON array)
- `GET /api/users/:id` - Get user by ID
- `POST /api/users` - Create new user
- `DELETE /api/users/:id` - Delete user

## Latest Results

**Test Configuration**: 30 seconds, 100 concurrent connections, Apple Silicon

| Framework | Runtime | Req/sec       | Avg Latency | vs Python      |
| --------- | ------- | ------------- | ----------- | -------------- |
| **Ultimo**  | Rust    | **152k-156k** | **0.6ms**   | **15x faster** |
| **Axum**  | Rust    | **153k-156k** | **0.6ms**   | **15x faster** |
| Hono      | Bun     | 124k-132k     | 0.8ms       | 13x faster     |
| Hono      | Node.js | 61-62k        | 1.6ms       | 6x faster      |
| FastAPI   | Python  | 9.5k-10.6k    | 9.5ms       | baseline       |

**Key Findings**:

- ✅ Ultimo matches Axum (Rust's most popular framework) in performance
- ✅ Zero performance penalty for Ultimo's auto RPC/OpenAPI generation features
- ✅ Both Rust frameworks deliver 15x better throughput than Python
- ✅ Bun performs well (85% of Rust), Node.js at 40% of Rust

See [RESULTS_WITH_AXUM.md](./RESULTS_WITH_AXUM.md) for detailed analysis and cost comparisons.

_Note: Results vary by hardware. These are from Apple Silicon (macOS)._
