import { defineConfig } from "vocs";

export default defineConfig({
  title: "Ultimo",
  rootDir: "docs",
  description: "Modern Rust web framework with automatic TypeScript generation",
  logoUrl: "/logo.svg",
  iconUrl: "/favicon.ico",
  ogImageUrl: "https://docs.ultimo.dev/og-image.png",
  topNav: [
    { text: "Docs", link: "/getting-started" },
    {
      text: "Examples",
      link: "https://github.com/ultimo-rs/ultimo/tree/main/examples",
    },
    { text: "GitHub", link: "https://github.com/ultimo-rs/ultimo" },
  ],
  sidebar: [
    {
      text: "Introduction",
      items: [
        {
          text: "Getting Started",
          link: "/getting-started",
        },
        {
          text: "API Reference",
          link: "/api-reference",
        },
        {
          text: "Performance",
          link: "/performance",
        },
        {
          text: "Roadmap",
          link: "/roadmap",
        },
        {
          text: "Changelog",
          link: "/changelog",
        },
      ],
    },
    {
      text: "Core Concepts",
      items: [
        {
          text: "Routing",
          link: "/routing",
        },
        {
          text: "Middleware",
          link: "/middleware",
        },
        {
          text: "RPC System",
          link: "/rpc",
        },
      ],
    },
    {
      text: "Features",
      items: [
        {
          text: "Security",
          link: "/security",
        },
        {
          text: "Authentication",
          collapsed: false,
          items: [
            {
              text: "Sessions",
              link: "/sessions",
            },
            {
              text: "JWT",
              link: "/jwt",
            },
            {
              text: "API Keys",
              link: "/api-keys",
            },
            {
              text: "Authorization (Guards)",
              link: "/authorization",
            },
          ],
        },
        {
          text: "Static Files",
          link: "/static-files",
        },
        {
          text: "WebSocket",
          link: "/websocket",
        },
        {
          text: "TypeScript Clients",
          link: "/typescript",
        },
        {
          text: "OpenAPI Support",
          link: "/openapi",
        },
        {
          text: "CLI Tools",
          link: "/cli",
        },
        {
          text: "Testing",
          link: "/testing",
        },
      ],
    },
    {
      text: "Integrations",
      items: [
        {
          text: "Database",
          link: "/database",
        },
        {
          text: "SQLx",
          link: "/sqlx",
        },
        {
          text: "Diesel",
          link: "/diesel",
        },
      ],
    },
    {
      text: "Deployment",
      items: [
        {
          text: "Overview",
          link: "/deployment",
        },
        {
          text: "Docker",
          link: "/deployment/docker",
        },
        {
          text: "Fly.io",
          link: "/deployment/fly-io",
        },
        {
          text: "Railway",
          link: "/deployment/railway",
        },
        {
          text: "AWS (ECS/Fargate)",
          link: "/deployment/aws",
        },
        {
          text: "Google Cloud Run",
          link: "/deployment/google-cloud-run",
        },
        {
          text: "Azure Container Apps",
          link: "/deployment/azure",
        },
        {
          text: "DigitalOcean",
          link: "/deployment/digitalocean",
        },
        {
          text: "Kubernetes",
          link: "/deployment/kubernetes",
        },
      ],
    },
  ],
  socials: [
    {
      icon: "github",
      link: "https://github.com/ultimo-rs/ultimo",
    },
  ],
  theme: {
    accentColor: {
      light: "#0ea5e9",
      dark: "#38bdf8",
    },
  },
});
