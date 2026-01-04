import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Code, Loader2, Plus } from "lucide-react";
import { useState } from "react";
import { Button } from "../components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../components/ui/card";
import { Input } from "../components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "../components/ui/table";
import { UltimoRpcClient, type User } from "../lib/ultimo-client";

const rpcClient = new UltimoRpcClient("/api");

export function RpcExample() {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [showTypes, setShowTypes] = useState(false);
  const queryClient = useQueryClient();

  const { data, isLoading, error } = useQuery({
    queryKey: ["rpc-users"],
    queryFn: () => rpcClient.listUsers({}),
  });

  const createMutation = useMutation({
    mutationFn: (input: { name: string; email: string }) =>
      rpcClient.createUser({
        name: input.name,
        email: input.email,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["rpc-users"] });
      setName("");
      setEmail("");
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (name && email) {
      createMutation.mutate({ name, email });
    }
  };

  return (
    <div className="max-w-4xl mx-auto">
      <Card className="mb-8">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>RPC Client Example</CardTitle>
              <CardDescription>
                Type-safe RPC calls with auto-generated TypeScript client from
                Rust
              </CardDescription>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowTypes(!showTypes)}
            >
              <Code className="h-4 w-4 mr-2" />
              {showTypes ? "Hide" : "Show"} Types
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {showTypes && (
            <div className="bg-gray-900 text-gray-100 p-4 rounded-lg mb-6 overflow-x-auto">
              <pre className="text-xs">
                {`// Auto-generated from Rust backend
interface User {
  id: number;
  name: string;
  email: string;
}

const rpcClient = new UltimoRpcClient('/api')

// List all users
rpcClient.listUsers({}) 
  → Promise<{ users: User[]; total: number }>

// Get user by ID  
rpcClient.getUserById({ id: 1 })
  → Promise<User>

// Create new user
rpcClient.createUser({ name: "Alice", email: "alice@example.com" })
  → Promise<User>`}
              </pre>
            </div>
          )}

          <div className="bg-blue-50 border border-blue-200 text-blue-800 px-4 py-3 rounded mb-6">
            <div className="flex items-start gap-2">
              <div className="flex-shrink-0 mt-0.5">✨</div>
              <div className="text-sm">
                <strong>Type-Safe RPC:</strong> This example uses an
                auto-generated TypeScript client that provides full type safety
                and IDE autocomplete. The types are automatically synced from
                your Rust backend!
              </div>
            </div>
          </div>

          <form onSubmit={handleSubmit} className="flex gap-4 mb-6">
            <Input
              placeholder="Name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="flex-1"
            />
            <Input
              placeholder="Email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="flex-1"
            />
            <Button type="submit" disabled={createMutation.isPending}>
              {createMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Plus className="h-4 w-4" />
              )}
              Add User
            </Button>
          </form>

          {createMutation.error && (
            <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4">
              {createMutation.error.message}
            </div>
          )}

          {isLoading && (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-blue-600" />
            </div>
          )}

          {error && (
            <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
              Error: {error.message}
            </div>
          )}

          {data && (
            <>
              <div className="mb-4 text-sm text-gray-600">
                Total users: <strong>{data.total}</strong>
              </div>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>ID</TableHead>
                    <TableHead>Name</TableHead>
                    <TableHead>Email</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {data.map((user: User) => (
                    <TableRow key={user.id}>
                      <TableCell>{user.id}</TableCell>
                      <TableCell>{user.name}</TableCell>
                      <TableCell>{user.email}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </>
          )}

          {data && data.length === 0 && (
            <div className="text-center py-8 text-gray-500">
              No users yet. Add one above!
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Benefits of RPC vs REST</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid md:grid-cols-2 gap-6 text-sm">
            <div>
              <h4 className="font-semibold mb-2 text-green-700">
                ✅ RPC Advantages
              </h4>
              <ul className="space-y-1 text-gray-600">
                <li>• Full TypeScript type safety</li>
                <li>• Auto-generated client code</li>
                <li>• Single source of truth (Rust types)</li>
                <li>• Better IDE autocomplete</li>
                <li>• Compile-time error detection</li>
                <li>• No manual type definitions needed</li>
              </ul>
            </div>
            <div>
              <h4 className="font-semibold mb-2 text-blue-700">
                📡 REST Advantages
              </h4>
              <ul className="space-y-1 text-gray-600">
                <li>• Standard HTTP methods</li>
                <li>• Better caching support</li>
                <li>• More universal/compatible</li>
                <li>• Easier to debug in browser</li>
                <li>• RESTful resource modeling</li>
                <li>• Works with any HTTP client</li>
              </ul>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
