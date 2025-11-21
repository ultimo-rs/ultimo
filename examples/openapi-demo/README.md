# Ultimo OpenAPI Demo

This example demonstrates OpenAPI 3.0 integration with Ultimo framework in two distinct modes:

## Overview

The demo provides two separate server implementations showcasing different API design approaches:

1. **REST Mode** (`rest-server`) - Traditional RESTful API with resource-based endpoints
2. **RPC Mode** (`rpc-server`) - Function-oriented API using Ultimo's RPC registry

## 1. REST Mode

Regular REST API with conventional endpoints and detailed OpenAPI schemas.

### Run

```bash
cargo run --bin rest-server
```

### Endpoints

- `GET /api/users/:id` - Get user by ID
- `GET /api/users` - List all users
- `POST /api/users` - Create a new user

### Features

- ✅ Standard RESTful routes
- ✅ Path parameters (`:id`)
- ✅ Manual OpenAPI spec generation
- ✅ Detailed field-level schema definitions
- ✅ Proper HTTP status codes

### Documentation

- **Swagger UI**: http://127.0.0.1:3000/docs
- **OpenAPI Spec**: http://127.0.0.1:3000/openapi.json
- **Saved File**: `openapi-rest.json`

### Example Request

```bash
# List all users
curl http://127.0.0.1:3000/api/users

# Get specific user
curl http://127.0.0.1:3000/api/users/1

# Create new user
curl -X POST http://127.0.0.1:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Charlie","email":"charlie@example.com"}'
```

## 2. RPC Mode

RPC-based API using Ultimo's RPC registry with automatic OpenAPI generation.

### Run

```bash
cargo run --bin rpc-server
```

### Endpoints

- `GET /api/getUser?id=1` - Get user by ID
- `GET /api/listUsers` - List all users
- `POST /api/createUser` - Create a new user

### Features

- ✅ RPC registry with query/mutation methods
- ✅ Automatic OpenAPI generation from RPC procedures
- ✅ Function-style endpoint naming
- ✅ TypeScript type definitions
- ✅ Less boilerplate code

### Documentation

- **Swagger UI**: http://127.0.0.1:3000/docs
- **OpenAPI Spec**: http://127.0.0.1:3000/openapi.json
- **Saved File**: `openapi-rpc.json`

### Example Request

```bash
# List all users
curl http://127.0.0.1:3000/api/listUsers

# Get specific user
curl "http://127.0.0.1:3000/api/getUser?id=1"

# Create new user
curl -X POST http://127.0.0.1:3000/api/createUser \
  -H "Content-Type: application/json" \
  -d '{"name":"Charlie","email":"charlie@example.com"}'
```

## Comparison

| Feature                | REST Mode                        | RPC Mode                                             |
| ---------------------- | -------------------------------- | ---------------------------------------------------- |
| **Endpoint Style**     | `/api/users/:id`                 | `/api/getUser?id=1`                                  |
| **HTTP Methods**       | RESTful (GET, POST on resources) | Function-based (GET for queries, POST for mutations) |
| **OpenAPI Generation** | Manual with detailed schemas     | Automatic from RPC registry                          |
| **Schema Quality**     | Detailed field definitions       | Generic object types                                 |
| **Boilerplate**        | More (manual spec building)      | Less (auto-generated)                                |
| **Use Case**           | Traditional REST APIs            | Function-oriented APIs                               |
| **Learning Curve**     | Familiar to REST developers      | Simpler for RPC-style APIs                           |

## Interactive Testing

Both servers provide Swagger UI at `/docs` endpoint where you can:

1. **Browse** all available endpoints
2. **Try it out** - Interactive request testing
3. **View** request/response schemas
4. **Execute** real API calls
5. **See** example responses

## Using with External Tools

### Prism Mock Server

Create a mock server from your OpenAPI spec:

```bash
# Install Prism
npm install -g @stoplight/prism-cli

# Run mock server (REST mode)
prism mock openapi-rest.json

# Run mock server (RPC mode)
prism mock openapi-rpc.json

# Test it
curl http://localhost:4010/api/users
```

### OpenAPI Generator (Multi-Language Clients)

Generate clients in various languages:

#### TypeScript

```bash
npx @openapitools/openapi-generator-cli generate \
  -i openapi-rest.json \
  -g typescript-fetch \
  -o ./generated-client-ts
```

#### Python

```bash
npx @openapitools/openapi-generator-cli generate \
  -i openapi-rest.json \
  -g python \
  -o ./generated-client-py
```

#### Go

```bash
npx @openapitools/openapi-generator-cli generate \
  -i openapi-rest.json \
  -g go \
  -o ./generated-client-go
```

### Validate Spec

```bash
# Install validator
npm install -g @apidevtools/swagger-cli

# Validate spec
swagger-cli validate openapi-rest.json
swagger-cli validate openapi-rpc.json
```

## Generate TypeScript Types

Generate TypeScript client types from RPC procedures:

```bash
cargo run --bin generate
```

This creates `client-types.ts` with TypeScript definitions for the API.

## Benefits

### REST Mode Benefits

- ✅ **Industry standard** REST conventions
- ✅ **Detailed schemas** with field-level definitions
- ✅ **Better tooling support** (many tools expect REST)
- ✅ **Semantic HTTP methods** (GET, POST, PUT, DELETE)

### RPC Mode Benefits

- ✅ **Less boilerplate** (automatic OpenAPI generation)
- ✅ **Type-safe** TypeScript generation
- ✅ **Function-oriented** (matches business logic)
- ✅ **Simpler routing** (no resource modeling needed)

## When to Use Each Mode

### Use REST Mode When:

- Building **public APIs** for third-party developers
- Need **maximum compatibility** with existing tools
- Following **strict REST conventions**
- Require **detailed documentation** with examples
- Working with **resource-oriented** domains

### Use RPC Mode When:

- Building **internal APIs** for your own frontend
- Want **rapid development** with less boilerplate
- Prefer **function-oriented** API design
- Need **TypeScript integration** out of the box
- Working with **action-oriented** domains

## Code Examples

### REST Mode Code

```rust
// Manual OpenAPI generation with detailed schemas
let mut openapi = OpenApiBuilder::new()
    .title("User API - REST Mode")
    .version("1.0.0")
    .build();

// Define detailed operation
let mut operation = Operation {
    summary: Some("Get user by ID".to_string()),
    parameters: Some(vec![...]),
    responses: HashMap::new(),
    // ... detailed schema definitions
};

openapi.add_path("/api/users/{id}".to_string(), PathItem {
    get: Some(operation),
    post: None,
    put: None,
    delete: None,
    patch: None,
});
```

### RPC Mode Code

```rust
// Automatic OpenAPI generation from RPC registry
let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);

rpc.query("getUser", handler, "{ id: number }", "User");
rpc.mutation("createUser", handler, "{ name: string }", "User");

// One-liner generates complete OpenAPI spec
let openapi = rpc.generate_openapi("User API - RPC Mode", "1.0.0", "/api");
```

## Resources

- [OpenAPI Specification](https://swagger.io/specification/)
- [Swagger UI](https://swagger.io/tools/swagger-ui/)
- [Prism Mock Server](https://github.com/stoplightio/prism)
- [OpenAPI Generator](https://openapi-generator.tech/)
