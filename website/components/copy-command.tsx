"use client";

import { Check, Copy, Terminal } from "lucide-react";
import { useState } from "react";
import { cn } from "@/lib/utils";

/** A shell command that copies itself to the clipboard when clicked. */
export function CopyCommand({
  command,
  className,
}: {
  command: string;
  className?: string;
}) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard blocked: the command stays visible to copy by hand.
    }
  }

  return (
    <button
      type="button"
      onClick={copy}
      aria-label={copied ? `Copied: ${command}` : `Copy “${command}”`}
      className={cn(
        "inline-flex h-12 items-center justify-center gap-3 rounded-md border border-input bg-background/50 px-6 font-mono text-sm text-foreground transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-orange-500/60",
        className,
      )}
    >
      <Terminal className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
      <span>{command}</span>
      {copied ? (
        <Check className="h-4 w-4 text-green-500" aria-hidden="true" />
      ) : (
        <Copy className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
      )}
      <span className="sr-only" aria-live="polite">
        {copied ? "Copied to clipboard" : ""}
      </span>
    </button>
  );
}
