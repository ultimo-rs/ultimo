import { ArrowRight, Terminal } from "lucide-react"
import { Button } from "@/components/ui/button"

export function HeroSection() {
  return (
    <section className="relative pt-32 pb-20 md:pt-48 md:pb-32 overflow-hidden min-h-[90vh] flex flex-col items-center justify-center">
      <div className="absolute inset-0 bg-grid-pattern -z-20 opacity-20" />

      <div className="absolute top-0 left-0 w-full h-full overflow-hidden -z-10 pointer-events-none">
        {/* Primary center spotlight */}
        <div className="absolute top-[-150px] left-1/2 -translate-x-1/2 w-[800px] h-[800px] bg-orange-500/20 blur-[120px] rounded-full" />

        {/* Left diagonal beam */}
        <div className="absolute top-0 left-[10%] w-[500px] h-[700px] bg-gradient-to-b from-orange-500/15 via-orange-600/10 to-transparent rotate-[-15deg] blur-[100px]" />

        {/* Right diagonal beam */}
        <div className="absolute top-0 right-[10%] w-[500px] h-[700px] bg-gradient-to-b from-red-500/15 via-orange-500/10 to-transparent rotate-[15deg] blur-[100px]" />

        {/* Accent spotlight left */}
        <div className="absolute top-[20%] left-[5%] w-[400px] h-[400px] bg-orange-600/10 blur-[80px] rounded-full" />

        {/* Accent spotlight right */}
        <div className="absolute top-[30%] right-[5%] w-[350px] h-[350px] bg-red-500/10 blur-[80px] rounded-full" />

        {/* Light source line at top */}
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-[400px] h-[3px] bg-gradient-to-r from-transparent via-orange-500 to-transparent opacity-60 blur-sm" />
      </div>

      <div className="container px-4 md:px-6 mx-auto flex flex-col items-center text-center relative z-10">
        <div className="inline-flex items-center rounded-full border border-orange-500/30 bg-orange-500/10 px-3 py-1 text-sm font-medium text-orange-400 mb-8 backdrop-blur-sm">
          <span className="flex h-2 w-2 rounded-full bg-orange-500 mr-2 animate-pulse"></span>
          v0.1.0 Now Available
        </div>

        <h1 className="text-4xl sm:text-6xl md:text-7xl font-bold tracking-tight mb-6 max-w-4xl text-balance">
          The <span className="text-gradient">Rust Framework</span> for <br className="hidden md:block" />
          Modern Web Development
        </h1>

        <p className="text-lg md:text-xl text-muted-foreground max-w-2xl mb-10 text-balance leading-relaxed">
          Performance-equivalent to Axum (152k+ req/sec). Automatic TypeScript client generation. The full-stack
          experience you've been waiting for.
        </p>

        <div className="flex flex-col sm:flex-row gap-4 w-full justify-center">
          <Button size="lg" className="h-12 px-8 text-base">
            Start Building
            <ArrowRight className="ml-2 h-4 w-4" />
          </Button>
          <Button
            size="lg"
            variant="outline"
            className="h-12 px-8 text-base border-input bg-background/50 text-foreground hover:bg-accent hover:text-accent-foreground"
          >
            <Terminal className="mr-2 h-4 w-4" />
            cargo install ultimo
          </Button>
        </div>

        <div className="mt-20 relative w-full max-w-5xl mx-auto rounded-xl border border-border bg-zinc-950 shadow-2xl overflow-hidden group">
          <div className="absolute inset-0 bg-gradient-to-b from-orange-500/5 to-transparent opacity-50" />
          <div className="flex items-center border-b border-white/10 bg-white/5 px-4 py-3">
            <div className="flex gap-2">
              <div className="h-3 w-3 rounded-full bg-red-500/20 border border-red-500/50" />
              <div className="h-3 w-3 rounded-full bg-yellow-500/20 border border-yellow-500/50" />
              <div className="h-3 w-3 rounded-full bg-green-500/20 border border-green-500/50" />
            </div>
            <div className="mx-auto text-xs font-mono text-zinc-500">server.rs</div>
          </div>
          <div className="p-6 overflow-x-auto text-left">
            <pre className="font-mono text-sm leading-relaxed">
              <code className="text-zinc-300">
                <span className="text-purple-400">use</span> <span className="text-zinc-100">ultimo::prelude::*;</span>
                {"\n\n"}
                <span className="text-purple-400">#[tokio::main]</span>
                {"\n"}
                <span className="text-blue-400">async fn</span> <span className="text-yellow-300">main</span>() {"->"}{" "}
                <span className="text-zinc-100">ultimo::Result{"<()>"}</span> {"{"}
                {"\n"} <span className="text-zinc-500">// Initialize app</span>
                {"\n"} <span className="text-blue-400">let</span> <span className="text-blue-400">mut</span> app ={" "}
                <span className="text-green-400">Ultimo</span>::<span className="text-yellow-300">new</span>();
                {"\n\n"}
                {"\n"} <span className="text-zinc-500">// Define a route</span>
                {"\n"} app.<span className="text-yellow-300">get</span>(
                <span className="text-orange-300">"/hello"</span>, |ctx|{" "}
                <span className="text-blue-400">async move</span> {"{"}
                {"\n"} ctx.<span className="text-yellow-300">json</span>(<span className="text-green-400">json!</span>(
                {"{"} <span className="text-orange-300">"message"</span>:{" "}
                <span className="text-orange-300">"Hello from Ultimo!"</span> {"}"})).
                <span className="text-blue-400">await</span>
                {"\n"} {"}"});
                {"\n\n"}
                {"\n"} <span className="text-zinc-500">// Start server</span>
                {"\n"} app.<span className="text-yellow-300">listen</span>(
                <span className="text-orange-300">"127.0.0.1:3000"</span>).<span className="text-blue-400">await</span>
                {"\n"}
                {"}"}
              </code>
            </pre>
          </div>
        </div>
      </div>
    </section>
  )
}
