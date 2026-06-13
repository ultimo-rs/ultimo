import { ImageResponse } from "next/og";
import { getAllPosts, getPostBySlug } from "@/lib/blog";

export const dynamic = "force-static";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export function generateStaticParams() {
  const posts = getAllPosts();
  return posts.map((post) => ({ slug: post.slug }));
}

export default async function OGImage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const post = getPostBySlug(slug);

  if (!post) {
    return new ImageResponse(
      (
        <div
          style={{
            width: "100%",
            height: "100%",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: "#0a0a0a",
            color: "#fff",
            fontSize: 48,
            fontWeight: 700,
          }}
        >
          Ultimo Blog
        </div>
      ),
      { ...size }
    );
  }

  const tagColors: Record<string, string> = {
    tutorial: "#22c55e",
    architecture: "#3b82f6",
    comparison: "#a855f7",
    performance: "#f59e0b",
    fintech: "#06b6d4",
    realtime: "#ec4899",
    typescript: "#3178c6",
    security: "#ef4444",
    rust: "#ca3500",
  };

  const primaryTag = post.meta.tags[0] || "rust";
  const tagColor = tagColors[primaryTag] || "#f97316";

  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          backgroundColor: "#0a0a0a",
          padding: "60px",
          fontFamily: "system-ui, sans-serif",
        }}
      >
        {/* Top: tag + title */}
        <div style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "12px",
            }}
          >
            <div
              style={{
                display: "flex",
                backgroundColor: tagColor,
                color: "#fff",
                padding: "6px 16px",
                borderRadius: "20px",
                fontSize: 18,
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.05em",
              }}
            >
              {primaryTag}
            </div>
            <div style={{ display: "flex", color: "#a1a1aa", fontSize: 18 }}>
              {`${post.meta.readingTime} min read`}
            </div>
          </div>

          <div
            style={{
              display: "flex",
              fontSize: 52,
              fontWeight: 800,
              color: "#ffffff",
              lineHeight: 1.2,
              maxWidth: "900px",
            }}
          >
            {post.meta.title}
          </div>
        </div>

        {/* Bottom: branding + CTA */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "16px" }}>
            <div
              style={{
                width: "44px",
                height: "44px",
                backgroundColor: "#ca3500",
                borderRadius: "8px",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "#fff",
                fontSize: 24,
                fontWeight: 800,
              }}
            >
              U
            </div>
            <div style={{ display: "flex", flexDirection: "column" }}>
              <div
                style={{ display: "flex", color: "#ffffff", fontSize: 22, fontWeight: 700 }}
              >
                ultimo.dev
              </div>
              <div style={{ display: "flex", color: "#a1a1aa", fontSize: 16 }}>
                The Rust Framework for Speed, Security & Efficiency
              </div>
            </div>
          </div>

          <div
            style={{
              display: "flex",
              backgroundColor: "#f97316",
              color: "#000",
              padding: "12px 28px",
              borderRadius: "8px",
              fontSize: 18,
              fontWeight: 700,
            }}
          >
            Read on ultimo.dev →
          </div>
        </div>
      </div>
    ),
    { ...size }
  );
}
