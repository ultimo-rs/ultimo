use ultimo::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

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
    println!("🚀 Ultimo Benchmark Server");
    
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
        User {
            id: 3,
            name: "Charlie".to_string(),
            email: "charlie@example.com".to_string(),
        },
    ]));

    let mut app = Ultimo::new_without_defaults();

    // GET /api/users
    let users_get = users.clone();
    app.get("/api/users", move |ctx: Context| {
        let users = users_get.clone();
        async move {
            let users_data = users.lock().unwrap().clone();
            ctx.json(users_data).await
        }
    });

    // GET /api/users/:id
    let users_get_id = users.clone();
    app.get("/api/users/:id", move |ctx: Context| {
        let users = users_get_id.clone();
        async move {
            let id: u32 = ctx.req.param("id")?.parse()
                .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;
            
            let user = {
                let users = users.lock().unwrap();
                users.iter().find(|u| u.id == id)
                    .cloned()
                    .ok_or_else(|| UltimoError::NotFound("User not found".to_string()))?
            };
            
            ctx.json(user).await
        }
    });

    // POST /api/users
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

    // DELETE /api/users/:id
    let users_delete = users.clone();
    app.delete("/api/users/:id", move |ctx: Context| {
        let users = users_delete.clone();
        async move {
            let id: u32 = ctx.req.param("id")?.parse()
                .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;
            
            {
                let mut users_data = users.lock().unwrap();
                let index = users_data.iter().position(|u| u.id == id)
                    .ok_or_else(|| UltimoError::NotFound("User not found".to_string()))?;
                users_data.remove(index);
            }
            
            ctx.status(204).await;
            ctx.text("").await
        }
    });

    println!("🌐 Server running on http://127.0.0.1:3000");
    println!();
    
    app.listen("127.0.0.1:3000").await
}
