# Benchmark Results: Ultimo vs Hono.js

**Date**: November 16, 2025  
**Duration**: 30 seconds per test  
**Concurrency**: 100 simultaneous connections  
**Tool**: oha (Rust HTTP load tester)

## Summary

| Framework | Runtime | Avg Req/sec  | Performance vs Ultimo | vs Node.js      |
| --------- | ------- | ------------ | ------------------- | --------------- |
| **Ultimo**  | Rust    | **~153,000** | **baseline** 🚀     | **2.3x faster** |
| Hono      | Bun     | ~107,000     | 0.70x (30% slower)  | **1.6x faster** |
| Hono      | Node.js | ~68,000      | 0.44x (56% slower)  | baseline        |

**Key Takeaways:**

- 🥇 **Ultimo is the fastest** - 2.3x faster than Node, 1.4x faster than Bun
- 🥈 **Bun is 1.6x faster than Node** - significant improvement over V8
- 🥉 **Node.js is the slowest** - but still handles 68k req/sec (respectable)

## Detailed Results

### GET /api/users (List All Users)

| Metric              | Ultimo (Rust) | Hono (Bun) | Hono (Node.js) | Winner                 |
| ------------------- | ----------- | ---------- | -------------- | ---------------------- |
| **Requests/sec**    | 152,529     | 107,138    | 67,178         | 🏆 Ultimo (2.3x vs Node) |
| **Average Latency** | 0.7ms       | 0.9ms      | 1.5ms          | 🏆 Ultimo                |
| **Fastest**         | 0.00ms      | 0.10ms     | 0.10ms         | 🏆 Ultimo                |
| **Slowest**         | 10.6ms      | 5.4ms      | 15.0ms         | 🏆 Bun                 |
| **Success Rate**    | 100%        | 100%       | 100%           | ✅ All                 |

### GET /api/users/1 (Get by ID)

| Metric              | Ultimo (Rust) | Hono (Bun) | Hono (Node.js) | Winner                 |
| ------------------- | ----------- | ---------- | -------------- | ---------------------- |
| **Requests/sec**    | 154,265     | 106,741    | 68,401         | 🏆 Ultimo (2.3x vs Node) |
| **Average Latency** | 0.6ms       | 0.9ms      | 1.5ms          | 🏆 Ultimo                |
| **Fastest**         | 0.00ms      | 0.10ms     | 0.10ms         | 🏆 Ultimo                |
| **Slowest**         | 12.1ms      | 5.8ms      | 7.1ms          | 🏆 Bun                 |
| **Success Rate**    | 100%        | 100%       | 100%           | ✅ All                 |

## Key Insights

## Key Insights

### 🚀 Performance Rankings

1. **Ultimo (Rust)**: 153k req/sec - Clear winner
2. **Hono (Bun)**: 107k req/sec - 30% slower than Ultimo, 60% faster than Node
3. **Hono (Node.js)**: 68k req/sec - Baseline JavaScript performance

### 📊 Performance Advantages

- **Ultimo vs Node**: 2.3x faster throughput, 2.5x lower latency
- **Ultimo vs Bun**: 1.4x faster throughput, 1.5x lower latency
- **Bun vs Node**: 1.6x faster throughput, 1.7x lower latency

### 🎯 Why Ultimo is Faster

1. **Zero-Copy I/O**: Rust's ownership system enables efficient memory usage
2. **Native Compilation**: No JIT warmup, immediate peak performance
3. **Tokio Runtime**: One of the fastest async runtimes available
4. **No Garbage Collection**: Predictable, low-latency performance
5. **Optimized Release Build**: Full compiler optimizations (LTO, codegen-units=1)

### 📊 Real-World Impact

**Throughput per minute:**

- **Ultimo (Rust)**: ~9.2M requests/minute
- **Hono (Bun)**: ~6.4M requests/minute (30% less)
- **Hono (Node)**: ~4.1M requests/minute (56% less)

**Cost Savings:**

- Ultimo needs **44% of the servers** compared to Node.js
- Ultimo needs **70% of the servers** compared to Bun
- Bun needs **63% of the servers** compared to Node.js

**Example**: To handle 10M req/min:

- Node.js: ~3 servers needed
- Bun: ~2 servers needed
- Ultimo: ~2 servers needed (with 8% headroom)

## Test Environment

- **Hardware**: MacBook (Apple Silicon/Intel - specs not specified)
- **OS**: macOS
- **Rust**: v1.84.0
- **Node.js**: v22.18.0
- **Bun**: v1.3.0
- **Ultimo Build**: Release mode with LTO and single codegen unit
- **Hono**: Standard production setup

## Reproducing Results

```bash
# Start servers
cd examples/benchmark

# Terminal 1: Ultimo
cd ultimo-server && cargo run --release

# Terminal 2: Hono (Node.js)
cd hono-server && npm start

# Terminal 3: Hono (Bun)
cd hono-server && bun run start:bun

# Terminal 4: Run benchmark
./run-benchmarks.sh
```

## Notes

- All three frameworks showed 100% success rates
- Bun (v1.3.0) shows impressive improvement over Node.js V8 engine
- Results may vary based on hardware and system load
- This is a synthetic benchmark; real-world performance depends on application complexity
- Ultimo used release build optimizations (LTO, single codegen unit)

## Conclusion

**Ultimo is the clear performance winner**, delivering:

- **2.3x the throughput of Node.js**
- **1.4x the throughput of Bun**
- **Sub-millisecond average latency** across all endpoints

**Bun shows impressive gains** over Node.js (1.6x faster), proving that JavaScript runtime innovation matters. However, **Rust's zero-cost abstractions and native compilation still provide a significant edge** for high-performance APIs.

### When to Choose What?

- **Choose Ultimo** when: Maximum performance, lowest latency, predictable behavior, cost optimization
- **Choose Bun** when: Need JavaScript ecosystem, faster than Node, good balance of speed/DX
- **Choose Node.js** when: Mature ecosystem, team familiarity, "fast enough" performance

For high-throughput production APIs, **Ultimo delivers the best performance-per-dollar**.
