# Testing & Coverage Guide

This document outlines the testing strategy, coverage standards, and best practices for the Ultimo framework.

## 📊 Coverage Standards

**Minimum Coverage Threshold: 60%**

- ✅ All new features must include tests
- ✅ Core modules should maintain 70%+ coverage
- ✅ CI pipeline enforces minimum threshold
- ✅ Coverage reports generated on every PR

## 🧪 Test Statistics

| Metric                   | Value  |
| ------------------------ | ------ |
| **Total Tests**          | 77     |
| **Test Functions**       | 79     |
| **Estimated Coverage**   | 60-65% |
| **Core Module Coverage** | 70%+   |

## 📋 Module Coverage Breakdown

| Module          | Lines | Tests | Coverage | Status       |
| --------------- | ----- | ----- | -------- | ------------ |
| `router.rs`     | ~250  | 7     | ~80%     | ✅ Excellent |
| `rpc.rs`        | ~950  | 17    | ~70%     | ✅ Good      |
| `openapi.rs`    | ~720  | 20    | ~75%     | ✅ Good      |
| `middleware.rs` | ~350  | 10    | ~65%     | ✅ Good      |
| `database/`     | ~200  | 10    | ~85%     | ✅ Excellent |
| `response.rs`   | ~160  | 6     | ~70%     | ✅ Good      |
| `validation.rs` | ~50   | 2     | ~80%     | ✅ Good      |
| `context.rs`    | ~314  | 1     | ~20%     | ⚠️ Low       |
| `error.rs`      | ~200  | 2     | ~40%     | ⚠️ Moderate  |
| `app.rs`        | ~280  | 2     | ~30%     | ⚠️ Low       |
| `handler.rs`    | ~45   | 1     | ~50%     | ⚠️ Moderate  |

## 🚀 Quick Start

### Running Tests

```bash
# Run all library tests
cargo test --lib

# Run with detailed output
cargo test --lib -- --nocapture

# Run specific test
cargo test --lib test_name

# Run tests in watch mode
cargo watch -x 'test --lib'

# Or use Make
make test
make test-watch
```

### Generating Coverage Reports

```bash
# Install coverage tool (one-time setup)
cargo install cargo-llvm-cov

# Generate HTML coverage report
./scripts/coverage.sh

# View the report
open target/llvm-cov/html/index.html

# Or use Make shortcuts
make coverage
make coverage-open
```

### Alternative: Quick Commands

```bash
# Using cargo aliases (defined in .cargo/config.toml)
cargo coverage              # Generate HTML report
cargo coverage-summary      # Print summary to terminal
cargo coverage-json         # Generate JSON report
```

## 🛠️ Development Workflow

### Before Committing

```bash
# Run the complete CI check locally
make ci

# This runs:
# 1. cargo clippy --all-targets --all-features
# 2. cargo fmt --all -- --check
# 3. cargo test --lib --no-fail-fast
# 4. ./scripts/coverage.sh (checks minimum threshold)
```

### Writing Tests

#### Unit Tests

Place tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        let result = my_function();
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = my_async_function().await;
        assert!(result.is_ok());
    }
}
```

#### Integration Tests

Place in `ultimo/tests/` directory:

```rust
// ultimo/tests/integration_test.rs
use ultimo::prelude::*;

#[tokio::test]
async fn test_full_workflow() {
    // Test complete scenarios
}
```

## 📁 Project Structure

```
aero-rs/
├── .cargo/
│   └── config.toml          # Cargo aliases for coverage
├── scripts/
│   ├── coverage.sh          # Coverage generation script
│   └── test-summary.sh      # Test statistics script
├── ultimo/
│   ├── src/
│   │   ├── *.rs            # Source files with #[cfg(test)] modules
│   │   └── lib.rs
│   └── tests/              # Integration tests
│       └── integration_test.rs
├── Makefile                 # Development commands
└── TESTING.md              # This file
```

## 🎯 Coverage Goals

### Current Priority: Increase Coverage to 70%

**Modules needing more tests:**

1. `context.rs` - Context methods (req/res handling)
2. `app.rs` - HTTP server integration
3. `error.rs` - Error conversion paths
4. `handler.rs` - Handler trait implementations

### How to Contribute

1. **Find untested code**: Run coverage report and identify gaps
2. **Write tests**: Add unit tests for uncovered functions
3. **Verify coverage**: Run `./scripts/coverage.sh` to check improvement
4. **Submit PR**: Include test coverage in PR description

## 🔧 Troubleshooting

### cargo-llvm-cov Installation Issues

If `cargo install cargo-llvm-cov` fails:

```bash
# Update Rust to latest stable
rustup update stable

# Ensure llvm-tools is installed
rustup component add llvm-tools-preview

# Try installation again
cargo install cargo-llvm-cov
```

### Alternative Coverage Tools

If cargo-llvm-cov doesn't work, try:

```bash
# Using tarpaulin (Linux only)
cargo install cargo-tarpaulin
cargo tarpaulin --lib --out Html

# Using grcov (requires nightly)
rustup install nightly
cargo +nightly install grcov
CARGO_INCREMENTAL=0 RUSTFLAGS='-C instrument-coverage' cargo test
grcov . --binary-path ./target/debug/ -s . -t html --branch --ignore-not-existing -o ./target/coverage/
```

### Coverage Not Matching Expected

The coverage tool may exclude:

- Test code itself (`#[cfg(test)]` modules)
- Generated code
- External dependencies
- Code behind feature flags not enabled

## 📚 Best Practices

### ✅ DO

- Write tests for public APIs
- Test error cases and edge cases
- Use descriptive test names
- Test one thing per test
- Use `#[should_panic]` for expected panics
- Document test purpose with comments
- Keep tests fast and isolated

### ❌ DON'T

- Test implementation details
- Write tests that depend on external state
- Skip tests without good reason
- Test private functions directly (test through public API)
- Commit code without running tests
- Ignore failing tests

## 🤖 CI/CD Integration

GitHub Actions workflow (`.github/workflows/test.yml`):

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Install coverage tool
        run: cargo install cargo-llvm-cov

      - name: Run tests
        run: cargo test --lib --no-fail-fast

      - name: Generate coverage
        run: cargo llvm-cov --lib --lcov --output-path lcov.info

      - name: Upload to Codecov
        uses: codecov/codecov-action@v4
        with:
          files: lcov.info

      - name: Check coverage threshold
        run: ./scripts/coverage.sh
```

## 📊 Coverage Badges

Add to README.md:

```markdown
[![codecov](https://codecov.io/gh/yourusername/ultimo-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/yourusername/ultimo-rs)
```

## 🔗 Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [cargo-llvm-cov Documentation](https://github.com/taiki-e/cargo-llvm-cov)
- [Codecov Integration](https://about.codecov.io/)
- [Testing Best Practices](https://matklad.github.io/2021/05/31/how-to-test.html)

---

**Last Updated:** November 20, 2025  
**Maintainers:** Ultimo Contributors
