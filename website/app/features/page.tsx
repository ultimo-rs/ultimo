import {
  ArrowRight,
  BookOpen,
  Boxes,
  Code2,
  Database,
  FileType,
  Gauge,
  Globe,
  Layers,
  Lock,
  ShieldCheck,
  Terminal,
  Workflow,
  Zap,
} from "lucide-react";
import type { Metadata } from "next";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { siteConfig } from "@/lib/config";

export const metadata: Metadata = {
  title: "Features - Ultimo Framework",
  description:
    "Explore the powerful features of Ultimo: blazing fast performance, type-safe RPC, automatic TypeScript client generation, OpenAPI support, and more.",
};

const features = [
  {
    title: "Blazing Fast Performance",
    description:
      "Ultimo matches Axum's performance with 152k+ requests per second and sub-millisecond latency. Built on proven foundations like Hyper and Tokio, you get raw Rust speed without compromise.",
    icon: Zap,
    highlights: [
      "152k+ req/sec throughput",
      "Sub-millisecond latency",
      "Zero-copy parsing where possible",
      "Efficient async runtime (Tokio)",
      "Optimized for modern CPUs",
    ],
  },
  {
    title: "Type-Safe RPC with TypeScript Generation",
    description:
      "Define your API once in Rust and get fully typed TypeScript clients automatically. No more manual type syncing, API drift, or runtime errors from mismatched types.",
    icon: FileType,
    highlights: [
      "Automatic TypeScript client generation",
      "End-to-end type safety",
      "Zero runtime overhead",
      "IDE autocomplete for all endpoints",
      "Catch errors at compile time, not production",
    ],
  },
  {
    title: "OpenAPI 3.0 Specification",
    description:
      "Generate OpenAPI 3.0 specifications directly from your Rust code. Integrate with Swagger UI, Postman, and any OpenAPI-compatible tool ecosystem.",
    icon: BookOpen,
    highlights: [
      "Auto-generated OpenAPI 3.0 specs",
      "Swagger UI integration",
      "Compatible with all OpenAPI tools",
      "Request/response validation",
      "Interactive API documentation",
    ],
  },
  {
    title: "Hybrid REST + RPC Design",
    description:
      "Support both traditional REST endpoints and efficient JSON-RPC procedures in the same application. Choose the right paradigm for each use case without architectural compromises.",
    icon: Layers,
    highlights: [
      "REST and JSON-RPC in one app",
      "Flexible routing system",
      "Consistent middleware pipeline",
      "Choose the best tool for each job",
      "Gradual migration path",
    ],
  },
  {
    title: "Built-in Security & Validation",
    description:
      "Authentication, authorization, CORS, rate limiting, and request validation are included out of the box. Focus on building features, not security infrastructure.",
    icon: ShieldCheck,
    highlights: [
      "JWT authentication support",
      "Role-based access control",
      "Request/response validation",
      "CORS middleware",
      "Rate limiting built-in",
    ],
  },
  {
    title: "Developer-First Experience",
    description:
      "Ergonomic APIs, helpful error messages, comprehensive CLI tools, and excellent documentation. Designed to make you productive from day one.",
    icon: Code2,
    highlights: [
      "Intuitive, chainable APIs",
      "Clear, actionable error messages",
      "CLI for scaffolding & development",
      "Hot reload in development",
      "Comprehensive documentation",
    ],
  },
  {
    title: "Database Agnostic",
    description:
      "Works seamlessly with SQLx, Diesel, SeaORM, or any Rust database library. Use your preferred ORM or go with raw SQL—Ultimo doesn't lock you in.",
    icon: Database,
    highlights: [
      "Works with SQLx, Diesel, SeaORM",
      "Connection pooling support",
      "Transaction middleware",
      "Migration tools compatibility",
      "No vendor lock-in",
    ],
  },
  {
    title: "Universal Deployment",
    description:
      "Deploy anywhere Rust runs: traditional servers, containers, edge computing, or serverless. Compile to a single binary with no runtime dependencies.",
    icon: Globe,
    highlights: [
      "Single binary deployment",
      "No runtime dependencies",
      "Docker & Kubernetes ready",
      "Edge computing compatible",
      "Minimal resource footprint",
    ],
  },
  {
    title: "Advanced Middleware System",
    description:
      "Compose behavior with a powerful, type-safe middleware system. Built-in middleware for common tasks, easy to write custom middleware.",
    icon: Workflow,
    highlights: [
      "Type-safe middleware composition",
      "Built-in common middleware",
      "Request/response transformation",
      "Error handling pipeline",
      "Async-first design",
    ],
  },
  {
    title: "Production-Ready Tooling",
    description:
      "CLI tools for project scaffolding, development server with hot reload, built-in testing utilities, and deployment helpers.",
    icon: Terminal,
    highlights: [
      "Project scaffolding CLI",
      "Development server with hot reload",
      "Testing utilities included",
      "Build optimization tools",
      "Deployment helpers",
    ],
  },
  {
    title: "Extensible Architecture",
    description:
      "Plugin system for extending functionality. Works with the entire Rust ecosystem. Add any crate without fighting the framework.",
    icon: Boxes,
    highlights: [
      "Plugin system for extensions",
      "Compatible with all Rust crates",
      "Custom handler registration",
      "Service injection support",
      "Framework doesn't get in your way",
    ],
  },
  {
    title: "Optimized for Scale",
    description:
      "Built to handle everything from prototypes to high-traffic production systems. Efficient memory usage, connection pooling, and request pipelining.",
    icon: Gauge,
    highlights: [
      "Efficient memory management",
      "Connection pooling built-in",
      "Request pipelining support",
      "Graceful shutdown",
      "Health check endpoints",
    ],
  },
];

