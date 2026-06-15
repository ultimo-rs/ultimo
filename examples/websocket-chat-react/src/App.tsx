import { CodeBlock } from "@/components/CodeBlock";
import { ConnectionStatus } from "@/components/ConnectionStatus";
import { MessageInput } from "@/components/MessageInput";
import { MessageList } from "@/components/MessageList";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { useWebSocket } from "@/hooks/useWebSocket";

function App() {
  const { messages, sendMessage, isConnected, connectionError } = useWebSocket(
    "ws://localhost:4000/ws",
  );

  return (
    <div className="min-h-screen bg-gradient-to-b from-slate-100 via-white to-slate-100">
      <header className="border-b bg-background/90 backdrop-blur">
        <div className="mx-auto flex w-full max-w-5xl flex-col gap-2 px-4 py-6 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-primary">
              Ultimo Examples
            </p>
            <h1 className="mt-1 text-2xl font-bold tracking-tight sm:text-3xl">
              WebSocket Chat Showcase
            </h1>
          </div>
          <p className="max-w-md text-sm text-muted-foreground">
            Same shadcn UI language across examples, now with a highlighted
            real-time message payload sample.
          </p>
        </div>
      </header>

      <main className="mx-auto grid w-full max-w-5xl gap-6 px-4 py-8">
        <Card className="h-[600px] w-full">
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle>WebSocket Chat</CardTitle>
                <CardDescription>
                  Real-time chat with Ultimo WebSocket
                </CardDescription>
              </div>
              <ConnectionStatus
                isConnected={isConnected}
                error={connectionError}
              />
            </div>
          </CardHeader>
          <CardContent className="flex h-[490px] flex-col overflow-hidden p-0">
            <MessageList messages={messages} />
            <MessageInput onSendMessage={sendMessage} disabled={!isConnected} />
          </CardContent>
        </Card>

        <CodeBlock
          title="WebSocket Payload Example"
          language="json"
          code={`{
  "type": "message",
  "username": "alice",
  "content": "Hey team, this is real-time!",
  "timestamp": 1718448000
}`}
        />
      </main>
    </div>
  );
}

export default App;
