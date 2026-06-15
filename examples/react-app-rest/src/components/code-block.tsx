import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";
import { cn } from "../lib/utils";

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
    <section
      className={cn(
        "overflow-hidden rounded-lg border bg-slate-950",
        className,
      )}
    >
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
        codeTagProps={{
          style: {
            fontFamily:
              '"JetBrains Mono", "Fira Code", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
          },
        }}
      >
        {code}
      </SyntaxHighlighter>
    </section>
  );
}
