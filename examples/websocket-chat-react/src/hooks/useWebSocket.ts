import { useEffect, useRef, useState } from "react";

export interface Message {
  id: string;
  content: string;
  sender: "user" | "other" | "system";
  timestamp: Date;
}

export interface UseWebSocketReturn {
  messages: Message[];
  sendMessage: (content: string) => void;
  isConnected: boolean;
  connectionError: string | null;
}

export function useWebSocket(url: string): UseWebSocketReturn {
  const [messages, setMessages] = useState<Message[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | undefined>(undefined);
  const isConnectingRef = useRef(false);

  useEffect(() => {
    // Prevent double connection in React Strict Mode
    if (isConnectingRef.current) return;
    isConnectingRef.current = true;

    const connect = () => {
      // Don't create a new connection if one already exists and is open
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        return;
      }

      try {
        const ws = new WebSocket(url);

        ws.onopen = () => {
          setIsConnected(true);
          setConnectionError(null);
          setMessages((prev) => [
            ...prev,
            {
              id: `${Date.now()}-${Math.random()}`,
              content: "Connected to chat server",
              sender: "system",
              timestamp: new Date(),
            },
          ]);
        };

        ws.onmessage = (event) => {
          const message: Message = {
            id: `${Date.now()}-${Math.random()}`,
            content: event.data,
            sender: "other",
            timestamp: new Date(),
          };
          setMessages((prev) => [...prev, message]);
        };

        ws.onerror = () => {
          setConnectionError("WebSocket error occurred");
        };

        ws.onclose = (event) => {
          setIsConnected(false);

          // Only reconnect if it wasn't a clean close
          if (event.code !== 1000) {
            setMessages((prev) => [
              ...prev,
              {
                id: `${Date.now()}-${Math.random()}`,
                content: "Disconnected from server. Reconnecting...",
                sender: "system",
                timestamp: new Date(),
              },
            ]);

            // Auto-reconnect after 3 seconds
            reconnectTimeoutRef.current = setTimeout(() => {
              connect();
            }, 3000);
          }
        };

        wsRef.current = ws;
      } catch (error) {
        setConnectionError(`Connection failed: ${error}`);
      }
    };

    connect();

    return () => {
      isConnectingRef.current = false;
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [url]);

  const sendMessage = (content: string) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(content);

      const message: Message = {
        id: `${Date.now()}-${Math.random()}`,
        content,
        sender: "user",
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, message]);
    }
  };

  return {
    messages,
    sendMessage,
    isConnected,
    connectionError,
  };
}
