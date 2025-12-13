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
    "ws://localhost:4000/ws"
  );

  return (
    <div className="min-h-screen bg-background p-4 flex items-center justify-center">
      <Card className="w-full max-w-4xl h-[600px] flex flex-col">
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
        <CardContent className="flex-1 flex flex-col p-0 overflow-hidden">
          <MessageList messages={messages} />
          <MessageInput onSendMessage={sendMessage} disabled={!isConnected} />
        </CardContent>
      </Card>
    </div>
  );
}

export default App;
