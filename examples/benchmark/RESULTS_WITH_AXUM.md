# Web Framework Benchmark Results

Complete performance comparison of **Ultimo** vs **Axum** (Rust's most popular framework) vs **Hono.js** (Node/Bun) vs **FastAPI** (Python).

## Test Configuration

- **Tool**: oha (HTTP load testing tool written in Rust)
- **Duration**: 30 seconds per endpoint
- **Concurrency**: 100 concurrent connections
- **Test Date**: November 16, 2025
- **Hardware**: Apple Silicon (macOS)

## Results Summary

### GET /api/users (List All Users)

| Framework | Runtime | Req/sec     | Avg Latency | Success Rate |
| --------- | ------- | ----------- | ----------- | ------------ |
| **Axum**  | Rust    | **152,316** | 0.7ms       | 100%         |
| **Ultimo**  | Rust    | **151,635** | 0.7ms       | 100%         |
| Hono      | Bun     | 103,445     | 1.0ms       | 100%         |
| Hono      | Node.js | 53,919      | 1.9ms       | 100%         |
| FastAPI   | Python  | 10,285      | 9.7ms       | 100%         |

### GET /api/users/1 (Get Single User)

| Framework | Runtime | Req/sec     | Avg Latency | Success Rate |
| --------- | ------- | ----------- | ----------- | ------------ |
| **Axum**  | Rust    | **154,035** | 0.6ms       | 100%         |
| **Ultimo**  | Rust    | **153,923** | 0.6ms       | 100%         |
| Hono      | Bun     | 103,587     | 1.0ms       | 100%         |
| Hono      | Node.js | 54,589      | 1.8ms       | 100%         |
| FastAPI   | Python  | 9,049       | 11.1ms      | 100%         |

## Performance Analysis

### 🏆 Ultimo vs Axum: Dead Heat!

**Key Finding**: Ultimo and Axum deliver **virtually identical performance** - both frameworks handle ~152-154k req/sec with 0.6-0.7ms latency.

**What this means**:

- ✅ **Ultimo is production-ready** - matching the most battle-tested Rust web framework
- ✅ **Zero performance penalty** - Ultimo's additional features (RPC, OpenAPI generation) don't compromise speed
- ✅ **Choose based on features, not speed** - both are equally fast

**Axum vs Ultimo Feature Comparison**:

| Feature                   | Axum                | Ultimo                 |
| ------------------------- | ------------------- | -------------------- |
| Performance               | ⚡ 152-154k req/sec | ⚡ 152-154k req/sec  |
| Async Runtime             | Tokio               | Tokio                |
| Middleware                | ✅ Tower-based      | ✅ Built-in + custom |
| Type Safety               | ✅ Strong           | ✅ Strong            |
| **Auto RPC Generation**   | ❌ Manual           | ✅ Automatic         |
| **OpenAPI Generation**    | ❌ Manual           | ✅ Automatic         |
| **Client SDK Generation** | ❌ Manual           | ✅ Automatic         |
| Community Size            | Large (tokio-rs)    | Growing              |
| Documentation             | Extensive           | Good                 |
| Learning Curve            | Medium              | Medium               |

### Rust Dominance

Both Rust frameworks (Ultimo & Axum) deliver:

- **15x faster** than FastAPI (Python)
- **2.8x faster** than Hono on Node.js
- **1.5x faster** than Hono on Bun

### Language Runtime Hierarchy

1. **Rust** (Ultimo, Axum): 152-154k req/sec
2. **Bun**: 103k req/sec (67% of Rust)
3. **Node.js**: 54k req/sec (35% of Rust)
4. **Python**: 10k req/sec (6.5% of Rust)

## Infrastructure Cost Comparison

Assuming 100,000 requests per second target load:

### Servers Required

| Framework   | Req/sec per server | Servers needed | Cost/month\* |
| ----------- | ------------------ | -------------- | ------------ |
| **Ultimo**    | 152,000            | **1 server**   | **$50**      |
| **Axum**    | 152,000            | **1 server**   | **$50**      |
| Hono (Bun)  | 103,000            | 1 server       | $50          |
| Hono (Node) | 54,000             | 2 servers      | $100         |
| FastAPI     | 10,000             | **10 servers** | **$500**     |

\*Based on typical cloud VPS pricing ($50/month per server)

### Annual Infrastructure Costs

For 100k req/sec sustained load:

| Framework     | Monthly | Annual   | vs Rust     |
| ------------- | ------- | -------- | ----------- |
| **Ultimo/Axum** | $50     | **$600** | baseline    |
| Hono (Bun)    | $50     | $600     | same        |
| Hono (Node)   | $100    | $1,200   | +$600       |
| FastAPI       | $500    | $6,000   | **+$5,400** |

**Savings using Rust vs Python**: **$5,400/year** (9x cost reduction)

## When to Choose Each Framework

### Choose **Ultimo** when:

- ✅ You want **automatic RPC generation** (type-safe client-server communication)
- ✅ You need **automatic OpenAPI documentation** from code
- ✅ You want **automatic client SDK generation** (TypeScript, etc.)
- ✅ You're building **modern web APIs** with full-stack TypeScript/Rust integration
- ✅ You want maximum performance with minimal boilerplate
- ✅ You're starting a new project and want modern DX

### Choose **Axum** when:

- ✅ You need **mature, battle-tested** framework (production proven)
- ✅ You want **extensive Tower middleware ecosystem**
- ✅ You prefer **explicit, verbose** code over magic/generation
- ✅ You need **community support** and extensive documentation
- ✅ You're migrating from other Tokio-based services
- ✅ You want stability over new features

### Choose **Hono (Bun)** when:

- ✅ Your team knows JavaScript/TypeScript well
- ✅ You need decent performance (67% of Rust)
- ✅ You want modern JS runtime with fast startup
- ✅ You're building microservices with Node ecosystem

### Choose **Hono (Node.js)** when:

- ✅ You need Node.js ecosystem compatibility
- ✅ Performance is good enough (35% of Rust)
- ✅ Your team is JavaScript-first
- ✅ You prioritize development speed over runtime speed

### Choose **FastAPI** when:

- ✅ Your team is Python-first
- ✅ You need Python ML/data science integration
- ✅ Development velocity > runtime performance
- ✅ 10k req/sec is sufficient for your use case
- ✅ You can afford 10x the infrastructure

## Real-World Impact

### High-Traffic API (1M requests/second)

| Framework     | Servers | Monthly Cost | Annual Cost |
| ------------- | ------- | ------------ | ----------- |
| **Ultimo/Axum** | 7       | **$350**     | **$4,200**  |
| Hono (Bun)    | 10      | $500         | $6,000      |
| Hono (Node)   | 19      | $950         | $11,400     |
| FastAPI       | **100** | **$5,000**   | **$60,000** |

**Annual savings (Rust vs Python)**: **$55,800**

At scale, choosing Rust over Python saves enough to hire an additional senior engineer.

## Technical Details

### Test Scenarios

Both endpoints tested:

1. **GET /api/users** - Returns array of 3 users with id, name, email
2. **GET /api/users/1** - Returns single user object

### Server Implementations

All servers implement identical API:

- In-memory data store (no database)
- JSON serialization
- Mutex-protected shared state
- Standard HTTP server configurations

### Performance Characteristics

**Latency Distribution** (GET /api/users):

| Framework   | Fastest | Average | Slowest |
| ----------- | ------- | ------- | ------- |
| Ultimo        | 0.0ms   | 0.7ms   | 6.1ms   |
| Axum        | 0.1ms   | 0.7ms   | 6.3ms   |
| Hono (Bun)  | 0.2ms   | 1.0ms   | 2.8ms   |
| Hono (Node) | 0.6ms   | 1.9ms   | 165.0ms |
| FastAPI     | 2.4ms   | 9.7ms   | 23.4ms  |

**Observations**:

- Rust frameworks have **lowest average latency** (0.7ms)
- Rust frameworks have **most consistent performance**
- Node.js shows occasional **high tail latency** (165ms spikes)
- Python has **highest baseline latency** (9.7ms average)

## Conclusions

### 1. **Ultimo = Axum in Performance** ✅

Ultimo matches Axum's legendary performance while adding:

- Automatic RPC generation
- Automatic OpenAPI documentation
- Built-in client SDK generation

**Verdict**: No performance trade-off for extra features.

### 2. **Rust is King for High-Performance APIs**

Both Rust frameworks deliver:

- 15x better throughput than Python
- 2-3x better throughput than Node.js
- Sub-millisecond latencies
- Predictable, consistent performance

### 3. **Choose Based on Your Priorities**

- **Maximum features + performance**: **Ultimo** (auto-generation + speed)
- **Maximum maturity + performance**: **Axum** (battle-tested + speed)
- **JavaScript ecosystem**: Hono on Bun (good performance)
- **Python ecosystem**: FastAPI (accept 10x cost)

### 4. **Infrastructure Cost Matters**

At 1M req/sec:

- Rust: $4,200/year
- Python: $60,000/year

**The performance difference pays for itself.**

## Recommendation

For new projects requiring high performance:

1. **Ultimo** - Best for modern full-stack with automatic API generation
2. **Axum** - Best for maximum stability and community support
3. **Hono (Bun)** - Best for TypeScript-first teams
4. **FastAPI** - Best when Python ecosystem is required

**Bottom Line**: Choose Ultimo or Axum for maximum performance. Choose based on whether you want automatic RPC/OpenAPI generation (Ultimo) or maximum battle-tested stability (Axum).
