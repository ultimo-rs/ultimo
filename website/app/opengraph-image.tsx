import { ImageResponse } from "next/og";

export const dynamic = "force-static";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default async function OGImage() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: "#0a0a0a",
          padding: "60px",
          fontFamily: "system-ui, sans-serif",
          gap: "32px",
        }}
      >
        {/* Logo */}
        <div
          style={{
            width: "80px",
            height: "80px",
            backgroundColor: "#ca3500",
            borderRadius: "16px",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            color: "#fff",
            fontSize: 44,
            fontWeight: 800,
          }}
        >
          U
        </div>

        {/* Title */}
        <div
          style={{
            display: "flex",
            fontSize: 64,
            fontWeight: 800,
            color: "#ffffff",
            textAlign: "center",
          }}
        >
          <div style={{ display: "flex" }}>The&nbsp;</div>
          <div style={{ display: "flex", color: "#f97316" }}>Rust Framework</div>
        </div>

        {/* Subtitle */}
        <div
          style={{
            display: "flex",
            fontSize: 32,
            color: "#a1a1aa",
            textAlign: "center",
          }}
        >
          for Speed, Security & Efficiency
        </div>

        {/* CTA */}
        <div
          style={{
            display: "flex",
            backgroundColor: "#f97316",
            color: "#000",
            padding: "16px 40px",
            borderRadius: "10px",
            fontSize: 22,
            fontWeight: 700,
            marginTop: "16px",
          }}
        >
          Start Building → ultimo.dev
        </div>

        {/* Pillars */}
        <div
          style={{
            display: "flex",
            gap: "40px",
            marginTop: "8px",
            color: "#71717a",
            fontSize: 18,
          }}
        >
          <div style={{ display: "flex" }}>Zero GC</div>
          <div style={{ display: "flex" }}>•</div>
          <div style={{ display: "flex" }}>100% Safe Rust</div>
          <div style={{ display: "flex" }}>•</div>
          <div style={{ display: "flex" }}>Auto TypeScript</div>
          <div style={{ display: "flex" }}>•</div>
          <div style={{ display: "flex" }}>Single Binary</div>
        </div>
      </div>
    ),
    { ...size }
  );
}
