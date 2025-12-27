"use client";

import {
  BookOpen,
  Code2,
  FileType,
  Globe,
  Layers,
  Radio,
  ShieldCheck,
  Zap,
} from "lucide-react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

const featureCategories = [
  {
    id: "core",
    label: "Core Features",
    features: [
      {
        title: "Blazing Fast",
        description:
          "Lightning-fast performance that outpaces the competition. Pure Rust efficiency at its finest.",
        icon: Zap,
        details: [
          "158,000+ requests per second",
          "Sub-millisecond average latency",
          "Zero-cost abstractions",
          "Async/await with Tokio",
        ],
      },
      {
        title: "Type-Safe RPC",
        description:
          "Define your API once in Rust, get fully typed TypeScript clients automatically.",
        icon: FileType,
        details: [
          "Automatic TypeScript generation",
          "Full IDE autocomplete",
          "Compile-time type checking",
          "Single source of truth",
        ],
      },
      {
        title: "Hybrid Design",
        description:
          "Support both REST endpoints and JSON-RPC procedures in the same application.",
        icon: Layers,
        details: [
          "RESTful routing",
          "JSON-RPC procedures",
          "Mix and match freely",
          "Flexible configuration",
        ],
      },
      {
        title: "WebSocket Support",
        description:
          "Zero-dependency RFC 6455 compliant WebSocket with built-in pub/sub system.",
        icon: Radio,
        details: [
          "Zero external dependencies",
          "Built-in pub/sub",
          "Type-safe handlers",
          "Production-ready with 93 tests",
        ],
      },
    ],
  },
  {
    id: "developer-tools",
    label: "Developer Tools",
    features: [
      {
        title: "OpenAPI Support",
        description:
          "Generate OpenAPI 3.0 specifications from your code automatically.",
        icon: BookOpen,
        details: [
          "Auto-generated specs",
          "Swagger UI integration",
          "API documentation",
          "Multi-language clients",
        ],
      },
      {
        title: "Batteries Included",
        description:
          "Validation, authentication, CORS, and middleware are built-in.",
        icon: ShieldCheck,
        details: [
          "Request validation",
          "Built-in CORS",
          "Auth helpers",
          "Composable middleware",
        ],
      },
      {
        title: "Developer First",
        description:
          "CLI tools and ergonomic APIs designed for maximum productivity.",
        icon: Code2,
        details: [
          "Intuitive routing API",
          "Hot reload in dev",
          "Helpful error messages",
          "Great documentation",
        ],
      },
    ],
  },
  {
    id: "ecosystem",
    label: "Ecosystem",
    features: [
      {
        title: "Database Support",
        description:
          "First-class integration with SQLx and Diesel. Build type-safe database queries.",
        icon: Globe,
        details: [
          "SQLx async support",
          "Diesel ORM integration",
          "PostgreSQL, MySQL, SQLite",
          "Connection pooling built-in",
        ],
      },
      {
        title: "Universal Deployment",
        description:
          "Deploy anywhere Rust runs - from edge to cloud to bare metal.",
        icon: Layers,
        details: [
          "Docker-friendly",
          "Edge runtime compatible",
          "Cloud platform support",
          "Static binary deployment",
        ],
      },
      {
        title: "Rust Ecosystem",
        description:
          "Works seamlessly with any Rust crate. Full access to the Rust ecosystem.",
        icon: Code2,
        details: [
          "Any Rust library works",
          "Tokio async runtime",
          "Serde serialization",
          "Rich crate ecosystem",
        ],
      },
    ],
  },
];

export function FeaturesSection() {
  return (
    <section className="py-24 bg-muted/30 relative overflow-hidden">
      <div className="absolute top-0 left-0 w-full h-full overflow-hidden -z-10 pointer-events-none">
        <div className="absolute top-[20%] left-[10%] w-[600px] h-[600px] bg-orange-500/10 blur-[120px] rounded-full" />
        <div className="absolute bottom-[10%] right-[15%] w-[500px] h-[500px] bg-red-500/10 blur-[100px] rounded-full" />
      </div>

      <div className="container px-4 md:px-6 mx-auto">
        <div className="text-center max-w-3xl mx-auto mb-16">
          <h2 className="text-3xl md:text-5xl font-bold mb-6 tracking-tight">
            Everything you need to ship
          </h2>
          <p className="text-muted-foreground text-lg">
            Ultimo combines the raw performance of Rust with the developer
            experience of modern full-stack frameworks.
          </p>
        </div>

        <Tabs defaultValue="core" className="w-full">
          <div className="flex justify-center mb-12">
            <TabsList className="bg-muted/50 backdrop-blur-sm border border-border">
              {featureCategories.map((category) => (
                <TabsTrigger
                  key={category.id}
                  value={category.id}
                  className="data-[state=active]:bg-orange-500/10 data-[state=active]:text-orange-500 data-[state=active]:border-orange-500/20"
                >
                  {category.label}
                </TabsTrigger>
              ))}
            </TabsList>
          </div>

          {featureCategories.map((category) => (
            <TabsContent key={category.id} value={category.id} className="mt-0">
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 max-w-6xl mx-auto">
                {category.features.map((feature, idx) => (
                  <div
                    key={idx}
                    className="group relative overflow-hidden rounded-xl border border-border bg-card p-6 hover:bg-accent/50 transition-all hover:border-orange-500/30"
                  >
                    <div className="absolute inset-0 bg-gradient-to-br from-orange-500/5 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />

                    <div className="relative z-10">
                      <div className="flex items-start gap-4 mb-4">
                        <div className="inline-flex h-12 w-12 items-center justify-center rounded-lg bg-orange-500/10 text-orange-500 border border-orange-500/20 group-hover:scale-110 transition-transform shrink-0">
                          <feature.icon className="h-6 w-6" />
                        </div>
                        <div className="flex-1 min-w-0">
                          <h3 className="text-lg font-bold mb-1 text-foreground group-hover:text-orange-500 transition-colors">
                            {feature.title}
                          </h3>
                          <p className="text-muted-foreground text-sm leading-relaxed">
                            {feature.description}
                          </p>
                        </div>
                      </div>

                      <div className="space-y-1.5 pl-16">
                        {feature.details.map((detail, detailIdx) => (
                          <div
                            key={detailIdx}
                            className="flex items-start gap-2 text-xs text-muted-foreground"
                          >
                            <span className="text-orange-500 mt-0.5">▸</span>
                            <span>{detail}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </TabsContent>
          ))}
        </Tabs>
      </div>
    </section>
  );
}
