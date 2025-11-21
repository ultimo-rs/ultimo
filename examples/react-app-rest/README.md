# Ultimo REST API Example (React)

This is a full-stack example demonstrating Ultimo Framework's REST API with a React frontend.

## Features

- **Traditional REST Architecture**: Resource-based endpoints (`/api/users`, `/api/users/:id`)
- **TanStack Query**: Server state management with caching and optimistic updates
- **shadcn/ui**: Beautiful, accessible UI components
- **TypeScript**: Full type safety across frontend and backend
- **Form Validation**: Client-side validation with error handling

## Prerequisites

1. **Backend Server**: Run the REST server first

   ```bash
   cd ../../
   cargo run --bin rest-server
   ```

   Server will start at `http://localhost:3000`

2. **Node.js**: Version 18 or higher

## Getting Started

```bash
# Install dependencies
npm install

# Start development server (runs on port 5173)
npm run dev
```

Open [http://localhost:5173](http://localhost:5173) to view the app.

## API Endpoints Used

- `GET /api/users` - List all users
- `GET /api/users/:id` - Get user by ID
- `POST /api/users` - Create new user
- `PUT /api/users/:id` - Update existing user
- `DELETE /api/users/:id` - Delete user

## Project Structure

```
src/
├── App.tsx              # Main app component
├── main.tsx             # App entry point
├── pages/
│   └── RestExample.tsx  # REST API demo with CRUD operations
├── components/
│   └── ui/              # shadcn/ui components
└── lib/
    └── utils.ts         # Utility functions
```

## OpenAPI Documentation

Visit [http://localhost:3000/docs](http://localhost:3000/docs) to see the interactive Swagger UI documentation for the REST API.

## Technologies

- **React 18**: Modern React with hooks
- **TypeScript**: Static type checking
- **Vite**: Fast build tool and dev server
- **TanStack Query**: Async state management
- **shadcn/ui**: Component library built on Radix UI
- **Tailwind CSS**: Utility-first styling
