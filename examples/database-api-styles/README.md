# Database API Styles Example

This example demonstrates that **Ultimo's database integration is completely flexible** and works with ANY API routing style.

## The Key Concept

> **Database operations (`ctx.sqlx()` and `ctx.diesel()`) are agnostic to your API routing style.**

The same database logic works seamlessly with:

1. **Traditional REST** - Resource-based URLs with HTTP verbs
2. **JSON-RPC** - Single endpoint with method dispatch
3. **RPC-Style REST** - Action-based multiple endpoints

## Three API Styles, Same Database Code

All three styles use the **exact same database operations**:

```rust
// Shared database logic in db_operations module
pub async fn create_user(pool: &PgPool, input: CreateUserInput) -> Result<User> {
    sqlx::query_as("INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *")
        .bind(&input.name)
        .bind(&input.email)
        .fetch_one(pool)
        .await
}
```

This function is reused in all three API styles - just wrapped with different routing patterns!

## Running the Example

1. **Start PostgreSQL:**

```bash
docker run --name postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=ultimo_example \
  -p 5432:5432 \
  -d postgres:16
```

2. **Run the server:**

```bash
cd examples/database-api-styles
cargo run
```

Server starts on `http://127.0.0.1:3003`

3. **Open documentation:**

```bash
open http://127.0.0.1:3003
```

## API Style 1: Traditional REST

Resource-based URLs with HTTP verbs.

**Endpoints:**

- `GET /rest/users` - List all users
- `GET /rest/users/:id` - Get user by ID
- `POST /rest/users` - Create user
- `PUT /rest/users/:id` - Update user
- `DELETE /rest/users/:id` - Delete user

**Example:**

```bash
# List users
curl http://localhost:3003/rest/users

# Create user
curl -X POST http://localhost:3003/rest/users \
  -H 'Content-Type: application/json' \
  -d '{"name":"Alice","email":"alice@example.com"}'

# Get user
curl http://localhost:3003/rest/users/1

# Update user
curl -X PUT http://localhost:3003/rest/users/1 \
  -H 'Content-Type: application/json' \
  -d '{"name":"Alice Smith"}'

# Delete user
curl -X DELETE http://localhost:3003/rest/users/1
```

**Best for:** Public APIs, standard CRUD, HTTP caching

## API Style 2: JSON-RPC (Single Endpoint)

All operations through one endpoint with method dispatch.

**Endpoint:**

- `POST /rpc`

**Example:**

```bash
# List users
curl -X POST http://localhost:3003/rpc \
  -H 'Content-Type: application/json' \
  -d '{"method":"listUsers","params":{}}'

# Create user
curl -X POST http://localhost:3003/rpc \
  -H 'Content-Type: application/json' \
  -d '{"method":"createUser","params":{"name":"Bob","email":"bob@example.com"}}'

# Get user
curl -X POST http://localhost:3003/rpc \
  -H 'Content-Type: application/json' \
  -d '{"method":"getUser","params":{"id":1}}'

# Update user
curl -X POST http://localhost:3003/rpc \
  -H 'Content-Type: application/json' \
  -d '{"method":"updateUser","params":{"id":1,"name":"Bob Smith"}}'

# Delete user
curl -X POST http://localhost:3003/rpc \
  -H 'Content-Type: application/json' \
  -d '{"method":"deleteUser","params":{"id":1}}'
```

**Best for:** Internal APIs, microservices, request batching

## API Style 3: RPC-Style REST (Multiple Endpoints)

Action-based endpoints (like tRPC).

**Endpoints:**

- `GET /rpc-rest/listUsers` - List all users
- `GET /rpc-rest/getUser?id=1` - Get user by ID
- `POST /rpc-rest/createUser` - Create user
- `POST /rpc-rest/updateUser` - Update user
- `POST /rpc-rest/deleteUser` - Delete user

**Example:**

```bash
# List users
curl http://localhost:3003/rpc-rest/listUsers

# Create user
curl -X POST http://localhost:3003/rpc-rest/createUser \
  -H 'Content-Type: application/json' \
  -d '{"name":"Carol","email":"carol@example.com"}'

# Get user
curl 'http://localhost:3003/rpc-rest/getUser?id=1'

# Update user
curl -X POST http://localhost:3003/rpc-rest/updateUser \
  -H 'Content-Type: application/json' \
  -d '{"id":1,"name":"Carol Smith"}'

# Delete user
curl -X POST http://localhost:3003/rpc-rest/deleteUser \
  -H 'Content-Type: application/json' \
  -d '{"id":1}'
```

**Best for:** Action-oriented APIs, TypeScript-first apps, clear debugging

## Comparison

| Feature      | REST                   | JSON-RPC          | RPC-REST          |
| ------------ | ---------------------- | ----------------- | ----------------- |
| Endpoints    | Multiple (resource)    | Single            | Multiple (action) |
| HTTP Methods | GET, POST, PUT, DELETE | POST only         | GET, POST         |
| URL Pattern  | `/users/:id`           | `/rpc`            | `/getUser?id=1`   |
| Caching      | ✅ GET requests        | ❌ POST only      | ✅ GET queries    |
| Debugging    | ✅ Clear URLs          | ⚠️ Check body     | ✅ Clear URLs     |
| Batching     | ❌ No                  | ✅ Yes            | ❌ No             |
| **Database** | **✅ ctx.sqlx()**      | **✅ ctx.sqlx()** | **✅ ctx.sqlx()** |

## The Architecture

```
┌─────────────────────────────────────┐
│     Shared Database Operations      │
│   (ctx.sqlx() / ctx.diesel())      │
│                                     │
│  • list_users(pool)                │
│  • get_user(pool, id)              │
│  • create_user(pool, input)        │
│  • update_user(pool, id, input)    │
│  • delete_user(pool, id)           │
└──────────────┬──────────────────────┘
               │
       ┌───────┴───────┐
       │               │
       ▼               ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│  REST API   │ │  JSON-RPC   │ │  RPC-REST   │
├─────────────┤ ├─────────────┤ ├─────────────┤
│ GET /users  │ │ POST /rpc   │ │ GET /list   │
│ POST /users │ │   method    │ │ POST /create│
│ PUT /users  │ │   params    │ │ POST /update│
│ DELETE /..  │ │             │ │ POST /delete│
└─────────────┘ └─────────────┘ └─────────────┘
       │               │               │
       └───────┬───────┴───────────────┘
               │
               ▼
    ┌──────────────────────┐
    │  Same Database Pool  │
    │  (SQLx or Diesel)    │
    └──────────────────────┘
```

## Key Takeaway

**You're not locked into any API style!** Ultimo's database integration is orthogonal to routing. Choose the style that fits your needs:

- **REST** for public APIs
- **JSON-RPC** for microservices
- **RPC-REST** for TypeScript apps
- **Hybrid** - mix and match!

Change your API style anytime without touching database code! 🚀

## Adding TypeScript & OpenAPI

All three styles work with:

- `ts-rs` for automatic TypeScript type generation
- OpenAPI spec generation
- Generated TypeScript clients

See the main documentation for details on adding these features to any API style.
