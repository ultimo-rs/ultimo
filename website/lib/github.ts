import { siteConfig } from "@/lib/config";

/**
 * The repo's GitHub star count, fetched once at build time (the site is a
 * static export). Returns `null` if GitHub can't be reached, so the UI can
 * fall back to a plain "GitHub" link. Set `GITHUB_TOKEN` in the build
 * environment to avoid the unauthenticated rate limit.
 */
export async function getStarCount(): Promise<number | null> {
  try {
    const res = await fetch(`https://api.github.com/repos/${siteConfig.repo}`, {
      headers: {
        Accept: "application/vnd.github+json",
        ...(process.env.GITHUB_TOKEN
          ? { Authorization: `Bearer ${process.env.GITHUB_TOKEN}` }
          : {}),
      },
      signal: AbortSignal.timeout(5000),
    });
    if (!res.ok) return null;
    const data = (await res.json()) as { stargazers_count?: unknown };
    return typeof data.stargazers_count === "number"
      ? data.stargazers_count
      : null;
  } catch {
    return null;
  }
}

/** 789 → "789", 1234 → "1.2k", 12500 → "12.5k". */
export function formatStars(count: number): string {
  if (count < 1000) return String(count);
  return `${(count / 1000).toFixed(1).replace(/\.0$/, "")}k`;
}
