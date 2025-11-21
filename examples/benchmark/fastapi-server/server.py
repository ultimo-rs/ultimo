from fastapi import FastAPI
from pydantic import BaseModel
from typing import List

app = FastAPI()

# In-memory user store
users = [
    {"id": 1, "name": "Alice", "email": "alice@example.com"},
    {"id": 2, "name": "Bob", "email": "bob@example.com"},
    {"id": 3, "name": "Charlie", "email": "charlie@example.com"},
]


class User(BaseModel):
    id: int
    name: str
    email: str


class CreateUserInput(BaseModel):
    name: str
    email: str


@app.get("/api/users", response_model=List[User])
async def get_users():
    return users


@app.get("/api/users/{user_id}", response_model=User)
async def get_user(user_id: int):
    user = next((u for u in users if u["id"] == user_id), None)
    if not user:
        return {"error": "User not found"}, 404
    return user


@app.post("/api/users", response_model=User)
async def create_user(input: CreateUserInput):
    new_id = max(u["id"] for u in users) + 1 if users else 1
    new_user = {"id": new_id, "name": input.name, "email": input.email}
    users.append(new_user)
    return new_user


@app.delete("/api/users/{user_id}", status_code=204)
async def delete_user(user_id: int):
    global users
    user_index = next((i for i, u in enumerate(users) if u["id"] == user_id), None)
    if user_index is None:
        return {"error": "User not found"}, 404
    users.pop(user_index)
    return None


if __name__ == "__main__":
    import uvicorn
    
    print("🚀 FastAPI Benchmark Server")
    print("🌐 Server running on http://localhost:3003")
    print()
    
    uvicorn.run(app, host="0.0.0.0", port=3003, log_level="error")
