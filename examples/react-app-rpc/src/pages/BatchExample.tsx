import { useMutation } from "@tanstack/react-query";
import { Loader2, Zap } from "lucide-react";
import { useState } from "react";
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

  // Simulate multiple individual calls
  const individualMutation = useMutation({
    mutationFn: async () => {
      const start = performance.now();
      const results = await Promise.all([
        rpcClient.getUser({ id: 1 }),
        rpcClient.getUser({ id: 2 }),
        rpcClient.getUser({ id: 3 }),
        rpcClient.listUsers({}),
      ]);
      setSingleTime(performance.now() - start);
      return results;
    },
  });

  // Batch all calls in one HTTP request
  const batchMutation = useMutation({
    mutationFn: async () => {
      const start = performance.now();
      const batchResults = await rpcClient.batch([
        { method: "getUser", params: { id: 1 } },
        { method: "getUser", params: { id: 2 } },
        { method: "getUser", params: { id: 3 } },
        { method: "listUsers", params: {} },
      ]);
      setBatchTime(performance.now() - start);
      setResults(batchResults);
      return batchResults;
    },
  });

  // Fire-and-forget notification
  const notifyMutation = useMutation({
    mutationFn: () =>
      rpcClient.notify("logEvent", {
        event: "batch_demo_viewed",
        timestamp: Date.now(),
      }),
  });

  return (
    <div className="max-w-4xl mx-auto space-y-6">
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
          {/* Individual calls */}
          <div className="border rounded-lg p-4">
            <h3 className="font-semibold mb-2">
              Individual Calls (4 HTTP requests)
            </h3>
            <p className="text-sm text-gray-500 mb-3">
              Each call is a separate fetch() — 4 round trips to the server.
            </p>
            <div className="flex items-center gap-4">
              <Button
                onClick={() => individualMutation.mutate()}
                disabled={individualMutation.isPending}
              >
                {individualMutation.isPending && (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                )}
                Run 4 Individual Calls
              </Button>
              {singleTime !== null && (
                <span className="text-sm text-gray-600">
                  ⏱️ {singleTime.toFixed(1)}ms (4 HTTP requests)
                </span>
              )}
            </div>
          </div>

          {/* Batch call */}
          <div className="border rounded-lg p-4 border-orange-200 bg-orange-50">
            <h3 className="font-semibold mb-2 text-orange-700">
              Batch Call (1 HTTP request)
            </h3>
            <p className="text-sm text-gray-500 mb-3">
              All 4 calls sent as a JSON array in one fetch() — 1 round trip,
              server executes them concurrently.
            </p>
            <div className="flex items-center gap-4">
              <Button
                onClick={() => batchMutation.mutate()}
                disabled={batchMutation.isPending}
                className="bg-orange-500 hover:bg-orange-600"
              >
                {batchMutation.isPending && (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                )}
                Run Batch (1 Request)
              </Button>
              {batchTime !== null && (
                <span className="text-sm text-orange-700 font-medium">
                  ⏱️ {batchTime.toFixed(1)}ms (1 HTTP request)
                </span>
              )}
            </div>
          </div>

          {/* Notification */}
          <div className="border rounded-lg p-4">
            <h3 className="font-semibold mb-2">
              Notification (fire-and-forget)
            </h3>
            <p className="text-sm text-gray-500 mb-3">
              Send a notification — no response expected. Useful for analytics,
              logging, side effects.
            </p>
            <Button
              variant="outline"
              onClick={() => notifyMutation.mutate()}
              disabled={notifyMutation.isPending}
            >
              Send Notification
            </Button>
            {notifyMutation.isSuccess && (
              <span className="ml-3 text-sm text-green-600">✓ Sent (no response)</span>
            )}
          </div>

          {/* Timing comparison */}
          {singleTime !== null && batchTime !== null && (
            <div className="bg-gray-100 rounded-lg p-4">
              <h3 className="font-semibold mb-2">⚡ Performance Comparison</h3>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-gray-500">Individual (4 requests):</span>
                  <span className="ml-2 font-mono">{singleTime.toFixed(1)}ms</span>
                </div>
                <div>
                  <span className="text-gray-500">Batch (1 request):</span>
                  <span className="ml-2 font-mono text-orange-600 font-bold">
                    {batchTime.toFixed(1)}ms
                  </span>
                </div>
              </div>
              {batchTime < singleTime && (
                <p className="mt-2 text-sm text-green-700">
                  Batch was{" "}
                  <strong>{((singleTime / batchTime) * 100 - 100).toFixed(0)}% faster</strong>
                  {" "}— fewer network round trips!
                </p>
              )}
            </div>
          )}

          {/* Batch results */}
          {results && (
            <div className="bg-gray-900 text-gray-100 p-4 rounded-lg overflow-x-auto">
              <p className="text-xs text-gray-400 mb-2">Batch Response:</p>
              <pre className="text-xs">
                {JSON.stringify(results, null, 2)}
              </pre>
            </div>
          )}

          {/* Code example */}
          <div className="bg-gray-900 text-gray-100 p-4 rounded-lg overflow-x-auto">
            <p className="text-xs text-gray-400 mb-2">TypeScript Code:</p>
            <pre className="text-xs">{`// Batch: 4 calls in 1 HTTP request
const results = await client.batch([
  { method: "getUser", params: { id: 1 } },
  { method: "getUser", params: { id: 2 } },
  { method: "getUser", params: { id: 3 } },
  { method: "listUsers", params: {} },
]);
// results[0].result → User { id: 1, ... }

// Notification: fire-and-forget
await client.notify("logEvent", { event: "page_view" });`}</pre>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
