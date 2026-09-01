import { Bot, FileType, PackageCheck, Plug } from "lucide-react";

const pillars = [
  {
    icon: FileType,
    title: "Typed codegen",
    description:
      "Write your API once in Rust; the typed TypeScript client is generated from it. Agents never guess client types — they're derived.",
  },
  {
    icon: PackageCheck,
    title: "Scaffolds that build",
    description:
      "`ultimo new` produces projects that compile out of the box, with dependency pins that track the current release. No stale-version dead ends.",
  },
  {
    icon: Bot,
    title: "Docs agents can read",
    description:
      "Machine-readable llms.txt docs and a Context7-indexed corpus keep agents grounded in the current API. Every scaffold ships an AGENTS.md ruleset.",
  },
  {
    icon: Plug,
    title: "MCP server (soon)",
    description:
      "An `ultimo mcp` server will give your agent live, typed tools — search docs, scaffold, and introspect your project's RPC surface.",
  },
];

export function AiNativeSection() {
  return (
    <section className="py-24 overflow-hidden relative">
      <div className="absolute top-0 left-0 w-full h-full overflow-hidden -z-10 pointer-events-none">
        <div className="absolute top-[-100px] right-[10%] w-[500px] h-[600px] bg-gradient-to-b from-orange-500/12 to-transparent blur-[100px]" />
      </div>

      <div className="container px-4 md:px-6 mx-auto">
        <div className="mb-16 text-center">
          <div className="inline-flex items-center gap-2 px-3 py-1 mb-6 rounded-full border border-orange-500/20 bg-orange-500/10 text-orange-500 text-sm font-medium">
            <Bot className="h-4 w-4" />
            AI-native
          </div>
          <h2 className="text-3xl md:text-5xl font-bold mb-6 tracking-tight">
            Built for you and your{" "}
            <span className="text-gradient">coding agent</span>
          </h2>
          <p className="text-muted-foreground text-lg max-w-2xl mx-auto">
            Most software is now written with AI coding agents. Ultimo is designed
            to be the easiest Rust framework for them to build with — typed
            contracts, deterministic scaffolds, and docs made to be read by
            machines.
          </p>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6 max-w-6xl mx-auto">
          {pillars.map((pillar) => (
            <div
              key={pillar.title}
              className="rounded-xl border border-border bg-card p-6 shadow-lg flex flex-col"
            >
              <div className="mb-4 inline-flex h-11 w-11 items-center justify-center rounded-lg bg-gradient-to-br from-orange-500 to-amber-500">
                <pillar.icon className="h-5 w-5 text-white" />
              </div>
              <h3 className="text-lg font-semibold mb-2 text-foreground">
                {pillar.title}
              </h3>
              <p className="text-sm text-muted-foreground leading-relaxed">
                {pillar.description}
              </p>
            </div>
          ))}
        </div>

        <div className="mt-12 text-center">
          <a
            href="https://docs.ultimo.dev/ai-agents"
            className="inline-flex items-center gap-2 text-orange-500 hover:text-orange-400 font-medium transition-colors"
          >
            Using Ultimo with AI coding agents →
          </a>
        </div>
      </div>
    </section>
  );
}
