use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
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
async fn main() {
    println!("🚀 Axum Benchmark Server");
    
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

    let app = Router::new()
        .route("/api/users", get(get_users))
        .route("/api/users/:id", get(get_user))
        .route("/api/users", post(create_user))
        .route("/api/users/:id", delete(delete_user))
        .with_state(users);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3004")
        .await
        .unwrap();
    
    println!("🌐 Server running on http://127.0.0.1:3004");
    println!();
    
    axum::serve(listener, app).await.unwrap();
}

async fn get_users(State(users): State<UserStore>) -> Json<Vec<User>> {
    let users_data = users.lock().unwrap().clone();
    Json(users_data)
}

async fn get_user(
    Path(id): Path<u32>,
    State(users): State<UserStore>,
) -> Result<Json<User>, StatusCode> {
    let users_data = users.lock().unwrap();
    let user = users_data
        .iter()
        .find(|u| u.id == id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(user))
}

async fn create_user(
    State(users): State<UserStore>,
    Json(input): Json<CreateUserInput>,
) -> Json<User> {
    let mut users_data = users.lock().unwrap();
    let new_id = users_data.iter().map(|u| u.id).max().unwrap_or(0) + 1;
    let new_user = User {
        id: new_id,
        name: input.name,
        email: input.email,
    };
    users_data.push(new_user.clone());
    Json(new_user)
}

async fn delete_user(
    Path(id): Path<u32>,
    State(users): State<UserStore>,
) -> impl IntoResponse {
    let mut users_data = users.lock().unwrap();
    let index = users_data
        .iter()
        .position(|u| u.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;
    users_data.remove(index);
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}
