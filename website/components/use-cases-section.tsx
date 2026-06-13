import {
  Bot,
  Brain,
  LineChart,
  Radio,
  Server,
} from "lucide-react";

const useCases = [
  {
    title: "Financial Services",
    description:
      "Trading systems, risk engines, payment processing — where tail latency directly impacts revenue.",
    icon: LineChart,
  },
  {
    title: "AI & ML Backends",
    description:
      "Inference serving, model orchestration, and feature pipelines that need predictable response times.",
    icon: Brain,
  },
  {
    title: "Bot Execution",
    description:
      "Trading bots, automation engines, and webhook processors that react to events in milliseconds.",
    icon: Bot,
  },
  {
    title: "Real-Time Systems",
    description:
      "Live data feeds, notifications, and collaborative tools over WebSocket pub/sub channels.",
    icon: Radio,
  },
  {
    title: "Backend Infrastructure",
    description:
      "API gateways, microservices, and data pipelines deployed as single static binaries.",
    icon: Server,
  },
];

export function UseCasesSection() {
  return (
    <section className="py-24 border-y border-border bg-background/50 relative overflow-hidden">
      <div className="absolute top-0 left-0 w-full h-full overflow-hidden -z-10 pointer-events-none">
        <div className="absolute top-[40%] left-[20%] w-[600px] h-[600px] bg-orange-500/5 blur-[120px] rounded-full" />
      </div>

      <div className="container px-4 md:px-6 mx-auto">
        <div className="text-center mb-16">
          <h2 className="text-3xl md:text-4xl font-bold mb-4 tracking-tight">
            Built For Systems Where{" "}
            <span className="text-gradient">Every Millisecond Counts</span>
          </h2>
          <p className="text-muted-foreground text-lg max-w-2xl mx-auto">
            Ultimo is designed for backend workloads where speed, security, and
            efficiency are non-negotiable requirements.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 max-w-5xl mx-auto">
          {useCases.map((useCase) => (
            <div
              key={useCase.title}
              className="group relative rounded-xl border border-border bg-card p-6 transition-all hover:border-orange-500/30 hover:shadow-lg hover:shadow-orange-500/5"
            >
              <div className="p-2.5 rounded-lg bg-orange-500/10 text-orange-500 border border-orange-500/20 w-fit mb-4 group-hover:bg-orange-500/20 transition-colors">
                <useCase.icon className="h-5 w-5" />
              </div>
              <h3 className="text-lg font-semibold mb-2 text-foreground">
                {useCase.title}
              </h3>
              <p className="text-sm text-muted-foreground leading-relaxed">
                {useCase.description}
              </p>
            </div>
          ))}
        </div>

        <div className="mt-12 text-center">
          <div className="inline-flex flex-wrap items-center gap-6 text-sm text-muted-foreground">
            <span className="flex items-center gap-2">
              <span className="h-1.5 w-1.5 rounded-full bg-orange-500" />
              Zero GC pauses
            </span>
            <span className="flex items-center gap-2">
              <span className="h-1.5 w-1.5 rounded-full bg-orange-500" />
              100% safe Rust
            </span>
            <span className="flex items-center gap-2">
              <span className="h-1.5 w-1.5 rounded-full bg-orange-500" />
              Single binary deploy
            </span>
            <span className="flex items-center gap-2">
              <span className="h-1.5 w-1.5 rounded-full bg-orange-500" />
              Predictable memory
            </span>
          </div>
        </div>
      </div>
    </section>
  );
}
