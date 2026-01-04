import type React from "next";
import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";

const geistSans = Geist({
  subsets: ["latin"],
  variable: "--font-sans",
});
const geistMono = Geist_Mono({
  subsets: ["latin"],
  variable: "--font-mono",
});

const siteUrl = "https://ultimo.dev";
const title = "Ultimo - Modern Rust Web Framework";
const description =
  "Performance-equivalent to Axum with automatic TypeScript client generation. The modern full-stack framework for Rust.";

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: {
    default: title,
    template: "%s | Ultimo",
  },
  description,
  keywords: [
    "Rust",
    "Web Framework",
    "TypeScript",
    "Full Stack",
    "Axum",
    "RPC",
    "REST API",
    "WebSocket",
  ],
  authors: [{ name: "Ultimo Team" }],
  creator: "Ultimo Team",
  openGraph: {
    type: "website",
    locale: "en_US",
    url: siteUrl,
    title,
    description,
    siteName: "Ultimo",
    images: [
      {
        url: `${siteUrl}/api/og?title=Ultimo&description=${encodeURIComponent(description)}`,
        width: 1200,
        height: 630,
        alt: "Ultimo - Modern Rust Web Framework",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
    images: [`${siteUrl}/api/og?title=Ultimo&description=${encodeURIComponent(description)}`],
  },
  icons: {
    icon: "/favicon.ico",
    shortcut: "/favicon.ico",
    apple: "/apple-icon.png",
  },
  manifest: "/manifest.webmanifest",
  generator: "v0.app",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body
        className={`${geistSans.variable} ${geistMono.variable} min-h-screen bg-background text-foreground selection:bg-orange-500/30`}
      >
        <SiteHeader />
        {children}
        <SiteFooter />
      </body>
    </html>
  );
}
