import { useMutation } from "@tanstack/react-query";
import { Loader2, Zap } from "lucide-react";
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
import { UltimoRpcClient, type JsonRpcError } from "../lib/ultimo-client";

const rpcClient = new UltimoRpcClient("/api");

interface BatchResult {
  result?: unknown;
  error?: JsonRpcError;
}

export function BatchExample() {
  const [results, setResults] = useState<BatchResult[] | null>(null);
  const [singleTime, setSingleTime] = useState<number | null>(null);
  const [batchTime, setBatchTime] = useState<number | null>(null);

  const individualMutation = useMutation({
    mutationFn: async () => {
      const start = performance.now();
      const values = await Promise.all([
        rpcClient.getUser({ id: 1 }),
        rpcClient.getUser({ id: 2 }),
        rpcClient.getUser({ id: 3 }),
        rpcClient.listUsers({}),
      ]);
      setSingleTime(performance.now() - start);
      return values;
    },
  });

  const batchMutation = useMutation({
    mutationFn: async () => {
      const start = performance.now();
      const batchValues = await rpcClient.batch([
        { method: "getUser", params: { id: 1 } },
        { method: "getUser", params: { id: 2 } },
        { method: "getUser", params: { id: 3 } },
        { method: "listUsers", params: {} },
      ]);
      setBatchTime(performance.now() - start);
      setResults(batchValues);
      return batchValues;
    },
  });

  const notifyMutation = useMutation({
    mutationFn: () =>
      rpcClient.notify("logEvent", {
        event: "batch_demo_viewed",
        timestamp: Date.now(),
      }),
  });

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Zap className="h-5 w-5 text-orange-500" />
            JSON-RPC 2.0 Batch Demo
          </CardTitle>
          <CardDescription>
            Compare individual calls vs batch requests. Batch sends all calls in
            a single HTTP request, reducing latency.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="rounded-lg border bg-card p-4">
            <h3 className="mb-2 font-semibold">Individual Calls (4 HTTP requests)</h3>
            <p className="mb-3 text-sm text-muted-foreground">
              Each call is a separate fetch() and uses 4 round trips.
            </p>
            <div className="flex flex-wrap items-center gap-4">
              <Button
                onClick={() => individualMutation.mutate()}
                disabled={individualMutation.isPending}
              >
                {individualMutation.isPending && (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                )}
                Run 4 Individual Calls
              </Button>
              {singleTime !== null && (
                <span className="text-sm text-muted-foreground">
                  ⏱️ {singleTime.toFixed(1)}ms (4 HTTP requests)
                </span>
              )}
            </div>
          </div>

          <div className="rounded-lg border border-amber-300/70 bg-amber-50 p-4">
            <h3 className="mb-2 font-semibold text-amber-800">
              Batch Call (1 HTTP request)
            </h3>
            <p className="mb-3 text-sm text-muted-foreground">
              All 4 calls are sent as one JSON array in a single round trip.
            </p>
            <div className="flex flex-wrap items-center gap-4">
              <Button
                onClick={() => batchMutation.mutate()}
                disabled={batchMutation.isPending}
                className="bg-amber-500 text-amber-950 hover:bg-amber-400"
              >
                {batchMutation.isPending && (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                )}
                Run Batch (1 Request)
              </Button>
              {batchTime !== null && (
                <span className="text-sm font-medium text-amber-800">
                  ⏱️ {batchTime.toFixed(1)}ms (1 HTTP request)
                </span>
              )}
            </div>
          </div>

          <div className="rounded-lg border bg-card p-4">
            <h3 className="mb-2 font-semibold">Notification (fire-and-forget)</h3>
            <p className="mb-3 text-sm text-muted-foreground">
              Send a notification with no response body expected.
            </p>
            <Button
              variant="outline"
              onClick={() => notifyMutation.mutate()}
              disabled={notifyMutation.isPending}
            >
              Send Notification
            </Button>
            {notifyMutation.isSuccess && (
              <span className="ml-3 text-sm text-emerald-700">✓ Sent (no response)</span>
            )}
          </div>

          {singleTime !== null && batchTime !== null && (
            <div className="rounded-lg border bg-muted/40 p-4">
              <h3 className="mb-2 font-semibold">⚡ Performance Comparison</h3>
              <div className="grid gap-3 text-sm sm:grid-cols-2">
                <div>
                  <span className="text-muted-foreground">Individual (4 requests):</span>
                  <span className="ml-2 font-mono">{singleTime.toFixed(1)}ms</span>
                </div>
                <div>
                  <span className="text-muted-foreground">Batch (1 request):</span>
                  <span className="ml-2 font-mono font-bold text-amber-700">
                    {batchTime.toFixed(1)}ms
                  </span>
                </div>
              </div>
              {batchTime < singleTime && (
                <p className="mt-2 text-sm text-emerald-700">
                  Batch was{" "}
                  <strong>{((singleTime / batchTime) * 100 - 100).toFixed(0)}% faster</strong>
                  {" "}with fewer network round trips.
                </p>
              )}
            </div>
          )}

          {results && (
            <CodeBlock
              title="Batch Response"
              language="json"
              code={JSON.stringify(results, null, 2)}
            />
          )}

          <CodeBlock
            title="TypeScript Code"
            code={`// Batch: 4 calls in 1 HTTP request
const results = await client.batch([
  { method: "getUser", params: { id: 1 } },
  { method: "getUser", params: { id: 2 } },
  { method: "getUser", params: { id: 3 } },
  { method: "listUsers", params: {} },
]);

// Notification: fire-and-forget
await client.notify("logEvent", { event: "page_view" });`}
          />
        </CardContent>
      </Card>
    </div>
  );
}
