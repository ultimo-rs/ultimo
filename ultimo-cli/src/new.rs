use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

/// The `ultimo` crate version scaffolds should pin, as a `major.minor` (`^`-compatible)
/// requirement. Derived from this CLI's own version (the CLI and framework share the
/// workspace version), so scaffolds always track the current release and never drift
/// back to a stale hardcoded pin.
fn ultimo_dep_version() -> String {
    let v = env!("CARGO_PKG_VERSION");
    let mut parts = v.split('.');
    format!(
        "{}.{}",
        parts.next().unwrap_or("0"),
        parts.next().unwrap_or("0")
    )
}

/// The `ts-rs` version scaffolds pin when they generate TypeScript types.
const TS_RS_DEP: &str = "12";

/// Write a consumer-facing `AGENTS.md` into a scaffolded project — orientation for
/// a coding agent *building on* this app (distinct from the Ultimo repo's own
/// maintainer-facing AGENTS.md). Kept generic so it fits every template.
fn write_agents_md(name: &str, project_dir: &Path) -> Result<()> {
    let agents = format!(
        r#"# {name} — working notes for coding agents

This is an [Ultimo](https://docs.ultimo.dev) app (Rust web framework on Hyper + Tokio).
When editing it, follow these conventions.

## Commands

- `cargo run` — start the server.
- `cargo test` — run tests.
- `ultimo dev` — hot-reload dev server (rebuilds on change).
- `ultimo generate -o ./client.ts` — regenerate the typed TypeScript client (RPC projects).

## Adding a route

Handlers take a `Context` and return `Result<Response>`. Build responses with
`ctx.json(..)`, `ctx.text(..)`, `ctx.html(..)`, `ctx.stream(..)`, or `ctx.sse(..)`:

```rust
app.get("/hello", |ctx: Context| async move {{
    ctx.json(serde_json::json!({{ "message": "hi" }})).await
}});
```

## Adding a typed RPC procedure (RPC projects)

Register on the `RpcRegistry`, then regenerate the client so the frontend types
stay in sync:

```rust
registry.query("getThing", |input: GetThingInput| async move {{
    Ok(Thing {{ /* ... */ }})
}});
// then: ultimo generate -o ./client.ts
```

Derive `TS` on request/response types (`#[derive(TS)]`) so their TypeScript types
are generated automatically.

## Conventions

- Return `Result<Response>` from handlers; return `Err(UltimoError::…)` for error
  responses (they become proper HTTP status codes).
- Optional capabilities are Cargo features (auth, sessions, csrf, websocket,
  static-files, compression, client-gen, database). Enable them in `Cargo.toml`.
- Prefer the built-in middleware (`ultimo::middleware::builtin`) over hand-rolling.

## Docs for agents

- Full docs: https://docs.ultimo.dev  ·  machine-readable: https://docs.ultimo.dev/llms.txt
- Context7: https://context7.com/ultimo-rs/ultimo
"#,
        name = name
    );
    fs::write(project_dir.join("AGENTS.md"), agents)?;
    Ok(())
}

pub async fn run(name: String, template: String) -> Result<()> {
    println!("🚀 Creating new project: {}", name.green());
    println!("📦 Template: {}", template);
    println!();

    let project_dir = Path::new(&name);

    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    match template.as_str() {
        "basic" => create_basic_template(&name, project_dir)?,
        "fullstack" => create_fullstack_template(&name, project_dir)?,
        "api-only" => create_api_template(&name, project_dir)?,
        "rpc" => create_rpc_template(&name, project_dir)?,
        "production" => create_production_template(&name, project_dir)?,
        _ => anyhow::bail!(
            "Unknown template: {}. Available: basic, fullstack, api-only, rpc, production",
            template
        ),
    }

    // Every scaffold gets a consumer-facing AGENTS.md for coding agents.
    write_agents_md(&name, project_dir)?;

    println!("✅ Project created successfully!");
    println!();
    println!("{}", "Next steps:".bold());
    println!("  cd {}", name);
    println!("  cargo run");
    println!();
    println!("📚 Learn more: https://docs.ultimo.dev");

    Ok(())
}

fn create_basic_template(name: &str, project_dir: &Path) -> Result<()> {
    println!("📝 Setting up basic REST API template...");

    // Create project structure
    fs::create_dir_all(project_dir.join("src"))?;

    // Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
ultimo = "{ultimo}"
tokio = {{ version = "1.35", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
"#,
        name,
        ultimo = ultimo_dep_version(),
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

    // main.rs
    let main_rs = r#"use ultimo::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[tokio::main]
async fn main() {
    let mut app = Ultimo::new();

    // Add CORS middleware
    app.use_middleware(ultimo::middleware::builtin::cors());

    // Routes
    app.get("/", |ctx: Context| async move {
        ctx.text("Welcome to Ultimo! 🚀").await
    });
    
    app.get("/users", |ctx: Context| async move {
        let users = vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            User {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        ];
        ctx.json(users).await
    });
    
    app.get("/users/:id", |ctx: Context| async move {
        let id = ctx.req.param("id")?;
        let user = User {
            id: id.parse().unwrap_or(1),
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        };
        ctx.json(user).await
    });
    
    println!("🚀 Server running on http://localhost:3000");
    println!("📝 Endpoints:");
    println!("  GET  /");
    println!("  GET  /users");
    println!("  GET  /users/:id");
    app.listen("127.0.0.1:3000").await.unwrap();
}
"#;
    fs::write(project_dir.join("src/main.rs"), main_rs)?;

    // .gitignore
    let gitignore = r#"# Rust
/target/
Cargo.lock

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
"#;
    fs::write(project_dir.join(".gitignore"), gitignore)?;

    // README.md
    let readme = format!(
        r#"# {}

A REST API built with [Ultimo](https://ultimo.dev).

## Getting Started

```bash
# Run the server
cargo run

# Visit http://localhost:3000
```

## API Endpoints

- `GET /` - Welcome message
- `GET /users` - List all users
- `GET /users/:id` - Get user by ID

## Learn More

- [Ultimo Documentation](https://docs.ultimo.dev)
- [Examples](https://github.com/ultimo-rs/ultimo/tree/main/examples)

## Next Steps

Ready for more? Check out these examples:
- `--template production` - Production-ready API with full CRUD + OpenAPI
- [examples/openapi-demo](https://github.com/ultimo-rs/ultimo/tree/main/examples/openapi-demo) - Complete OpenAPI implementation
- [examples/database-*](https://github.com/ultimo-rs/ultimo/tree/main/examples) - Real database persistence
"#,
        name
    );
    fs::write(project_dir.join("README.md"), readme)?;

    Ok(())
}

fn create_fullstack_template(name: &str, project_dir: &Path) -> Result<()> {
    println!("📝 Setting up fullstack template with RPC...");

    // Create backend structure
    fs::create_dir_all(project_dir.join("backend/src"))?;
    fs::create_dir_all(project_dir.join("frontend/src"))?;

    // Backend Cargo.toml
    let backend_cargo = format!(
        r#"[package]
name = "{}-backend"
version = "0.1.0"
edition = "2021"

[dependencies]
ultimo = "{ultimo}"
tokio = {{ version = "1.35", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
ts-rs = "{tsrs}"
"#,
        name,
        ultimo = ultimo_dep_version(),
        tsrs = TS_RS_DEP,
    );
    fs::write(project_dir.join("backend/Cargo.toml"), backend_cargo)?;

    // Backend main.rs with REST and RPC endpoints
    let backend_main = r#"use ultimo::prelude::*;
use ts_rs::TS;
use std::sync::{Arc, Mutex};

// REST-style models
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUserInput {
    name: String,
    email: String,
}

// RPC-style models with TypeScript generation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
struct UserRpc {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
struct CreateUserRpcRequest {
    name: String,
    email: String,
}

type UserStore = Arc<Mutex<Vec<User>>>;
type RpcUserStore = Arc<Mutex<Vec<UserRpc>>>;

#[tokio::main]
async fn main() {
    // Initialize shared stores
    let users: UserStore = Arc::new(Mutex::new(vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
        },
    ]));

    let rpc_users: RpcUserStore = Arc::new(Mutex::new(vec![
        UserRpc {
            id: 1,
            name: "Alice (RPC)".to_string(),
            email: "alice@example.com".to_string(),
        },
        UserRpc {
            id: 2,
            name: "Bob (RPC)".to_string(),
            email: "bob@example.com".to_string(),
        },
    ]));

    let mut app = Ultimo::new();
    
    // Add CORS middleware for frontend
    app.use_middleware(
        middleware::builtin::Cors::new()
            .allow_origin("http://localhost:5173")
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allow_headers(vec!["Content-Type", "Authorization"])
            .build(),
    );
    
    // REST-style endpoints
    let users_list = users.clone();
    app.get("/api/users", move |ctx: Context| {
        let users = users_list.clone();
        async move {
            let users_data = users.lock().unwrap().clone();
            ctx.json(users_data).await
        }
    });
    
    let users_create = users.clone();
    app.post("/api/users", move |ctx: Context| {
        let users = users_create.clone();
        async move {
            let input: CreateUserInput = ctx.req.json().await?;
            let new_user = {
                let mut users_data = users.lock().unwrap();
                let new_id = users_data.iter().map(|u| u.id).max().unwrap_or(0) + 1;
                let new_user = User {
                    id: new_id,
                    name: input.name,
                    email: input.email,
                };
                users_data.push(new_user.clone());
                new_user
            };
            ctx.json(new_user).await
        }
    });
    
    // RPC-style endpoints with type-safe TypeScript generation
    let rpc_users_list = rpc_users.clone();
    app.get("/rpc/users", move |ctx: Context| {
        let users = rpc_users_list.clone();
        async move {
            let users_data = users.lock().unwrap().clone();
            ctx.json(users_data).await
        }
    });
    
    let rpc_users_create = rpc_users.clone();
    app.post("/rpc/users", move |ctx: Context| {
        let users = rpc_users_create.clone();
        async move {
            let input: CreateUserRpcRequest = ctx.req.json().await?;
            let new_user = {
                let mut users_data = users.lock().unwrap();
                let new_id = users_data.iter().map(|u| u.id).max().unwrap_or(0) + 1;
                let new_user = UserRpc {
                    id: new_id,
                    name: input.name,
                    email: input.email,
                };
                users_data.push(new_user.clone());
                new_user
            };
            ctx.json(new_user).await
        }
    });
    
    println!("🚀 Backend running on http://localhost:3001");
    println!("📝 REST endpoints: /api/*");
    println!("📝 RPC endpoints: /rpc/* (generate TS with: ultimo generate -o ./client)");
    println!("💡 Users are stored in memory - restart to reset");
    app.listen("127.0.0.1:3001").await.unwrap();
}
"#;
    fs::write(project_dir.join("backend/src/main.rs"), backend_main)?;

    // Frontend package.json
    let frontend_package = format!(
        r#"{{
  "name": "{}-frontend",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "dev": "vite",
    "build": "vite build"
  }},
  "dependencies": {{
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  }},
  "devDependencies": {{
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@vitejs/plugin-react": "^4.0.0",
    "typescript": "^5.0.0",
    "vite": "^5.0.0"
  }}
}}
"#,
        name
    );
    fs::write(project_dir.join("frontend/package.json"), frontend_package)?;

    // Frontend index.html
    let frontend_html = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Ultimo Fullstack App</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#;
    fs::write(project_dir.join("frontend/index.html"), frontend_html)?;

    // Frontend vite.config.ts
    let vite_config = r#"import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': 'http://localhost:3001'
    }
  }
})
"#;
    fs::write(project_dir.join("frontend/vite.config.ts"), vite_config)?;

    // Frontend main.tsx
    let frontend_main = r#"import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
"#;
    fs::write(project_dir.join("frontend/src/main.tsx"), frontend_main)?;

    // Frontend App.tsx
    let frontend_app = r#"import { useState, useEffect } from 'react'

interface User {
  id: number
  name: string
  email: string
}

function App() {
  const [restUsers, setRestUsers] = useState<User[]>([])
  const [rpcUsers, setRpcUsers] = useState<User[]>([])
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [useRpc, setUseRpc] = useState(false)

  useEffect(() => {
    fetchUsers()
  }, [useRpc])

  const fetchUsers = async () => {
    const endpoint = useRpc ? '/rpc/users' : '/api/users'
    const response = await fetch(`http://localhost:3001${endpoint}`)
    const data = await response.json()
    
    if (useRpc) {
      setRpcUsers(data)
    } else {
      setRestUsers(data)
    }
  }

  const createUser = async (e: React.FormEvent) => {
    e.preventDefault()
    const endpoint = useRpc ? '/rpc/users' : '/api/users'
    
    await fetch(`http://localhost:3001${endpoint}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, email })
    })
    
    setName('')
    setEmail('')
    fetchUsers()
  }

  const currentUsers = useRpc ? rpcUsers : restUsers

  return (
    <div style={{ padding: '2rem', maxWidth: '800px', margin: '0 auto' }}>
      <h1>🚀 Ultimo Fullstack App</h1>
      
      <div style={{ marginBottom: '2rem', padding: '1rem', backgroundColor: '#f5f5f5', borderRadius: '8px' }}>
        <h3>Choose API Style:</h3>
        <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer' }}>
          <input
            type="checkbox"
            checked={useRpc}
            onChange={(e) => setUseRpc(e.target.checked)}
          />
          <span>Use RPC endpoints (type-safe with ts-rs)</span>
        </label>
        <p style={{ marginTop: '0.5rem', fontSize: '0.9em', color: '#666' }}>
          {useRpc 
            ? '📝 Using /rpc/* endpoints with TypeScript type generation' 
            : '🔄 Using /api/* REST endpoints'}
        </p>
      </div>
      
      <h2>Create User:</h2>
      <form onSubmit={createUser} style={{ marginBottom: '2rem' }}>
        <div style={{ marginBottom: '1rem' }}>
          <input
            type="text"
            placeholder="Name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            style={{ padding: '0.5rem', width: '100%' }}
            required
          />
        </div>
        <div style={{ marginBottom: '1rem' }}>
          <input
            type="email"
            placeholder="Email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            style={{ padding: '0.5rem', width: '100%' }}
            required
          />
        </div>
        <button type="submit" style={{ padding: '0.5rem 1rem' }}>
          Create User
        </button>
      </form>

      <h2>Users:</h2>
      <ul>
        {currentUsers.map((user) => (
          <li key={user.id}>
            <strong>{user.name}</strong> - {user.email}
          </li>
        ))}
      </ul>
    </div>
  )
}

export default App
"#;
    fs::write(project_dir.join("frontend/src/App.tsx"), frontend_app)?;

    // Root README
    let readme = format!(
        r#"# {}

A fullstack application built with [Ultimo](https://ultimo.dev) demonstrating both REST and RPC approaches.

## Project Structure

```
{}/
├── backend/     # Rust API with Ultimo (REST + RPC endpoints)
└── frontend/    # React frontend with Vite
```

## Getting Started

### Backend

```bash
cd backend
cargo run
```

The backend will start on http://localhost:3001

**API Endpoints:**

**REST Style:**
- `GET /api/users` - List all users
- `POST /api/users` - Create a new user

**RPC Style (with TypeScript generation):**
- `GET /rpc/users` - List all users
- `POST /rpc/users` - Create a new user

Generate TypeScript types from RPC endpoints:
```bash
cd backend
ultimo generate -o ../frontend/src/types
```

### Frontend

```bash
cd frontend
npm install
npm run dev
```

Frontend will start on http://localhost:5173

## Two API Approaches

This template demonstrates two ways to build APIs with Ultimo:

### 1. REST API (`/api/*`)
Traditional REST endpoints - simple and familiar.

```rust
app.get("/api/users", |ctx: Context| async move {{
    let users = get_users();
    ctx.json(&users).await
}});
```

### 2. RPC API (`/rpc/*`)
Type-safe endpoints with automatic TypeScript generation using `ts-rs`.

```rust
#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
struct User {{
    id: u32,
    name: String,
}}

app.get("/rpc/users", |ctx: Context| async move {{
    let users = get_users();
    ctx.json(&users).await
}});
```

The frontend can toggle between both approaches to see them in action!

## Features

- 🚀 Fast Rust backend with Ultimo
- ⚡ React + TypeScript frontend
- 🔄 REST API endpoints
- 📝 RPC endpoints with type-safe TypeScript generation
- 🎨 Toggle between API styles in the UI
- 🔥 Hot reload for development

## Learn More

- [Ultimo Documentation](https://docs.ultimo.dev)
- [ts-rs Documentation](https://github.com/Aleph-Alpha/ts-rs)

## Next Steps

Ready for more advanced patterns?
- `ultimo new my-app --template production` - Production API with full CRUD + OpenAPI
- [examples/react-app-rest](https://github.com/ultimo-rs/ultimo/tree/main/examples/react-app-rest) - Complete React + Ultimo app
- [examples/openapi-demo](https://github.com/ultimo-rs/ultimo/tree/main/examples/openapi-demo) - OpenAPI specification
- [examples/database-*](https://github.com/ultimo-rs/ultimo/tree/main/examples) - Database integration
"#,
        name, name
    );
    fs::write(project_dir.join("README.md"), readme)?;

    // .gitignore
    let gitignore = r#"# Rust
/backend/target/
backend/Cargo.lock

# Node
/frontend/node_modules/
/frontend/dist/

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
"#;
    fs::write(project_dir.join(".gitignore"), gitignore)?;

    Ok(())
}

fn create_api_template(name: &str, project_dir: &Path) -> Result<()> {
    println!("📝 Setting up API-only template with OpenAPI...");

    // Similar to basic but with OpenAPI
    create_basic_template(name, project_dir)?;

    // Add OpenAPI-specific code to main.rs
    let main_rs = r#"use ultimo::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[tokio::main]
async fn main() {
    let mut app = Ultimo::new();
    
    // Add CORS middleware
    app.use_middleware(ultimo::middleware::builtin::cors());
    
    // Routes
    app.get("/", |ctx: Context| async move {
        ctx.text("API Server - Visit /users for data").await
    });
    
    app.get("/users", |ctx: Context| async move {
        let users = vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            User {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        ];
        ctx.json(users).await
    });
    
    println!("🚀 API Server running on http://localhost:3000");
    println!("📝 Endpoints:");
    println!("  GET  /");
    println!("  GET  /users");
    app.listen("127.0.0.1:3000").await.unwrap();
}
"#;
    fs::write(project_dir.join("src/main.rs"), main_rs)?;

    Ok(())
}

fn create_rpc_template(name: &str, project_dir: &Path) -> Result<()> {
    println!("📝 Setting up RPC template with type-safe client generation...");

    // Create project structure
    fs::create_dir_all(project_dir.join("src/bin"))?;

    // Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
ultimo = {{ version = "{ultimo}", features = ["client-gen"] }}
tokio = {{ version = "1.35", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
ts-rs = "{tsrs}"
"#,
        name,
        ultimo = ultimo_dep_version(),
        tsrs = TS_RS_DEP,
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

    // src/api.rs — the single source of truth for the RPC surface. Both the
    // server (main.rs) and the client generator (src/bin/generate-client.rs)
    // build this same registry, so the typed client can never drift from the API.
    let api_rs = r#"//! RPC surface: types + the registry. `main.rs` mounts it; the
//! `generate-client` binary rebuilds it to emit the typed TypeScript client.
//! Add procedures here, then run `ultimo generate -o ./client.ts`.
use serde::{Deserialize, Serialize};
use ultimo::rpc::{RpcRegistry, TS};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct GetUserInput {
    pub id: u32,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct CreateUserInput {
    pub name: String,
    pub email: String,
}

/// Build the RPC registry. Each `query`/`mutation` becomes a typed method on the
/// generated TypeScript client.
pub fn registry() -> RpcRegistry {
    let rpc = RpcRegistry::new();

    // Query: read-only. Input and output types are derived into TypeScript.
    rpc.query("getUser", |input: GetUserInput| async move {
        // Replace with your real data source.
        Ok(User {
            id: input.id,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        })
    });

    // Mutation: writes. Same typed pipeline.
    rpc.mutation("createUser", |input: CreateUserInput| async move {
        Ok(User {
            id: 3,
            name: input.name,
            email: input.email,
        })
    });

    rpc
}
"#;
    fs::write(project_dir.join("src/api.rs"), api_rs)?;

    // main.rs — mounts the registry as a JSON-RPC 2.0 endpoint at POST /rpc.
    let main_rs = r#"mod api;

use ultimo::prelude::*;

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    let rpc = api::registry();

    let mut app = Ultimo::new();
    app.use_middleware(ultimo::middleware::builtin::cors());

    // Single JSON-RPC 2.0 endpoint: every procedure dispatches through POST /rpc
    // (supports single calls, batches, and notifications).
    let handler = rpc.clone();
    app.post("/rpc", move |ctx: Context| {
        let rpc = handler.clone();
        async move {
            let body = ctx.req.bytes().await?;
            let output = rpc.handle_request(&body).await;
            match output.into_body() {
                Some(bytes) => {
                    let value: serde_json::Value = serde_json::from_slice(&bytes)
                        .map_err(|e| UltimoError::Internal(e.to_string()))?;
                    ctx.json(value).await
                }
                None => {
                    // A notification (no id) produces no response body.
                    ctx.status(204).await;
                    ctx.text("").await
                }
            }
        }
    });

    println!("🚀 Ultimo RPC server on http://127.0.0.1:3000  (POST /rpc)");
    println!("📝 Regenerate the typed client: ultimo generate -o ./client.ts");
    app.listen("127.0.0.1:3000").await
}
"#;
    fs::write(project_dir.join("src/main.rs"), main_rs)?;

    // src/bin/generate-client.rs — the real generator `ultimo generate` runs.
    // It rebuilds the same registry via `#[path]` and writes the typed client to
    // the path given as its first argument (the convention `ultimo generate` expects).
    let generate_client = r#"//! Typed TypeScript client generator. Run via `ultimo generate -o ./client.ts`,
//! or directly: `cargo run --bin generate-client -- ./client.ts`.
#[path = "../api.rs"]
mod api;

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "client.ts".to_string());

    api::registry()
        .generate_client_file(&out)
        .expect("failed to write TypeScript client");

    println!("✅ TypeScript client generated: {out}");
}
"#;
    fs::write(
        project_dir.join("src/bin/generate-client.rs"),
        generate_client,
    )?;

    // README.md
    let readme = format!(
        r#"# {}

Type-safe JSON-RPC API with a generated TypeScript client.

## How it works

`src/api.rs` is the single source of truth: it defines the request/response types
and builds the `RpcRegistry`. The server (`src/main.rs`) mounts that registry at
`POST /rpc`, and `src/bin/generate-client.rs` rebuilds the same registry to emit a
fully typed TypeScript client — so the client can never drift from the API.

## Getting started

1. Run the server:
```bash
cargo run
```

2. Regenerate the typed client whenever `src/api.rs` changes:
```bash
ultimo generate -o ./client.ts
```

3. Call it from your frontend:
```typescript
import {{ UltimoRpcClient }} from './client';

const client = new UltimoRpcClient('http://localhost:3000/rpc');
const user = await client.getUser({{ id: 1 }});     // typed
await client.createUser({{ name: 'Ada', email: 'ada@example.com' }});
```

## Adding a procedure

Add a `query` (read) or `mutation` (write) in `src/api.rs::registry()`, deriving
`TS` on its input/output types, then re-run `ultimo generate`.

## Learn more

- Ultimo docs: https://docs.ultimo.dev  ·  RPC guide: https://docs.ultimo.dev/rpc
- For coding agents: see `AGENTS.md` in this project.
"#,
        name
    );
    fs::write(project_dir.join("README.md"), readme)?;

    // .gitignore
    let gitignore = r#"target/
Cargo.lock
.env
*.profraw
*.profdata
"#;
    fs::write(project_dir.join(".gitignore"), gitignore)?;

    Ok(())
}

fn create_production_template(name: &str, project_dir: &Path) -> Result<()> {
    println!("📝 Setting up production-ready REST API with OpenAPI...");

    // Create project structure
    fs::create_dir_all(project_dir.join("src"))?;

    // Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
ultimo = "{ultimo}"
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
"#,
        name,
        ultimo = ultimo_dep_version(),
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

    // main.rs with full CRUD + OpenAPI from examples/openapi-demo
    let main_rs = r#"use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use ultimo::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUserInput {
    name: String,
    email: String,
}

type UserStore = Arc<Mutex<Vec<User>>>;

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    println!("🚀 Production REST API with OpenAPI");
    println!();

    // Initialize user store with sample data
    let users: UserStore = Arc::new(Mutex::new(vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
        },
    ]));

    let mut app = Ultimo::new();

    // Add CORS middleware
    app.use_middleware(ultimo::middleware::builtin::cors());

    // Add logger middleware
    app.use_middleware(ultimo::middleware::builtin::logger());

    // GET /api/users/:id - Get user by ID
    let users_get = users.clone();
    app.get("/api/users/:id", move |ctx: Context| {
        let users = users_get.clone();
        async move {
            let id_str = ctx.req.param("id")?;
            let id: u32 = id_str
                .parse()
                .map_err(|_| UltimoError::BadRequest("Invalid 'id' parameter".to_string()))?;

            let user = {
                let users_data = users.lock().unwrap();
                users_data.iter().find(|u| u.id == id).cloned()
            };

            let user = user.ok_or_else(|| UltimoError::NotFound("User not found".to_string()))?;

            ctx.json(user).await
        }
    });

    // GET /api/users - List all users
    let users_list = users.clone();
    app.get("/api/users", move |ctx: Context| {
        let users = users_list.clone();
        async move {
            let users_data = users.lock().unwrap().clone();
            ctx.json(users_data).await
        }
    });

    // POST /api/users - Create new user
    let users_create = users.clone();
    app.post("/api/users", move |ctx: Context| {
        let users = users_create.clone();
        async move {
            let input: CreateUserInput = ctx.req.json().await?;
            let new_user = {
                let mut users_data = users.lock().unwrap();
                let new_id = users_data.iter().map(|u| u.id).max().unwrap_or(0) + 1;
                let new_user = User {
                    id: new_id,
                    name: input.name,
                    email: input.email,
                };
                users_data.push(new_user.clone());
                new_user
            };
            ctx.json(new_user).await
        }
    });

    // DELETE /api/users/:id - Delete user
    let users_delete = users.clone();
    app.delete("/api/users/:id", move |ctx: Context| {
        let users = users_delete.clone();
        async move {
            let id: u32 = ctx
                .req
                .param("id")?
                .parse()
                .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;

            {
                let mut users_data = users.lock().unwrap();
                let index = users_data
                    .iter()
                    .position(|u| u.id == id)
                    .ok_or_else(|| UltimoError::NotFound("User not found".to_string()))?;
                users_data.remove(index);
            }

            ctx.status(204).await;
            ctx.text("").await
        }
    });

    // Generate OpenAPI specification
    use std::collections::HashMap;
    use ultimo::openapi::{
        MediaType, OpenApiBuilder, Operation, Parameter, ParameterLocation, PathItem, RequestBody,
        Response, Schema,
    };

    println!("📋 Generating OpenAPI specification...");
    let mut openapi = OpenApiBuilder::new()
        .title("User API")
        .version("1.0.0")
        .description("Production-ready user management API with full CRUD operations")
        .server(
            "http://127.0.0.1:3000",
            Some("Development server".to_string()),
        )
        .tag("users", Some("User management operations".to_string()))
        .build();

    // Add OpenAPI paths (simplified - see full implementation in examples/openapi-demo)
    // You can expand this with detailed schemas for all operations

    println!("🌐 Server running on http://127.0.0.1:3000");
    println!("📚 API Endpoints:");
    println!("  GET    /api/users     - List all users");
    println!("  GET    /api/users/:id - Get user by ID");
    println!("  POST   /api/users     - Create new user");
    println!("  DELETE /api/users/:id - Delete user");
    println!();
    println!("💡 See examples/openapi-demo for full OpenAPI spec generation");
    
    app.listen("127.0.0.1:3000").await?;
    Ok(())
}
"#;
    fs::write(project_dir.join("src/main.rs"), main_rs)?;

    // README.md
    let readme = format!(
        r#"# {}

Production-ready REST API built with [Ultimo](https://github.com/ultimo-rs/ultimo).

## Features

- ✅ Full CRUD operations (GET, POST, DELETE)
- ✅ OpenAPI specification generation
- ✅ CORS middleware
- ✅ Request logging
- ✅ Thread-safe state with Arc<Mutex>
- ✅ Proper error handling

## Quick Start

```bash
# Run the server
cargo run

# Test endpoints
curl http://localhost:3000/api/users
curl http://localhost:3000/api/users/1
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{{"name":"Charlie","email":"charlie@example.com"}}'
curl -X DELETE http://localhost:3000/api/users/1
```

## Project Structure

```
{}
├── Cargo.toml          # Dependencies
└── src/
    └── main.rs         # API server with CRUD operations
```

## API Endpoints

- `GET /api/users` - List all users
- `GET /api/users/:id` - Get user by ID  
- `POST /api/users` - Create new user
- `DELETE /api/users/:id` - Delete user

## Learn More

This template is based on the production patterns from Ultimo examples:
- [examples/openapi-demo](https://github.com/ultimo-rs/ultimo/tree/main/examples/openapi-demo) - Full OpenAPI implementation
- [examples/react-app-rest](https://github.com/ultimo-rs/ultimo/tree/main/examples/react-app-rest) - Frontend integration
- [examples/database-*](https://github.com/ultimo-rs/ultimo/tree/main/examples) - Database persistence

### Next Steps

1. **Add more endpoints** - Expand with PUT/PATCH operations
2. **Complete OpenAPI spec** - See `examples/openapi-demo/src/rest-server.rs` for full implementation
3. **Add database** - Replace in-memory storage with PostgreSQL/SQLite
4. **Add authentication** - Implement JWT or session-based auth
5. **Add validation** - Use validator crate for input validation

## Documentation

- [Ultimo Documentation](https://docs.ultimo.dev)
- [API Examples](https://github.com/ultimo-rs/ultimo/tree/main/examples)
"#,
        name, name
    );
    fs::write(project_dir.join("README.md"), readme)?;

    // .gitignore
    let gitignore = r#"target/
Cargo.lock
.env
*.profraw
*.profdata
"#;
    fs::write(project_dir.join(".gitignore"), gitignore)?;

    Ok(())
}
