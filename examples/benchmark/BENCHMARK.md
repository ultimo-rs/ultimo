# Benchmark: Ultimo vs Hono.js

This benchmark compares the performance of **Ultimo** (Rust web framework) against **Hono.js** running on both Node.js and Bun.

## Quick Start

### 1. Start All Servers

**Terminal 1 - Ultimo Server:**

```bash
cd ultimo-server
cargo run --release
```

**Terminal 2 - Hono (Node.js):**

```bash
cd hono-server
npm install
npm start
```

**Terminal 3 - Hono (Bun):**

```bash
cd hono-server
bun install
bun run start:bun
```

### 2. Run Benchmarks

**Terminal 4:**

```bash
# Install oha (HTTP load generator)
cargo install oha

# Run the benchmark suite
./run-benchmarks.sh
```

## What's Being Tested

Identical REST API with in-memory storage:

- `GET /api/users` - List all users (3 users)
- `GET /api/users/:id` - Get user by ID
- `POST /api/users` - Create new user
- `DELETE /api/users/:id` - Delete user

## Benchmark Configuration

- **Duration**: 30 seconds per test
- **Concurrency**: 100 concurrent connections
- **Tool**: [oha](https://github.com/hatoo/oha) - Rust-based HTTP load tester

## Expected Results

Based on typical performance characteristics:

| Framework | Runtime | Req/s  | Avg Latency |
| --------- | ------- | ------ | ----------- |
| Ultimo      | Rust    | ~100k+ | <1ms        |
| Hono      | Bun     | ~50k+  | 1-2ms       |
| Hono      | Node.js | ~20k+  | 3-5ms       |

_Actual results vary by hardware and system load_

## Manual Testing

Test individual endpoints:

```bash
# Ultimo (port 3000)
oha -z 30s -c 100 http://localhost:3000/api/users

# Hono Node (port 3001)
oha -z 30s -c 100 http://localhost:3001/api/users

# Hono Bun (port 3002)
oha -z 30s -c 100 http://localhost:3002/api/users
```

## Why Ultimo is Faster

1. **Zero-copy I/O**: Rust's ownership system enables efficient memory usage
2. **Native Compilation**: No JIT warmup, immediate peak performance
3. **Async Runtime**: Tokio provides one of the fastest async runtimes available
4. **No Garbage Collection**: Predictable, low-latency performance

## Architecture

```
benchmark/
├── ultimo-server/        # Rust implementation
│   ├── Cargo.toml
│   └── src/main.rs
├── hono-server/        # JavaScript implementation
│   ├── package.json
│   ├── server.js       # Node.js version
│   └── server-bun.js   # Bun version
├── run-benchmarks.sh   # Automated benchmark runner
└── README.md
```
