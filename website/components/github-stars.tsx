"use client";

import { Github, Star } from "lucide-react";
import { useEffect, useState } from "react";
import { siteConfig } from "@/lib/config";
import { formatStars } from "@/lib/github";
import { cn } from "@/lib/utils";

const CACHE_KEY = "ultimo:stars";
const CACHE_MS = 60 * 60 * 1000;

/**
 * Star count that starts from the build-time value and refreshes once per
 * visitor per hour from the GitHub API (60 unauthenticated requests/hour per
 * IP is plenty for that). Keeps the build-time value if anything fails.
 */
export function useStarCount(initial: number | null): number | null {
  const [count, setCount] = useState(initial);

  useEffect(() => {
    try {
      const cached = JSON.parse(sessionStorage.getItem(CACHE_KEY) ?? "null");
      if (cached && Date.now() - cached.at < CACHE_MS) {
        setCount(cached.count);
        return;
      }
    } catch {
      // Storage unavailable (private mode, blocked) — fall through to fetch.
    }
    const controller = new AbortController();
    fetch(`https://api.github.com/repos/${siteConfig.repo}`, {
      signal: controller.signal,
    })
      .then((res) => (res.ok ? res.json() : null))
      .then((data) => {
        if (typeof data?.stargazers_count !== "number") return;
        setCount(data.stargazers_count);
        try {
          sessionStorage.setItem(
            CACHE_KEY,
            JSON.stringify({ count: data.stargazers_count, at: Date.now() }),
          );
        } catch {
          // Ignore storage errors.
        }
      })
      .catch(() => {});
    return () => controller.abort();
  }, []);

  return count;
}

/** "GitHub ★ 789" button linking to the repo. */
export function GitHubStarsButton({
  initial,
  className,
}: {
  initial: number | null;
  className?: string;
}) {
  const count = useStarCount(initial);
  const label =
    count === null
      ? "Ultimo on GitHub"
      : `Star Ultimo on GitHub — ${count.toLocaleString("en-US")} stars`;

  return (
    <a
      href={siteConfig.links.github}
      target="_blank"
      rel="noopener noreferrer"
      aria-label={label}
      className={cn(
        "inline-flex h-8 items-center gap-2 rounded-md border border-border/60 bg-background/40 px-2.5 text-sm font-medium text-muted-foreground transition-colors hover:border-border hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-orange-500/60",
        className,
      )}
    >
      <Github className="h-4 w-4" aria-hidden="true" />
      <span>GitHub</span>
      {count !== null && (
        <span className="flex items-center gap-1 border-l border-border/60 pl-2 tabular-nums text-foreground">
          <Star
            className="h-3.5 w-3.5 fill-orange-400 text-orange-400"
            aria-hidden="true"
          />
          {formatStars(count)}
        </span>
      )}
    </a>
  );
}
