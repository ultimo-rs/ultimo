import type { Message } from "@/hooks/useWebSocket";
import { cn } from "@/lib/utils";

interface ChatMessageProps {
  message: Message;
}

export function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.sender === "user";
  const isSystem = message.sender === "system";

  return (
    <div
      className={cn(
        "flex w-full mb-4",
        isUser && "justify-end",
        isSystem && "justify-center"
      )}
    >
      <div
        className={cn(
          "max-w-[70%] rounded-lg px-4 py-2",
          isUser && "bg-primary text-primary-foreground",
          !isUser && !isSystem && "bg-muted",
          isSystem && "bg-secondary text-secondary-foreground text-sm italic"
        )}
      >
        <p className="break-words">{message.content}</p>
        <span
          className={cn(
            "text-xs opacity-70 mt-1 block",
            isSystem && "text-center"
          )}
        >
          {message.timestamp.toLocaleTimeString()}
        </span>
      </div>
    </div>
  );
}
