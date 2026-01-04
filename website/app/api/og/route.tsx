import { ImageResponse } from "next/og";
import { NextRequest } from "next/server";

export const runtime = "edge";

export async function GET(req: NextRequest) {
  try {
    const { searchParams } = new URL(req.url);

    // Get parameters from URL
    const title = searchParams.get("title") || "Ultimo";
    const description =
      searchParams.get("description") ||
      "Modern Rust Web Framework with TypeScript generation";

    return new ImageResponse(
      (
        <div
          style={{
            height: "100%",
            width: "100%",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: "#0a0a0a",
            backgroundImage:
              "radial-gradient(circle at 25px 25px, #1a1a1a 2%, transparent 0%), radial-gradient(circle at 75px 75px, #1a1a1a 2%, transparent 0%)",
            backgroundSize: "100px 100px",
          }}
        >
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              padding: "80px",
            }}
          >
            <div
              style={{
                fontSize: 120,
                fontWeight: 900,
                background: "linear-gradient(to right, #ff6b35, #ff8c42)",
                backgroundClip: "text",
                color: "transparent",
                marginBottom: 20,
              }}
            >
              {title}
            </div>
            <div
              style={{
                fontSize: 36,
                color: "#999",
                textAlign: "center",
                maxWidth: "80%",
              }}
            >
              {description}
            </div>
          </div>
        </div>
      ),
      {
        width: 1200,
        height: 630,
      }
    );
  } catch (e: any) {
    console.log(`${e.message}`);
    return new Response(`Failed to generate the image`, {
      status: 500,
    });
  }
}
