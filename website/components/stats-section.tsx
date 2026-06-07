import { GitCompareArrows, Layers, Zap } from "lucide-react";

export function StatsSection() {
  return (
    // biome-ignore lint/correctness/useUniqueElementIds: anchor target for nav
    <section
      id="performance"
      className="py-24 border-y border-border bg-background relative overflow-hidden"
    >
      <div className="absolute top-0 left-0 w-full h-full overflow-hidden -z-10 pointer-events-none">
        <div className="absolute top-[30%] right-[5%] w-[700px] h-[700px] bg-orange-500/8 blur-[130px] rounded-full" />
        <div className="absolute bottom-[20%] left-[10%] w-[500px] h-[500px] bg-red-500/8 blur-[100px] rounded-full" />
      </div>

      <div className="container px-4 md:px-6 mx-auto">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
          <div>
            <h2 className="text-3xl md:text-4xl font-bold mb-6 tracking-tight">
              Uncompromising <br />
              <span className="text-gradient">Performance</span>
            </h2>
            <p className="text-muted-foreground text-lg mb-8 leading-relaxed">
              Ultimo is a thin layer over Hyper and Tokio — the same core that
              powers the fastest Rust servers — so you get native speed with a
              higher-level developer experience. We measure what the framework
              itself costs and guard it in CI.
            </p>

            <div className="space-y-6">
              <div className="flex items-start gap-4">
                <div className="p-2 rounded-lg bg-orange-500/10 text-orange-500 border border-orange-500/20">
                  <Zap className="h-5 w-5" />
                </div>
                <div>
                  <h4 className="text-lg font-semibold text-foreground">
                    O(1) routing
                  </h4>
                  <p className="text-muted-foreground text-sm">
                    Constant-time route lookup — 10 routes or 10,000, the same
                    cost.
                  </p>
                </div>
              </div>

              <div className="flex items-start gap-4">
                <div className="p-2 rounded-lg bg-orange-500/10 text-orange-500 border border-orange-500/20">
                  <Layers className="h-5 w-5" />
                </div>
                <div>
                  <h4 className="text-lg font-semibold text-foreground">
                    Built on Hyper + Tokio
                  </h4>
                  <p className="text-muted-foreground text-sm">
                    The proven async HTTP core of the Rust ecosystem — no
                    re-implementation.
                  </p>
                </div>
              </div>

              <div className="flex items-start gap-4">
                <div className="p-2 rounded-lg bg-orange-500/10 text-orange-500 border border-orange-500/20">
                  <GitCompareArrows className="h-5 w-5" />
                </div>
                <div>
                  <h4 className="text-lg font-semibold text-foreground">
                    Regression-guarded
                  </h4>
                  <p className="text-muted-foreground text-sm">
                    Every framework change is benchmarked in CI — we don't get
                    slower by accident.
                  </p>
                </div>
              </div>
            </div>
          </div>

          <div className="relative">
            <div className="absolute -inset-4 bg-orange-500/20 blur-3xl rounded-full opacity-20" />
            <div className="relative rounded-xl border border-border bg-card p-6 shadow-xl">
              <h3 className="text-sm font-medium text-muted-foreground mb-2 uppercase tracking-wider">
                Route lookup time
              </h3>
              <p className="text-xs text-muted-foreground mb-6">
                In-process micro-benchmark — constant as the routing table grows.
              </p>

              <div className="space-y-4">
                {[
                  { label: "10 routes" },
                  { label: "100 routes" },
                  { label: "500 routes" },
                ].map((row) => (
                  <div key={row.label} className="space-y-2">
                    <div className="flex justify-between text-sm">
                      <span className="font-medium text-muted-foreground">
                        {row.label}
                      </span>
                      <span className="text-orange-500 font-bold">O(1)</span>
                    </div>
                    <div className="h-2 w-full bg-secondary rounded-full overflow-hidden">
                      <div className="h-full bg-gradient-to-r from-orange-500 to-red-500 w-[34%]" />
                    </div>
                  </div>
                ))}
              </div>

              <a
                href="https://docs.ultimo.dev/performance"
                className="mt-6 inline-block text-sm font-medium text-orange-500 hover:underline"
              >
                Reproduce it yourself → docs/performance
              </a>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
