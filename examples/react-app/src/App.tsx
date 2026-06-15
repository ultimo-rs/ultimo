import { Link, Route, Routes } from "react-router-dom";
import { Button } from "./components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "./components/ui/card";
import { RestExample } from "./pages/RestExample";
import { RpcExample } from "./pages/RpcExample";

function App() {
  return (
    <div className="min-h-screen bg-gradient-to-b from-slate-100 via-white to-slate-100">
      <header className="border-b bg-background/90 backdrop-blur">
        <div className="mx-auto flex w-full max-w-5xl flex-col gap-4 px-4 py-6 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-primary">
              Ultimo Examples
            </p>
            <h1 className="mt-1 text-2xl font-bold tracking-tight sm:text-3xl">
              REST + RPC Showcase
            </h1>
          </div>
          <nav className="flex gap-3">
            <Link to="/rest">
              <Button variant="outline">REST API</Button>
            </Link>
            <Link to="/rpc">
              <Button variant="outline">RPC Client</Button>
            </Link>
          </nav>
          <p className="max-w-md text-sm text-muted-foreground">
            Unified shadcn interface demonstrating both request styles with
            colored code snippets.
          </p>
        </div>
      </header>

      <main className="mx-auto w-full max-w-5xl px-4 py-8">
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/rest" element={<RestExample />} />
          <Route path="/rpc" element={<RpcExample />} />
        </Routes>
      </main>
    </div>
  );
}

function HomePage() {
  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Welcome to Ultimo Framework Example</CardTitle>
          <CardDescription>
            This demo showcases two ways to interact with Ultimo backend:
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="rounded-lg border bg-card p-4">
            <h3 className="font-semibold mb-2">🔄 REST API Example</h3>
            <p className="mb-3 text-sm text-muted-foreground">
              Traditional REST endpoints with TanStack Query for data fetching,
              caching, and mutations.
            </p>
            <Link to="/rest">
              <Button>View REST Example</Button>
            </Link>
          </div>

          <div className="rounded-lg border bg-card p-4">
            <h3 className="font-semibold mb-2">⚡ RPC Client Example</h3>
            <p className="mb-3 text-sm text-muted-foreground">
              Type-safe RPC calls with auto-generated TypeScript client from
              Rust types.
            </p>
            <Link to="/rpc">
              <Button>View RPC Example</Button>
            </Link>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Features Demonstrated</CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="space-y-2 text-sm">
            <li>✅ TanStack Query for server state management</li>
            <li>✅ shadcn/ui components for beautiful UI</li>
            <li>✅ CRUD operations (Create, Read, Update, Delete)</li>
            <li>✅ Form validation and error handling</li>
            <li>✅ Loading and error states</li>
            <li>✅ Optimistic updates</li>
            <li>✅ Type-safe RPC with Rust → TypeScript generation</li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}

export default App;
