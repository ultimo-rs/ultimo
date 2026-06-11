export function JsonLd() {
  const structuredData = {
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "Organization",
        name: "Ultimo",
        url: "https://ultimo.dev",
        logo: "https://ultimo.dev/icon.svg",
        sameAs: ["https://github.com/ultimo-rs/ultimo"],
        description:
          "Modern Rust web framework with automatic TypeScript client generation.",
      },
      {
        "@type": "WebSite",
        name: "Ultimo",
        url: "https://ultimo.dev",
        description:
          "Performance-equivalent to Axum with automatic TypeScript client generation. The modern full-stack framework for Rust.",
      },
      {
        "@type": "SoftwareApplication",
        name: "Ultimo",
        applicationCategory: "DeveloperApplication",
        operatingSystem: "Cross-platform",
        url: "https://ultimo.dev",
        offers: {
          "@type": "Offer",
          price: "0",
          priceCurrency: "USD",
        },
        description:
          "A modern Rust web framework: REST + JSON-RPC in one app, automatic TypeScript client generation, and WebSocket support with built-in pub/sub.",
      },
    ],
  };

  return (
    <script
      type="application/ld+json"
      dangerouslySetInnerHTML={{ __html: JSON.stringify(structuredData) }}
    />
  );
}
