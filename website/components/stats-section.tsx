import { BarChart3, Clock, Zap } from "lucide-react";

export function StatsSection() {
  return (
    // biome-ignore lint/correctness/useUniqueElementIds: <explanation>
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
              Ultimo is built on top of Hyper and Tokio, matching the
              performance of the fastest Rust web frameworks while providing a
              higher-level developer experience.
            </p>

            <div className="space-y-6">
              <div className="flex items-start gap-4">
                <div className="p-2 rounded-lg bg-orange-500/10 text-orange-500 border border-orange-500/20">
                  <Zap className="h-5 w-5" />
                </div>
                <div>
                  <h4 className="text-lg font-semibold text-foreground">
                    158k+ req/sec
                  </h4>
                  <p className="text-muted-foreground text-sm">
                    Industry-leading throughput performance
                  </p>
                </div>
              </div>

              <div className="flex items-start gap-4">
                <div className="p-2 rounded-lg bg-orange-500/10 text-orange-500 border border-orange-500/20">
                  <Clock className="h-5 w-5" />
                </div>
                <div>
                  <h4 className="text-lg font-semibold text-foreground">
                    0.6ms Latency
                  </h4>
                  <p className="text-muted-foreground text-sm">
                    Average response time for hello-world benchmarks
                  </p>
                </div>
              </div>

              <div className="flex items-start gap-4">
                <div className="p-2 rounded-lg bg-orange-500/10 text-orange-500 border border-orange-500/20">
                  <BarChart3 className="h-5 w-5" />
                </div>
                <div>
                  <h4 className="text-lg font-semibold text-foreground">
                    15x Faster
                  </h4>
                  <p className="text-muted-foreground text-sm">
                    Compared to Python/FastAPI alternatives
                  </p>
                </div>
              </div>
            </div>
          </div>

          <div className="relative">
            <div className="absolute -inset-4 bg-orange-500/20 blur-3xl rounded-full opacity-20" />
            <div className="relative rounded-xl border border-border bg-card p-6 shadow-xl">
              <h3 className="text-sm font-medium text-muted-foreground mb-6 uppercase tracking-wider">
                Requests Per Second
              </h3>

              <div className="space-y-4">
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="font-medium text-foreground">
                      Ultimo (Rust)
                    </span>
                    <span className="text-orange-500 font-bold">158,247</span>
                  </div>
                  <div className="h-2 w-full bg-secondary rounded-full overflow-hidden">
                    <div className="h-full bg-gradient-to-r from-orange-500 to-red-500 w-[100%]" />
                  </div>
                </div>

                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="font-medium text-muted-foreground">
                      Axum (Rust)
                    </span>
                    <span className="text-muted-foreground">153,105</span>
                  </div>
                  <div className="h-2 w-full bg-secondary rounded-full overflow-hidden">
                    <div className="h-full bg-muted-foreground/30 w-[97%]" />
                  </div>
                </div>

                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="font-medium text-muted-foreground">
                      Hono (Bun)
                    </span>
                    <span className="text-muted-foreground">132,000</span>
                  </div>
                  <div className="h-2 w-full bg-secondary rounded-full overflow-hidden">
                    <div className="h-full bg-muted-foreground/30 w-[86%]" />
                  </div>
                </div>

                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="font-medium text-muted-foreground">
                      Hono (Node)
                    </span>
                    <span className="text-muted-foreground">62,000</span>
                  </div>
                  <div className="h-2 w-full bg-secondary rounded-full overflow-hidden">
                    <div className="h-full bg-muted-foreground/30 w-[40%]" />
                  </div>
                </div>

                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="font-medium text-muted-foreground">
                      FastAPI (Python)
                    </span>
                    <span className="text-muted-foreground">10,000</span>
                  </div>
                  <div className="h-2 w-full bg-secondary rounded-full overflow-hidden">
                    <div className="h-full bg-muted-foreground/30 w-[6%]" />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
