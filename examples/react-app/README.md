# Ultimo Framework - React Example

This example demonstrates how to build a full-stack application with:

- **Backend**: Ultimo framework (Rust)
- **Frontend**: React + TypeScript + Vite
- **UI**: shadcn/ui components
- **Data Fetching**: TanStack Query

## Features Demonstrated

### REST API Example

- ✅ CRUD operations (Create, Read, Delete)
- ✅ TanStack Query for data fetching and caching
- ✅ Optimistic updates and automatic refetching
- ✅ Form validation with error handling
- ✅ Loading and error states
- ✅ Beautiful UI with shadcn/ui components

### RPC Client Example (Coming Soon)

- 🚧 Type-safe RPC calls
- 🚧 Auto-generated TypeScript types from Rust
- 🚧 Single source of truth for API contracts

## Running the Example

### 1. Start the Rust Backend

```bash
cd examples/react-backend
cargo run --release
```

The backend will start on `http://localhost:3000`

### 2. Start the React Frontend

In a new terminal:

```bash
cd examples/react-app
npm install
npm run dev
```

The frontend will start on `http://localhost:5173`

### 3. Open in Browser

Navigate to `http://localhost:5173` and explore:

- REST API example at `/rest`
- RPC example at `/rpc` (coming soon)

## Project Structure

```
examples/
├── react-backend/          # Ultimo Rust backend
│   ├── src/
│   │   └── main.rs        # REST API endpoints
│   └── Cargo.toml
│
└── react-app/              # React frontend
    ├── src/
    │   ├── components/
    │   │   └── ui/        # shadcn/ui components
    │   ├── pages/
    │   │   ├── RestExample.tsx
    │   │   └── RpcExample.tsx
    │   ├── App.tsx
    │   └── main.tsx
    ├── package.json
    └── vite.config.ts
```

## API Endpoints

### Users

- `GET /users` - Get all users
- `GET /users/:id` - Get user by ID
- `POST /users` - Create new user
  ```json
  {
    "name": "Alice",
    "email": "alice@example.com"
  }
  ```
- `DELETE /users/:id` - Delete user

## Technologies Used

### Backend

- **Ultimo**: Modern Rust web framework
- **Tokio**: Async runtime
- **Serde**: JSON serialization
- **Validator**: Request validation

### Frontend

- **React 18**: UI framework
- **TypeScript**: Type safety
- **Vite**: Build tool
- **TanStack Query**: Server state management
- **React Router**: Navigation
- **shadcn/ui**: Beautiful UI components
- **Tailwind CSS**: Styling
- **Lucide React**: Icons

## Next Steps

1. Try adding more CRUD operations (Update)
2. Add pagination and filtering
3. Implement authentication
4. Explore the upcoming RPC example with type-safe communication
