# Axum Benchmark Server

Rust/Axum implementation for performance benchmarking.

[Axum](https://github.com/tokio-rs/axum) is the most popular Rust web framework, built on top of hyper and tokio.

## Setup

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

Server will start on http://localhost:3004

## API Endpoints

- `GET /api/users` - List all users
- `GET /api/users/{id}` - Get user by ID
- `POST /api/users` - Create new user
- `DELETE /api/users/{id}` - Delete user
