# Testing Optional Database Features

## Overview

The Ultimo framework includes optional database integrations (SQLx and Diesel) that are behind feature flags. These show **0% coverage** in default tests because they're not enabled.

## Why Optional Features?

- **Lean builds**: Users only include what they need
- **No forced dependencies**: Don't require PostgreSQL/MySQL/SQLite if not used
- **Flexible choice**: Support both SQLx and Diesel ORMs

## Current Optional Features

### Database Features (0% Coverage by Default)

1. **`database/sqlx.rs`** - SQLx integration
2. **`database/diesel.rs`** - Diesel ORM integration
3. **`database/mod.rs`** - Database coordinator module

## Testing Database Features

### Prerequisites

Install database client libraries:

```bash
# macOS
brew install postgresql@14 mysql sqlite

# Linux (Ubuntu/Debian)
sudo apt-get install libpq-dev libmysqlclient-dev libsqlite3-dev

# Linux (Fedora)
sudo dnf install postgresql-devel mysql-devel sqlite-devel
```

### Run Tests with Database Features

```bash
# Test with SQLx PostgreSQL
cargo test --lib --features sqlx-postgres

# Test with SQLx MySQL
cargo test --lib --features sqlx-mysql

# Test with SQLx SQLite (no system deps required)
cargo test --lib --features sqlx-sqlite

# Test with Diesel PostgreSQL
cargo test --lib --features diesel-postgres

# Test with Diesel MySQL
cargo test --lib --features diesel-mysql

# Test with Diesel SQLite
cargo test --lib --features diesel-sqlite

# Test ALL features (requires all DB client libraries)
cargo test --lib --all-features
```

### Coverage with Database Features

```bash
# Generate coverage with all features
RUSTFLAGS="-C instrument-coverage" \
LLVM_PROFILE_FILE="target/coverage/ultimo-%p-%m.profraw" \
cargo test --lib --all-features

# Then run the coverage tool
cargo coverage
```

## Adding Tests for Database Features

The database modules already have some tests in `database/tests.rs`, but they're behind feature flags:

```rust
#[cfg(feature = "sqlx-postgres")]
mod sqlx_tests {
    // SQLx-specific tests
}

#[cfg(feature = "diesel-postgres")]
mod diesel_tests {
    // Diesel-specific tests
}
```

### What Should Be Tested

For full coverage, we need tests for:

1. **Connection Pool Creation**

   - SQLx pool configuration
   - Diesel R2D2 pool configuration
   - Connection string parsing

2. **Database Enum Conversions**

   - `Database::Sqlx` variants
   - `Database::Diesel` variants
   - Type safety checks

3. **Error Handling**

   - Database connection errors
   - Query execution errors
   - Pool exhaustion scenarios

4. **Context Integration**
   - Database attachment to request context
   - Accessing database in handlers
   - State management

## Current Test Coverage (with `database` feature)

When tested with `--features database`:

- ✅ `database/error.rs`: 100%
- ✅ `database/tests.rs`: 83.70%
- ❌ `database/mod.rs`: 0% (needs feature-specific tests)
- ❌ `database/sqlx.rs`: 0% (needs SQLx backend)
- ❌ `database/diesel.rs`: 0% (needs Diesel backend)

## Recommended Approach

### Option 1: CI Testing (Recommended)

Add GitHub Actions workflow to test with all features:

```yaml
# .github/workflows/coverage.yml
name: Coverage

on: [push, pull_request]

jobs:
  coverage:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:14
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      mysql:
        image: mysql:8
        env:
          MYSQL_ROOT_PASSWORD: mysql
        options: >-
          --health-cmd "mysqladmin ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libpq-dev libmysqlclient-dev libsqlite3-dev

      - name: Run tests with all features
        run: cargo test --lib --all-features

      - name: Generate coverage
        run: cargo coverage
```

### Option 2: Docker Testing

Use Docker Compose for local testing:

```yaml
# docker-compose.test.yml
version: "3.8"

services:
  postgres:
    image: postgres:14
    environment:
      POSTGRES_PASSWORD: postgres
    ports:
      - "5432:5432"

  mysql:
    image: mysql:8
    environment:
      MYSQL_ROOT_PASSWORD: mysql
    ports:
      - "3306:3306"
```

```bash
# Start databases
docker-compose -f docker-compose.test.yml up -d

# Run tests
cargo test --lib --all-features

# Stop databases
docker-compose -f docker-compose.test.yml down
```

### Option 3: Mock Testing

Add mock implementations that don't require actual databases:

```rust
#[cfg(all(test, feature = "database"))]
mod mock_tests {
    // Test configuration and types without real DB connections

    #[test]
    fn test_database_enum_creation() {
        // Test enum variants exist and convert correctly
    }

    #[test]
    fn test_pool_config_types() {
        // Test configuration struct types
    }
}
```

## Target Coverage Goals

Once database features are properly tested:

- **Overall Goal**: 70%+ (from current 63.58%)
- **database/mod.rs**: 60%+
- **database/sqlx.rs**: 60%+
- **database/diesel.rs**: 60%+

This would give us:

- 12 files with actual coverage
- 3 database files properly tested
- **Estimated overall**: 68-72% coverage

## Action Items

- [ ] Set up CI pipeline with database services
- [ ] Add more unit tests for database types (no connection needed)
- [ ] Test with SQLite (easiest - no server required)
- [ ] Document database testing in main README
- [ ] Consider adding integration tests separate from unit tests

## Summary

Yes, you're absolutely right - we should test these files! The 0% coverage is technically correct (code not compiled), but we should:

1. **Add CI testing** with database features enabled
2. **Use SQLite tests** as minimum (no server needed)
3. **Document** that these are optional features
4. **Keep default builds lean** but ensure quality when features are enabled

This gives users confidence that database features work correctly when they choose to use them.
