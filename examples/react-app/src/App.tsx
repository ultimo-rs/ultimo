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
    <div className="min-h-screen bg-gray-50">
      <header className="bg-white border-b">
        <div className="container mx-auto px-4 py-4">
          <div className="flex items-center justify-between">
            <h1 className="text-2xl font-bold text-blue-600">
              ⚡ Ultimo Framework
            </h1>
            <nav className="flex gap-4">
              <Link to="/rest">
                <Button variant="outline">REST API</Button>
              </Link>
              <Link to="/rpc">
                <Button variant="outline">RPC Client</Button>
              </Link>
            </nav>
          </div>
        </div>
      </header>

      <main className="container mx-auto px-4 py-8">
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
    <div className="max-w-4xl mx-auto">
      <Card className="mb-8">
        <CardHeader>
          <CardTitle>Welcome to Ultimo Framework Example</CardTitle>
          <CardDescription>
            This demo showcases two ways to interact with Ultimo backend:
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <h3 className="font-semibold mb-2">🔄 REST API Example</h3>
            <p className="text-sm text-gray-600 mb-3">
              Traditional REST endpoints with TanStack Query for data fetching,
              caching, and mutations.
            </p>
            <Link to="/rest">
              <Button>View REST Example</Button>
            </Link>
          </div>

          <div className="border-t pt-4">
            <h3 className="font-semibold mb-2">⚡ RPC Client Example</h3>
            <p className="text-sm text-gray-600 mb-3">
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
