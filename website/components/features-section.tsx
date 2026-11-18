import {
  Activity,
  BookOpen,
  Database,
  Layers,
  ListChecks,
  Radio,
  Server,
  Terminal,
} from "lucide-react";
import { siteConfig } from "@/lib/config";

// Speed, type safety, security and AI tooling each get their own section
// further down the page, so they're deliberately not repeated here.
const features = [
  {
    title: "REST and JSON-RPC",
    description: "Both in one app, with batch requests and notifications.",
    icon: Layers,
    href: "/rpc",
  },
  {
    title: "Real-time",
    description: "WebSockets with built-in pub/sub, plus SSE and streaming.",
    icon: Radio,
    href: "/websocket",
  },
  {
    title: "OpenAPI",
    description: "Specs generated from your code, Swagger UI at /docs.",
    icon: BookOpen,
    href: "/openapi",
  },
  {
    title: "Validation",
    description: "Typed extractors that reject bad input before your handler runs.",
    icon: ListChecks,
    href: "/api-reference",
  },
  {
    title: "Caching and Redis",
    description: "Response caching, shared sessions and rate limits across instances.",
    icon: Server,
    href: "/caching",
  },
  {
    title: "Observability",
    description: "Prometheus metrics and OpenTelemetry tracing, one line each.",
    icon: Activity,
    href: "/metrics",
  },
  {
    title: "Databases",
    description: "SQLx and Diesel for PostgreSQL, MySQL and SQLite.",
    icon: Database,
    href: "/database",
  },
  {
    title: "CLI",
    description: "Scaffold, hot-reload and build with ultimo new, dev and build.",
    icon: Terminal,
    href: "/cli",
  },
];

export function FeaturesSection() {
  return (
    <section className="py-24 bg-muted/30">
      <div className="container px-4 md:px-6 mx-auto">
        <div className="text-center max-w-3xl mx-auto mb-14">
          <h2 className="text-3xl md:text-5xl font-bold mb-6 tracking-tight">
            Everything you need to ship
          </h2>
          <p className="text-muted-foreground text-lg">
            The pieces a production API needs are built in. Each one is an
            opt-in Cargo feature, so you only compile what you use.
          </p>
        </div>

        <ul className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 max-w-6xl mx-auto">
          {features.map((feature) => (
            <li key={feature.title}>
              <a
                href={`${siteConfig.links.docs}${feature.href}`}
                className="group flex h-full flex-col rounded-xl border border-border bg-card p-5 transition-colors hover:border-orange-500/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-orange-500"
              >
                <feature.icon
                  className="mb-4 h-5 w-5 text-orange-500"
                  aria-hidden="true"
                />
                <h3 className="mb-1.5 font-semibold text-foreground group-hover:text-orange-500 transition-colors">
                  {feature.title}
                </h3>
                <p className="text-sm leading-relaxed text-muted-foreground">
                  {feature.description}
                </p>
              </a>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
