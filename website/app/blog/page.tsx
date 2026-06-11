import { Calendar, Tag } from "lucide-react";
import type { Metadata } from "next";
import Link from "next/link";
import { getAllPosts } from "@/lib/blog";

export const metadata: Metadata = {
  title: "Blog - Ultimo Framework",
  description:
    "Tutorials, release notes, architecture deep-dives, and comparisons for the Ultimo Rust web framework.",
};

export default function BlogPage() {
  const posts = getAllPosts();

  return (
    <div className="min-h-screen selection:bg-orange-500/30">
      <main className="container mx-auto px-4 py-24 max-w-5xl">
        <div className="text-center mb-16">
          <span className="inline-block px-3 py-1 text-xs font-medium text-orange-400 bg-orange-500/10 border border-orange-500/20 rounded-full mb-4">
            Blog
          </span>
          <h1 className="text-4xl md:text-5xl font-bold tracking-tight mb-4">
            Latest from{" "}
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-orange-400 to-orange-600">
              Ultimo
            </span>
          </h1>
          <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
            Tutorials, release notes, architecture deep-dives, and comparisons
            for the modern Rust web framework.
          </p>
        </div>

        {posts.length === 0 ? (
          <p className="text-center text-muted-foreground">
            No posts yet. Check back soon!
          </p>
        ) : (
          <div className="grid gap-8 md:grid-cols-2 lg:grid-cols-3">
            {posts.map((post) => (
              <Link
                key={post.slug}
                href={`/blog/${post.slug}`}
                className="group block rounded-xl border border-border/50 bg-card/50 p-6 transition-all hover:border-orange-500/30 hover:bg-card/80"
              >
                <div className="flex flex-wrap gap-2 mb-3">
                  {post.tags.slice(0, 2).map((tag) => (
                    <span
                      key={tag}
                      className="inline-flex items-center gap-1 text-xs text-orange-400 bg-orange-500/10 px-2 py-0.5 rounded"
                    >
                      <Tag className="w-3 h-3" />
                      {tag}
                    </span>
                  ))}
                </div>
                <h2 className="text-xl font-semibold mb-2 group-hover:text-orange-400 transition-colors">
                  {post.title}
                </h2>
                <p className="text-sm text-muted-foreground mb-4 line-clamp-3">
                  {post.description}
                </p>
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Calendar className="w-3 h-3" />
                  <time dateTime={post.date}>
                    {new Date(post.date).toLocaleDateString("en-US", {
                      year: "numeric",
                      month: "long",
                      day: "numeric",
                    })}
                  </time>
                </div>
              </Link>
            ))}
          </div>
        )}
      </main>
    </div>
  );
}
