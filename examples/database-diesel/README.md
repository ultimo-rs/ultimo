# Database Diesel Example

This example demonstrates how to use Ultimo with Diesel ORM (PostgreSQL).

## Features

- ✅ Connection pooling with Diesel + r2d2
- ✅ CRUD operations (Create, Read, Update, Delete)
- ✅ Database access from Context with `ctx.diesel()`
- ✅ Transactions
- ✅ Schema definition with macros
- ✅ Type-safe queries with Diesel DSL
- ✅ Error handling

## Prerequisites

You need a running PostgreSQL database. The easiest way is with Docker:

```bash
docker run --name postgres-ultimo \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=ultimo_example \
  -p 5432:5432 \
  -d postgres:16
```

## Setup

1. **Set database URL** (optional, defaults to localhost):

```bash
export DATABASE_URL="postgres://postgres:postgres@localhost/ultimo_example"
```

2. **Run the example**:

```bash
cargo run --release
```

The server will:

- Connect to the database
- Create the `users` table if it doesn't exist
- Start listening on `http://127.0.0.1:3001`

## API Endpoints

### Health Check

```bash
curl http://localhost:3001/health
```

Response:

```json
{
  "status": "healthy",
  "database": "connected"
}
```

### Create User

```bash
curl -X POST http://localhost:3001/users \
  -H 'Content-Type: application/json' \
  -d '{"name":"Alice","email":"alice@example.com"}'
```

Response:

```json
{
  "id": 1,
  "name": "Alice",
  "email": "alice@example.com"
}
```

### List Users

```bash
curl http://localhost:3001/users
```

Response:

```json
{
  "users": [
    { "id": 1, "name": "Alice", "email": "alice@example.com" },
    { "id": 2, "name": "Bob", "email": "bob@example.com" }
  ],
  "total": 2
}
```

### Get User by ID

```bash
curl http://localhost:3001/users/1
```

Response:

```json
{
  "id": 1,
  "name": "Alice",
  "email": "alice@example.com"
}
```

### Update User

```bash
curl -X PUT http://localhost:3001/users/1 \
  -H 'Content-Type: application/json' \
  -d '{"name":"Alice Smith","email":"alice.smith@example.com"}'
```

### Delete User

```bash
curl -X DELETE http://localhost:3001/users/1
```

Returns `204 No Content` on success.

### Batch Create (Transaction Example)

```bash
curl -X POST http://localhost:3001/users/batch \
  -H 'Content-Type: application/json' \
  -d '[
    {"name":"Alice","email":"alice@example.com"},
    {"name":"Bob","email":"bob@example.com"},
    {"name":"Carol","email":"carol@example.com"}
  ]'
```

Response:

```json
{
  "created": 3,
  "users": [
    { "id": 1, "name": "Alice", "email": "alice@example.com" },
    { "id": 2, "name": "Bob", "email": "bob@example.com" },
    { "id": 3, "name": "Carol", "email": "carol@example.com" }
  ]
}
```

## Code Highlights

### Database Connection

```rust
let pool = DieselPool::new(&database_url)?;
app.with_diesel(pool);
```

### Schema Definition

```rust
diesel::table! {
    users (id) {
        id -> Int4,
        name -> Varchar,
        email -> Varchar,
    }
}
```

### Using Database in Handlers

```rust
app.get("/users", |ctx: Context| async move {
    let mut conn = ctx.diesel::<diesel::PgConnection>()?;

    let users = users::table
        .select(User::as_select())
        .load(&mut *conn)?;

    ctx.json(users).await
});
```

### Transactions

```rust
app.post("/users/batch", |ctx: Context| async move {
    let mut conn = ctx.diesel::<diesel::PgConnection>()?;

    // Run in transaction - all or nothing
    let results = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let mut created = Vec::new();
        for input in inputs {
            let user = diesel::insert_into(users::table)
                .values(&input)
                .returning(User::as_returning())
                .get_result(conn)?;
            created.push(user);
        }
        Ok(created)
    })?;

    ctx.json(results).await
});
```

## Diesel vs SQLx

### Diesel Advantages:

- ✅ **Compile-time verification** - Catches schema mismatches at compile time
- ✅ **Type-safe queries** - DSL prevents SQL injection and type errors
- ✅ **Better IDE support** - Full autocomplete for schema and queries
- ✅ **No DATABASE_URL required** - Schema is defined in code

### SQLx Advantages:

- ✅ **Raw SQL** - Write queries in SQL directly
- ✅ **Async native** - Built for async from the ground up
- ✅ **Simpler for complex queries** - Some queries are easier in raw SQL
- ✅ **Multiple databases** - Easier to switch between databases

## Error Handling

The example includes proper error handling:

- ✅ Invalid user ID → 400 Bad Request
- ✅ User not found → 404 Not Found
- ✅ Email already exists → 400 Bad Request
- ✅ Database errors → 500 Internal Server Error
- ✅ Transaction rollback on any error

## Diesel CLI (Optional)

For more advanced usage, you can use Diesel CLI:

```bash
# Install Diesel CLI
cargo install diesel_cli --no-default-features --features postgres

# Setup diesel
diesel setup

# Create migration
diesel migration generate create_users

# Run migrations
diesel migration run
```

## Testing

You can test all endpoints with this script:

```bash
#!/bin/bash

# Health check
echo "=== Health Check ==="
curl http://localhost:3001/health
echo -e "\n"

# Create users
echo "=== Create User 1 ==="
curl -X POST http://localhost:3001/users \
  -H 'Content-Type: application/json' \
  -d '{"name":"Alice","email":"alice@example.com"}'
echo -e "\n"

echo "=== Create User 2 ==="
curl -X POST http://localhost:3001/users \
  -H 'Content-Type: application/json' \
  -d '{"name":"Bob","email":"bob@example.com"}'
echo -e "\n"

# List users
echo "=== List Users ==="
curl http://localhost:3001/users
echo -e "\n"

# Batch create
echo "=== Batch Create ==="
curl -X POST http://localhost:3001/users/batch \
  -H 'Content-Type: application/json' \
  -d '[{"name":"Carol","email":"carol@example.com"},{"name":"Dave","email":"dave@example.com"}]'
echo -e "\n"

# Get user
echo "=== Get User 1 ==="
curl http://localhost:3001/users/1
echo -e "\n"

# Update user
echo "=== Update User 1 ==="
curl -X PUT http://localhost:3001/users/1 \
  -H 'Content-Type: application/json' \
  -d '{"name":"Alice Smith","email":"alice.smith@example.com"}'
echo -e "\n"

# Delete user
echo "=== Delete User 1 ==="
curl -X DELETE http://localhost:3001/users/1
echo -e "\n"

# List users again
echo "=== List Users After Delete ==="
curl http://localhost:3001/users
echo -e "\n"
```

## Cleanup

```bash
# Stop and remove Docker container
docker stop postgres-ultimo
docker rm postgres-ultimo
```

## Learn More

- [Diesel Documentation](https://diesel.rs/)
- [Ultimo Database Guide](../../docs/DATABASE.md)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
