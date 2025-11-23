import { Zap, FileType, BookOpen, Layers, Globe, ShieldCheck, Code2 } from "lucide-react"

const features = [
  {
    title: "Blazing Fast",
    description: "Matches Axum performance with 152k+ req/sec and sub-millisecond latency. Built on Hyper & Tokio.",
    icon: Zap,
    colSpan: "lg:col-span-2",
  },
  {
    title: "Type-Safe RPC",
    description:
      "Define your API in Rust, get fully typed TypeScript clients automatically. No more manual type syncing.",
    icon: FileType,
    colSpan: "lg:col-span-1",
  },
  {
    title: "OpenAPI Support",
    description:
      "Generate OpenAPI 3.0 specs from your code automatically. Compatible with Swagger UI and generic client generators.",
    icon: BookOpen,
    colSpan: "lg:col-span-1",
  },
  {
    title: "Hybrid Design",
    description:
      "Support both REST endpoints and JSON-RPC procedures in the same application. The best of both worlds.",
    icon: Layers,
    colSpan: "lg:col-span-2",
  },
  {
    title: "Batteries Included",
    description: "Validation, authentication, CORS, and rate limiting are built-in. Stop reinventing the wheel.",
    icon: ShieldCheck,
    colSpan: "lg:col-span-1",
  },
  {
    title: "Developer First",
    description: "CLI tools for scaffolding, development, and building. Ergonomic APIs designed for productivity.",
    icon: Code2,
    colSpan: "lg:col-span-1",
  },
  {
    title: "Universal",
    description: "Works with SQLx, Diesel, and any other Rust crate. Deploy anywhere Rust runs.",
    icon: Globe,
    colSpan: "lg:col-span-1",
  },
]

export function FeaturesSection() {
  return (
    <section id="features" className="py-24 bg-muted/30 relative overflow-hidden">
      <div className="absolute top-0 left-0 w-full h-full overflow-hidden -z-10 pointer-events-none">
        <div className="absolute top-[20%] left-[10%] w-[600px] h-[600px] bg-orange-500/10 blur-[120px] rounded-full" />
        <div className="absolute bottom-[10%] right-[15%] w-[500px] h-[500px] bg-red-500/10 blur-[100px] rounded-full" />
      </div>

      <div className="container px-4 md:px-6 mx-auto">
        <div className="text-center max-w-3xl mx-auto mb-16">
          <h2 className="text-3xl md:text-5xl font-bold mb-6 tracking-tight">Everything you need to ship</h2>
          <p className="text-muted-foreground text-lg">
            Ultimo combines the raw performance of Rust with the developer experience of modern full-stack frameworks.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {features.map((feature, i) => (
            <div
              key={i}
              className={`group relative overflow-hidden rounded-xl border border-border bg-card p-8 hover:bg-accent/50 transition-colors ${feature.colSpan || ""}`}
            >
              <div className="absolute inset-0 bg-gradient-to-br from-orange-500/5 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />

              <div className="relative z-10">
                <div className="mb-4 inline-flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10 text-orange-500 border border-primary/10">
                  <feature.icon className="h-6 w-6" />
                </div>
                <h3 className="text-xl font-bold mb-2 text-foreground">{feature.title}</h3>
                <p className="text-muted-foreground leading-relaxed">{feature.description}</p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
