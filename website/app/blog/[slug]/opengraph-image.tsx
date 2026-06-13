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
            <svg
              width="44"
              height="44"
              viewBox="410 360 205 310"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                fill="#ca3500"
                d="M607.332275,504.000061 C607.325745,531.494995 607.294312,558.489929 607.346252,585.484680 C607.351990,588.464844 606.914429,590.793030 604.239502,592.809204 C575.249878,614.659729 546.324097,636.595459 517.486328,658.645813 C514.470093,660.952148 512.407349,661.220764 509.285217,658.787476 C494.437103,647.215515 479.397339,635.889526 464.437622,624.460449 C463.270294,623.568604 461.866394,622.863892 461.402832,621.199707 C461.738007,619.164734 463.596191,618.413269 464.984375,617.332886 C489.177490,598.504211 513.360962,579.662415 537.663940,560.976318 C540.631287,558.694763 541.737915,556.304565 541.728333,552.592651 C541.612427,507.601562 541.702087,462.609955 541.577271,417.618927 C541.565918,413.511108 542.902283,410.948853 546.220398,408.520691 C564.773193,394.943878 583.144592,381.119385 601.604858,367.415741 C603.020752,366.364716 604.223633,364.783966 607.331726,364.954742 C607.331726,411.126068 607.331726,457.313049 607.332275,504.000061z"
              />
              <path
                fill="#f97316"
                d="M417.910645,580.828491 C417.870483,526.066223 417.848480,471.781677 417.742920,417.497284 C417.737000,414.448486 418.320068,412.166351 420.967529,410.233887 C440.600891,395.902435 460.139252,381.440887 479.722198,367.040222 C480.904022,366.171112 482.017792,365.067322 483.750183,365.120819 C485.773468,366.494324 485.143219,368.668732 485.145294,370.546783 C485.213379,432.657867 485.214691,494.769012 485.334961,556.879944 C485.339996,559.500427 484.560120,561.430542 482.468536,563.040771 C462.575134,578.363281 442.731476,593.751099 422.868835,609.114136 C421.372742,610.271667 419.836029,611.377991 417.910645,580.828491z"
              />
            </svg>
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
