import { cn } from "@/lib/utils";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";

interface CodeBlockProps {
  title?: string;
  code: string;
  language?: string;
  className?: string;
}

export function CodeBlock({
  title,
  code,
  language = "typescript",
  className,
}: CodeBlockProps) {
  return (
    <section className={cn("overflow-hidden rounded-lg border bg-slate-950", className)}>
      {title ? (
        <header className="border-b border-slate-800 bg-slate-900/80 px-4 py-2 text-xs font-medium uppercase tracking-wide text-slate-300">
          {title}
        </header>
      ) : null}
      <SyntaxHighlighter
        language={language}
        style={oneDark}
        wrapLongLines
        customStyle={{
          margin: 0,
          background: "transparent",
          padding: "1rem",
          fontSize: "0.78rem",
        }}
      >
        {code}
      </SyntaxHighlighter>
    </section>
  );
}
