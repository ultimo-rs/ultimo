import { defineConfig } from "vocs";

export default defineConfig({
  title: "Ultimo",
  rootDir: "docs",
  description: "Modern Rust web framework with automatic TypeScript generation",
  logoUrl: "/logo.svg",
  head: {
    meta: [
      {
        property: "og:type",
        content: "website",
      },
      {
        property: "og:title",
        content: "Ultimo Documentation",
      },
      {
        property: "og:description",
        content: "Modern Rust web framework with automatic TypeScript generation",
      },
      {
        property: "og:url",
        content: "https://docs.ultimo.dev",
      },
      {
        property: "og:image",
        content: "https://ultimo.dev/api/og?title=Ultimo%20Docs&description=Modern%20Rust%20web%20framework%20with%20automatic%20TypeScript%20generation",
      },
      {
        name: "twitter:card",
        content: "summary_large_image",
      },
      {
        name: "twitter:title",
        content: "Ultimo Documentation",
      },
      {
        name: "twitter:description",
        content: "Modern Rust web framework with automatic TypeScript generation",
      },
      {
        name: "twitter:image",
        content: "https://ultimo.dev/api/og?title=Ultimo%20Docs&description=Modern%20Rust%20web%20framework%20with%20automatic%20TypeScript%20generation",
      },
    ],
    link: [
      {
        rel: "icon",
        href: "/favicon.ico",
      },
      {
        rel: "apple-touch-icon",
        href: "/apple-icon.png",
      },
    ],
  },
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
