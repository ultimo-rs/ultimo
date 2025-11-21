import { Hono } from "hono";

const app = new Hono();

// In-memory user store
const users = [
  { id: 1, name: "Alice", email: "alice@example.com" },
  { id: 2, name: "Bob", email: "bob@example.com" },
  { id: 3, name: "Charlie", email: "charlie@example.com" },
];

// GET /api/users
app.get("/api/users", (c) => {
  return c.json(users);
});

// GET /api/users/:id
app.get("/api/users/:id", (c) => {
  const id = parseInt(c.req.param("id"));
  const user = users.find((u) => u.id === id);

  if (!user) {
    return c.json({ error: "User not found" }, 404);
  }

  return c.json(user);
});

// POST /api/users
app.post("/api/users", async (c) => {
  const body = await c.req.json();
  const newId = Math.max(...users.map((u) => u.id), 0) + 1;
  const newUser = {
    id: newId,
    name: body.name,
    email: body.email,
  };
  users.push(newUser);
  return c.json(newUser);
});

// DELETE /api/users/:id
app.delete("/api/users/:id", (c) => {
  const id = parseInt(c.req.param("id"));
  const index = users.findIndex((u) => u.id === id);

  if (index === -1) {
    return c.json({ error: "User not found" }, 404);
  }

  users.splice(index, 1);
  return c.body(null, 204);
});

console.log("🚀 Hono Benchmark Server (Bun)");
console.log("🌐 Server running on http://localhost:3002");
console.log();

export default {
  port: 3002,
  fetch: app.fetch,
};
