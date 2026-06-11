import { ArrowRight, Calendar, Tag } from "lucide-react";
import Link from "next/link";
import { getAllPosts } from "@/lib/blog";

export function BlogSection() {
  const posts = getAllPosts().slice(0, 3);

  if (posts.length === 0) return null;

  return (
    <section className="py-24 px-4">
      <div className="container mx-auto max-w-6xl">
        <div className="text-center mb-12">
          <span className="inline-block px-3 py-1 text-xs font-medium text-orange-400 bg-orange-500/10 border border-orange-500/20 rounded-full mb-4">
            Blog
          </span>
          <h2 className="text-3xl md:text-4xl font-bold tracking-tight mb-4">
            Latest from the Blog
          </h2>
          <p className="text-muted-foreground max-w-xl mx-auto">
            Tutorials, release notes, and deep dives into the Ultimo framework.
          </p>
        </div>

        <div className="grid gap-6 md:grid-cols-3">
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
              <h3 className="text-lg font-semibold mb-2 group-hover:text-orange-400 transition-colors line-clamp-2">
                {post.title}
              </h3>
              <p className="text-sm text-muted-foreground mb-4 line-clamp-2">
                {post.description}
              </p>
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <Calendar className="w-3 h-3" />
                <time dateTime={post.date}>
                  {new Date(post.date).toLocaleDateString("en-US", {
                    month: "short",
                    day: "numeric",
                    year: "numeric",
                  })}
                </time>
              </div>
            </Link>
          ))}
        </div>

        <div className="text-center mt-10">
          <Link
            href="/blog"
            className="inline-flex items-center gap-2 text-sm font-medium text-orange-400 hover:text-orange-300 transition-colors"
          >
            View all posts
            <ArrowRight className="w-4 h-4" />
          </Link>
        </div>
      </div>
    </section>
  );
}
