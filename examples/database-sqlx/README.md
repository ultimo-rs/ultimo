# Database SQLx Example

This example demonstrates how to use Ultimo with SQLx (PostgreSQL).

## Features

- ✅ Connection pooling with SQLx
- ✅ CRUD operations (Create, Read, Update, Delete)
- ✅ Database access from Context with `ctx.sqlx()`
- ✅ Transactions
- ✅ Error handling
- ✅ Health check endpoint

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
- Start listening on `http://127.0.0.1:3000`

## API Endpoints

### Health Check

```bash
curl http://localhost:3000/health
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
curl -X POST http://localhost:3000/users \
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
curl http://localhost:3000/users
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
curl http://localhost:3000/users/1
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
curl -X PUT http://localhost:3000/users/1 \
  -H 'Content-Type: application/json' \
  -d '{"name":"Alice Smith","email":"alice.smith@example.com"}'
```

### Delete User

```bash
curl -X DELETE http://localhost:3000/users/1
```

Returns `204 No Content` on success.

### Transaction Example

```bash
curl -X POST http://localhost:3000/transfer \
  -H 'Content-Type: application/json' \
  -d '{"from_user_id":1,"to_user_id":2,"amount":100}'
```

## Code Highlights

### Database Connection

```rust
let pool = SqlxPool::connect(&database_url).await?;
app.with_sqlx(pool);
```

### Using Database in Handlers

```rust
app.get("/users", |ctx: Context| async move {
    let db = ctx.sqlx::<sqlx::Postgres>()?;

    let users = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(db)
        .await?;

    ctx.json(users).await
});
```

### Transactions

```rust
app.post("/transfer", |mut ctx: Context| async move {
    let db = ctx.sqlx::<sqlx::Postgres>()?;

    // Start transaction
    let mut tx = db.begin().await?;

    // Execute queries
    sqlx::query("UPDATE accounts SET balance = balance - $1")
        .bind(amount)
        .execute(&mut *tx)
        .await?;

    // Commit
    tx.commit().await?;

    ctx.json(json!({"success": true})).await
});
```

## Error Handling

The example includes proper error handling:

- ✅ Invalid user ID → 400 Bad Request
- ✅ User not found → 404 Not Found
- ✅ Email already exists → 400 Bad Request
- ✅ Database errors → 500 Internal Server Error

## Testing

You can test all endpoints with this script:

```bash
#!/bin/bash

# Health check
echo "=== Health Check ==="
curl http://localhost:3000/health
echo -e "\n"

# Create users
echo "=== Create User 1 ==="
curl -X POST http://localhost:3000/users \
  -H 'Content-Type: application/json' \
  -d '{"name":"Alice","email":"alice@example.com"}'
echo -e "\n"

echo "=== Create User 2 ==="
curl -X POST http://localhost:3000/users \
  -H 'Content-Type: application/json' \
  -d '{"name":"Bob","email":"bob@example.com"}'
echo -e "\n"

# List users
echo "=== List Users ==="
curl http://localhost:3000/users
echo -e "\n"

# Get user
echo "=== Get User 1 ==="
curl http://localhost:3000/users/1
echo -e "\n"

# Update user
echo "=== Update User 1 ==="
curl -X PUT http://localhost:3000/users/1 \
  -H 'Content-Type: application/json' \
  -d '{"name":"Alice Smith","email":"alice.smith@example.com"}'
echo -e "\n"

# Transfer (transaction example)
echo "=== Transfer ==="
curl -X POST http://localhost:3000/transfer \
  -H 'Content-Type: application/json' \
  -d '{"from_user_id":1,"to_user_id":2,"amount":100}'
echo -e "\n"

# Delete user
echo "=== Delete User 1 ==="
curl -X DELETE http://localhost:3000/users/1
echo -e "\n"

# List users again
echo "=== List Users After Delete ==="
curl http://localhost:3000/users
echo -e "\n"
```

## Cleanup

```bash
# Stop and remove Docker container
docker stop postgres-ultimo
docker rm postgres-ultimo
```

## Learn More

- [SQLx Documentation](https://github.com/launchbadge/sqlx)
- [Ultimo Database Guide](../../docs/DATABASE.md)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
