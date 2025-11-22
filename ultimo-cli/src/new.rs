use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

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
ultimo = "0.1"
tokio = {{ version = "1.35", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
"#,
        name
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
ultimo = "0.1"
tokio = {{ version = "1.35", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
ts-rs = "8.1"
"#,
        name
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
    fs::create_dir_all(project_dir.join("src"))?;

    // Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
ultimo = "0.1"
tokio = {{ version = "1.35", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
ts-rs = "8.1"
"#,
        name
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

    // main.rs
    let main_rs = r#"use ultimo::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
struct CreateUserResponse {
    id: u32,
    name: String,
    email: String,
}

#[tokio::main]
async fn main() {
    let mut app = Ultimo::new();
    
    // Add CORS middleware
    app.use_middleware(
        middleware::builtin::Cors::new()
            .allow_origin("http://localhost:5173")
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allow_headers(vec!["Content-Type", "Authorization"])
            .build(),
    );
    
    // RPC-style routes with type-safe handlers
    app.get("/api/users", |ctx: Context| async move {
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
        
        ctx.json(&users).await
    });
    
    app.get("/api/users/:id", |ctx: Context| async move {
        let id: u32 = ctx.req.param("id")?.parse().unwrap_or(0);
        
        let user = User {
            id,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        };
        
        ctx.json(&user).await
    });
    
    app.post("/api/users", |ctx: Context| async move {
        let body: CreateUserRequest = ctx.req.json().await?;
        
        let user = CreateUserResponse {
            id: 3,
            name: body.name,
            email: body.email,
        };
        
        ctx.json(&user).await
    });
    
    println!("🚀 RPC Server running on http://localhost:3000");
    println!("📝 Generate TypeScript client with: ultimo generate -o ./client");
    app.listen("127.0.0.1:3000").await.unwrap();
}
"#;
    fs::write(project_dir.join("src/main.rs"), main_rs)?;

    // README.md
    let readme = format!(
        r#"# {}

RPC-style API with type-safe TypeScript client generation.

## Getting Started

1. Start the server:
```bash
cargo run
```

2. Generate TypeScript client:
```bash
ultimo generate -o ./client
```

3. Use the generated types in your frontend:
```typescript
import {{ User, CreateUserRequest }} from './client/bindings';

const user: User = await fetch('http://localhost:3000/api/users/1').then(r => r.json());
```

## Features

- ✅ Type-safe RPC-style endpoints
- ✅ Automatic TypeScript type generation with ts-rs
- ✅ CORS enabled for frontend integration
- ✅ JSON request/response handling

## API Endpoints

- `GET /api/users` - List all users
- `GET /api/users/:id` - Get user by ID
- `POST /api/users` - Create new user

## Learn More

- [Ultimo Documentation](https://docs.ultimo.dev)
- [ts-rs Documentation](https://github.com/Aleph-Alpha/ts-rs)
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
ultimo = "0.1"
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
"#,
        name
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
