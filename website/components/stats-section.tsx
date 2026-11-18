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
                  <h3 className="text-lg font-semibold text-foreground">
                    O(1) routing
                  </h3>
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
                  <h3 className="text-lg font-semibold text-foreground">
                    Built on Hyper + Tokio
                  </h3>
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
                  <h3 className="text-lg font-semibold text-foreground">
                    Regression-guarded
                  </h3>
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
              <h3 className="text-base font-semibold text-foreground mb-1">
                Route lookup cost as the app grows
              </h3>
              <p className="text-sm text-muted-foreground mb-6">
                Relative to 10 routes. Our benchmark suite caught a linear scan
                in the router; static routes are now a hash-map lookup.
              </p>

              <table className="w-full text-sm">
                <caption className="sr-only">
                  Route lookup time relative to a 10-route app, before and
                  after the router fix
                </caption>
                <thead>
                  <tr className="text-left text-muted-foreground">
                    <th scope="col" className="pb-3 font-medium">
                      Routes
                    </th>
                    <th scope="col" className="pb-3 font-medium">
                      Before (linear scan)
                    </th>
                    <th scope="col" className="pb-3 font-medium">
                      Now (O(1))
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {[
                    { routes: "10", before: 1, label: "1×" },
                    { routes: "100", before: 3.6, label: "3.6×" },
                    { routes: "500", before: 15, label: "15×" },
                  ].map((row) => (
                    <tr key={row.routes} className="border-t border-border/50">
                      <th
                        scope="row"
                        className="py-3 pr-4 text-left font-medium text-foreground tabular-nums"
                      >
                        {row.routes}
                      </th>
                      <td className="py-3 pr-4">
                        <div className="flex items-center gap-2">
                          <div className="h-2 flex-1 rounded-full bg-secondary">
                            <div
                              className="h-full rounded-full bg-muted-foreground/50"
                              style={{ width: `${(row.before / 15) * 100}%` }}
                            />
                          </div>
                          <span className="w-10 text-right tabular-nums text-muted-foreground">
                            {row.label}
                          </span>
                        </div>
                      </td>
                      <td className="py-3">
                        <div className="flex items-center gap-2">
                          <div className="h-2 flex-1 rounded-full bg-secondary">
                            <div
                              className="h-full rounded-full bg-orange-500"
                              style={{ width: `${(1 / 15) * 100}%` }}
                            />
                          </div>
                          <span className="w-10 text-right tabular-nums text-orange-500 font-semibold">
                            1×
                          </span>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>

              <a
                href="https://docs.ultimo.dev/performance"
                className="mt-6 inline-block text-sm font-medium text-orange-500 hover:underline"
              >
                How we measure, and how to reproduce it
              </a>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
