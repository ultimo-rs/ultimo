import { ArrowDown } from "lucide-react"

export function TypeSafeSection() {
  return (
    <section className="py-24 bg-muted/30 overflow-hidden relative">
      <div className="absolute top-0 left-0 w-full h-full overflow-hidden -z-10 pointer-events-none">
        <div className="absolute top-[-100px] left-[15%] w-[500px] h-[700px] bg-gradient-to-b from-orange-500/12 to-transparent blur-[100px]" />
        <div className="absolute top-[-100px] right-[15%] w-[500px] h-[700px] bg-gradient-to-b from-red-500/12 to-transparent blur-[100px]" />
      </div>

      <div className="container px-4 md:px-6 mx-auto">
        <div className="mb-16 text-center">
          <h2 className="text-3xl md:text-5xl font-bold mb-6 tracking-tight">
            End-to-End <span className="text-orange-500">Type Safety</span>
          </h2>
          <p className="text-muted-foreground text-lg max-w-2xl mx-auto">
            Define your API in Rust. Get TypeScript clients automatically. Catch errors at compile time, not runtime.
          </p>
        </div>

        <div className="relative grid grid-cols-1 md:grid-cols-2 gap-8 max-w-5xl mx-auto">
          {/* Connection Line */}
          <div className="hidden md:block absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 z-10 p-2 bg-background rounded-full border border-border">
            <div className="bg-gradient-to-r from-orange-500 to-amber-500 p-2 rounded-full">
              <ArrowDown className="h-6 w-6 text-white rotate-[-90deg] md:rotate-0" />
            </div>
          </div>

          {/* Backend Card */}
          <div className="rounded-xl border border-border bg-card overflow-hidden flex flex-col shadow-lg">
            <div className="px-6 py-4 border-b border-border bg-muted/50 flex items-center justify-between">
              <span className="text-sm font-medium text-foreground">backend/src/main.rs</span>
              <div className="px-2 py-0.5 rounded text-xs bg-orange-500/10 text-orange-500 border border-orange-500/20">
                Rust
              </div>
            </div>
            <div className="p-6 font-mono text-sm overflow-x-auto flex-1 bg-zinc-950 text-zinc-300">
              <pre>
                <code>
                  <span className="text-purple-400">struct</span> <span className="text-yellow-300">User</span> {"{"}{" "}
                  {"\n"}
                  {"    "}id: <span className="text-blue-400">u32</span>, {"\n"}
                  {"    "}name: <span className="text-blue-400">String</span>, {"\n"}
                  {"    "}email: <span className="text-blue-400">String</span> {"\n"}
                  {"}"} {"\n\n"}
                  rpc.<span className="text-yellow-300">query</span>( {"\n"}
                  {"    "}
                  <span className="text-orange-300">"getUser"</span>, {"\n"}
                  {"    "}|id: <span className="text-blue-400">u32</span>|{" "}
                  <span className="text-blue-400">async move</span> {"{"}...{"}"} {"\n"}
                  );
                </code>
              </pre>
            </div>
          </div>

          {/* Frontend Card */}
          <div className="rounded-xl border border-border bg-card overflow-hidden flex flex-col shadow-lg">
            <div className="px-6 py-4 border-b border-border bg-muted/50 flex items-center justify-between">
              <span className="text-sm font-medium text-foreground">frontend/src/client.ts</span>
              <div className="px-2 py-0.5 rounded text-xs bg-blue-500/10 text-blue-500 border border-blue-500/20">
                Generated
              </div>
            </div>
            <div className="p-6 font-mono text-sm overflow-x-auto flex-1 bg-zinc-950 text-zinc-300">
              <pre>
                <code>
                  <span className="text-purple-400">export interface</span>{" "}
                  <span className="text-yellow-300">User</span> {"{"} {"\n"}
                  {"    "}id: <span className="text-blue-400">number</span>; {"\n"}
                  {"    "}name: <span className="text-blue-400">string</span>; {"\n"}
                  {"    "}email: <span className="text-blue-400">string</span>; {"\n"}
                  {"}"} {"\n\n"}
                  <span className="text-zinc-500">// Fully typed!</span> {"\n"}
                  <span className="text-blue-400">const</span> user = <span className="text-purple-400">await</span>{" "}
                  client.<span className="text-yellow-300">getUser</span>(1); {"\n"}
                  console.<span className="text-yellow-300">log</span>(user.<span className="text-blue-400">name</span>
                  );
                </code>
              </pre>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