export default function FeaturesPage() {
  return (
    <div className="min-h-screen">
      {/* Hero Section */}
      <section className="relative pt-32 pb-20 overflow-hidden">
        <div className="absolute inset-0 -z-10">
          <div className="absolute inset-0 bg-gradient-to-b from-orange-500/5 via-background to-background" />
          <div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[600px] bg-orange-500/10 blur-[120px] rounded-full" />
        </div>

        <div className="container px-4 md:px-6 mx-auto text-center">
          <div className="inline-flex items-center rounded-full border border-orange-500/30 bg-orange-500/10 px-3 py-1 text-sm font-medium text-orange-400 mb-6 backdrop-blur-sm">
            <Zap className="w-4 h-4 mr-2" />
            Powerful Features
          </div>

          <h1 className="text-4xl md:text-6xl font-bold mb-6 max-w-4xl mx-auto">
            Everything you need to build
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-orange-500 to-orange-400">
              {" "}
              high-performance backends
            </span>
          </h1>

          <p className="text-xl text-muted-foreground max-w-3xl mx-auto mb-12 leading-relaxed">
            Ultimo combines the raw performance of Rust with the developer
            experience of modern full-stack frameworks. Type safety, automatic
            client generation, and production-ready features out of the box.
          </p>

          <div className="flex flex-col sm:flex-row gap-4 justify-center">
            <Button size="lg" asChild>
              <Link href={siteConfig.nav.getStarted}>
                Get Started
                <ArrowRight className="ml-2 h-4 w-4" />
              </Link>
            </Button>
            <Button size="lg" variant="outline" asChild>
              <Link href={siteConfig.nav.examples}>View Examples</Link>
            </Button>
          </div>
        </div>
      </section>

      {/* Features Grid */}
      <section className="py-20 relative">
        <div className="container px-4 md:px-6 mx-auto">
          <div className="grid gap-12 md:gap-16">
            {features.map((feature, index) => (
              <div
                key={index}
                className="grid md:grid-cols-2 gap-8 items-start"
              >
                <div className={index % 2 === 1 ? "md:order-2" : ""}>
                  <div className="inline-flex h-14 w-14 items-center justify-center rounded-xl bg-orange-500/10 text-orange-500 border border-orange-500/20 mb-6">
                    <feature.icon className="h-7 w-7" />
                  </div>
                  <h2 className="text-3xl font-bold mb-4">{feature.title}</h2>
                  <p className="text-lg text-muted-foreground leading-relaxed">
                    {feature.description}
                  </p>
                </div>

                <div className={index % 2 === 1 ? "md:order-1" : ""}>
                  <div className="rounded-xl border border-border bg-card p-8">
                    <h3 className="font-semibold mb-4 text-foreground">
                      Key Features:
                    </h3>
                    <ul className="space-y-3">
                      {feature.highlights.map((highlight, i) => (
                        <li key={i} className="flex items-start gap-3">
                          <div className="rounded-full bg-orange-500/10 p-1 mt-0.5">
                            <ArrowRight className="h-3 w-3 text-orange-500" />
                          </div>
                          <span className="text-muted-foreground">
                            {highlight}
                          </span>
                        </li>
                      ))}
                    </ul>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="py-24 relative overflow-hidden">
        <div className="absolute inset-0 -z-10">
          <div className="absolute inset-0 bg-gradient-to-t from-orange-500/10 to-transparent" />
          <div className="absolute bottom-0 left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-orange-500/15 blur-[120px] rounded-full" />
        </div>

        <div className="container px-4 md:px-6 mx-auto text-center">
          <h2 className="text-3xl md:text-4xl font-bold mb-6">
            Ready to experience the future of Rust web development?
          </h2>
          <p className="text-lg text-muted-foreground max-w-2xl mx-auto mb-10">
            Join developers building faster, safer, and more maintainable
            applications with Ultimo.
          </p>
          <div className="flex flex-col sm:flex-row gap-4 justify-center">
            <Button size="lg" asChild>
              <Link href={siteConfig.nav.getStarted}>
                Get Started
                <ArrowRight className="ml-2 h-4 w-4" />
              </Link>
            </Button>
            <Button size="lg" variant="outline" asChild>
              <Link href={siteConfig.nav.documentation}>
                Read Documentation
              </Link>
            </Button>
          </div>
        </div>
      </section>
    </div>
  );
}
