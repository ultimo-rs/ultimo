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
        _ => anyhow::bail!(
            "Unknown template: {}. Available: basic, fullstack, api-only",
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
    let main_rs = r#"use ultimo::{Ultimo, Context, Router};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

async fn get_users(_c: Context) -> Result<String, Box<dyn std::error::Error>> {
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
    
    Ok(serde_json::to_string(&users)?)
}

async fn get_user(c: Context) -> Result<String, Box<dyn std::error::Error>> {
    let id = c.req.param("id").unwrap_or("0");
    
    let user = User {
        id: id.parse().unwrap_or(0),
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    
    Ok(serde_json::to_string(&user)?)
}

#[tokio::main]
async fn main() {
    let mut app = Ultimo::new();
    
    // Routes
    app.get("/", |c: Context| async move {
        c.text("Welcome to Ultimo! 🚀")
    });
    
    app.get("/users", get_users);
    app.get("/users/:id", get_user);
    
    // Start server
    println!("🚀 Server running on http://localhost:3000");
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
"#,
        name
    );
    fs::write(project_dir.join("backend/Cargo.toml"), backend_cargo)?;

    // Backend main.rs with REST API
    let backend_main = r#"use ultimo::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
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

#[tokio::main]
async fn main() {
    let mut app = Ultimo::new();
    
    // Add CORS middleware for frontend
    app.use_middleware(
        middleware::builtin::Cors::new()
            .allow_origin("http://localhost:5173")
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .build(),
    );
    
    // Routes
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
    
    app.post("/api/users", |ctx: Context| async move {
        let body = ctx.req.json::<CreateUserInput>().await?;
        let user = User {
            id: 3,
            name: body.name,
            email: body.email,
        };
        ctx.json(&user).await
    });
    
    println!("🚀 Backend running on http://localhost:3001");
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
  const [users, setUsers] = useState<User[]>([])
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')

  useEffect(() => {
    fetchUsers()
  }, [])

  const fetchUsers = async () => {
    const response = await fetch('http://localhost:3001/api/users')
    const data = await response.json()
    setUsers(data)
  }

  const createUser = async (e: React.FormEvent) => {
    e.preventDefault()
    await fetch('http://localhost:3001/api/users', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, email })
    })
    setName('')
    setEmail('')
    fetchUsers()
  }

  return (
    <div style={{ padding: '2rem', maxWidth: '600px', margin: '0 auto' }}>
      <h1>🚀 Ultimo Fullstack App</h1>
      
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
        {users.map((user) => (
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

A fullstack application built with [Ultimo](https://ultimo.dev).

## Project Structure

```
{}/
├── backend/     # Rust REST API with Ultimo
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
- `GET /api/users` - List all users
- `POST /api/users` - Create a new user

### Frontend

```bash
cd frontend
npm install
npm run dev
```

Frontend will start on http://localhost:5173

## Features

- 🚀 Fast Rust backend with Ultimo
- ⚡ React + TypeScript frontend
- 🔄 Hot reload for both frontend and backend
- 📦 Production-ready setup
- 🎨 Modern development experience

## Learn More

- [Ultimo Documentation](https://docs.ultimo.dev)
- [Examples](https://github.com/ultimo-rs/ultimo/tree/main/examples)
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
    let main_rs = r#"use ultimo::{Ultimo, Context, OpenApi};
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
    let mut openapi = OpenApi::new("My API", "1.0.0");
    
    // Routes with OpenAPI docs
    app.get("/", |c: Context| async move {
        c.text("API Server - Visit /docs for OpenAPI spec")
    });
    
    app.get("/users", |_c: Context| async move {
        let users = vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
        ];
        Ok(serde_json::to_string(&users)?)
    });
    
    // Add OpenAPI documentation
    openapi.add_path("/users", "GET", "List all users");
    
    app.get("/docs", move |c: Context| {
        let spec = openapi.to_json();
        c.json(&spec)
    });
    
    println!("🚀 API Server running on http://localhost:3000");
    println!("📖 OpenAPI docs at http://localhost:3000/docs");
    app.listen("127.0.0.1:3000").await.unwrap();
}
"#;
    fs::write(project_dir.join("src/main.rs"), main_rs)?;

    Ok(())
}
