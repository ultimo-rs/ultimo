import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Code, Loader2, Plus } from "lucide-react";
import { useState } from "react";
import { CodeBlock } from "../components/code-block";
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
    <div className="space-y-6">
      <Card>
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
        <CardContent className="space-y-6">
          {showTypes && (
            <CodeBlock
              title="Auto-generated TypeScript"
              code={`// Auto-generated from Rust backend
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
            />
          )}

          <div className="rounded-lg border bg-primary/5 px-4 py-3 text-sm text-primary/90">
            <div className="flex items-start gap-2">
              <div className="flex-shrink-0 mt-0.5">✨</div>
              <div>
                <strong>Type-Safe RPC:</strong> This example uses an
                auto-generated TypeScript client that provides full type safety
                and IDE autocomplete. The types are automatically synced from
                your Rust backend!
              </div>
            </div>
          </div>

          <form
            onSubmit={handleSubmit}
            className="grid gap-4 sm:grid-cols-[1fr_1fr_auto]"
          >
            <Input
              placeholder="Name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="h-10"
            />
            <Input
              placeholder="Email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="h-10"
            />
            <Button
              type="submit"
              disabled={createMutation.isPending}
              className="h-10"
            >
              {createMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Plus className="h-4 w-4" />
              )}
              Add User
            </Button>
          </form>

          {createMutation.error && (
            <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
              {createMutation.error.message}
            </div>
          )}

          {isLoading && (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-blue-600" />
            </div>
          )}

          {error && (
            <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
              Error: {error.message}
            </div>
          )}

          {data && (
            <>
              <div className="text-sm text-muted-foreground">
                Total users: <strong>{data.total}</strong>
              </div>
              <div className="overflow-hidden rounded-lg border">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>ID</TableHead>
                      <TableHead>Name</TableHead>
                      <TableHead>Email</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {data.users.map((user: User) => (
                      <TableRow key={user.id}>
                        <TableCell>{user.id}</TableCell>
                        <TableCell>{user.name}</TableCell>
                        <TableCell>{user.email}</TableCell>
                      </TableRow>
                    ))}
                    {data.users.map((user: User) => (
                      <TableRow key={user.id}>
                        <TableCell>{user.id}</TableCell>
                        <TableCell>{user.name}</TableCell>
                        <TableCell>{user.email}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </>
          )}

          {data && data.users.length === 0 && (
            <div className="py-8 text-center text-sm text-muted-foreground">
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
          <div className="grid gap-4 text-sm md:grid-cols-2">
            <div className="rounded-lg border bg-emerald-50/50 p-4">
              <h4 className="mb-2 font-semibold text-emerald-700">
                ✅ RPC Advantages
              </h4>
              <ul className="space-y-1 text-muted-foreground">
                <li>• Full TypeScript type safety</li>
                <li>• Auto-generated client code</li>
                <li>• Single source of truth (Rust types)</li>
                <li>• Better IDE autocomplete</li>
                <li>• Compile-time error detection</li>
                <li>• No manual type definitions needed</li>
              </ul>
            </div>
            <div className="rounded-lg border bg-sky-50/50 p-4">
              <h4 className="mb-2 font-semibold text-sky-700">
                📡 REST Advantages
              </h4>
              <ul className="space-y-1 text-muted-foreground">
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
