import { ArrowLeft, Calendar, User } from "lucide-react";
import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { MDXRemote } from "next-mdx-remote/rsc";
import rehypePrettyCode from "rehype-pretty-code";
import rehypeSlug from "rehype-slug";
import { getAllPosts, getPostBySlug } from "@/lib/blog";
import { TableOfContents } from "@/components/table-of-contents";

interface PageProps {
  params: Promise<{ slug: string }>;
}

export async function generateStaticParams() {
  const posts = getAllPosts();
  return posts.map((post) => ({ slug: post.slug }));
}

export async function generateMetadata({
  params,
}: PageProps): Promise<Metadata> {
  const { slug } = await params;
  const post = getPostBySlug(slug);
  if (!post) return {};

  return {
    title: `${post.meta.title} - Ultimo Blog`,
    description: post.meta.description,
    openGraph: {
      title: post.meta.title,
      description: post.meta.description,
      type: "article",
      publishedTime: post.meta.date,
      authors: [post.meta.author],
    },
  };
}

export default async function BlogPostPage({ params }: PageProps) {
  const { slug } = await params;
  const post = getPostBySlug(slug);

  if (!post) {
    notFound();
  }

  // Strip the first H1 from content since we render it in the header
  const content = post.content.replace(/^#\s+.+\n+/, "");

  return (
    <div className="min-h-screen selection:bg-orange-500/30">
      <div className="container mx-auto px-4 py-24 max-w-6xl">
        <Link
          href="/blog"
          className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-orange-400 transition-colors mb-8"
        >
          <ArrowLeft className="w-4 h-4" />
          Back to blog
        </Link>

        <div className="lg:grid lg:grid-cols-[1fr_220px] lg:gap-12">
          <article>
            <header className="mb-12">
              <div className="flex flex-wrap gap-2 mb-4">
                {post.meta.tags.map((tag) => (
                  <span
                    key={tag}
                    className="text-xs text-orange-400 bg-orange-500/10 px-2 py-0.5 rounded"
                  >
                    {tag}
                  </span>
                ))}
              </div>
              <h1 className="text-3xl md:text-4xl font-bold tracking-tight mb-4">
                {post.meta.title}
              </h1>
              <div className="flex items-center gap-4 text-sm text-muted-foreground">
                <span className="flex items-center gap-1.5">
                  <User className="w-4 h-4" />
                  {post.meta.author}
                </span>
                <span className="flex items-center gap-1.5">
                  <Calendar className="w-4 h-4" />
                  <time dateTime={post.meta.date}>
                    {new Date(post.meta.date).toLocaleDateString("en-US", {
                      year: "numeric",
                      month: "long",
                      day: "numeric",
                    })}
                  </time>
                </span>
              </div>
            </header>

            <div className="prose prose-invert prose-orange max-w-none prose-headings:font-semibold prose-headings:tracking-tight prose-a:text-orange-400 prose-a:no-underline hover:prose-a:underline prose-code:text-orange-300 prose-pre:bg-[#0d1117] prose-pre:border prose-pre:border-border/50 prose-img:rounded-lg">
              <MDXRemote
                source={content}
                options={{
                  mdxOptions: {
                    rehypePlugins: [
                      rehypeSlug,
                      [rehypePrettyCode, { theme: "github-dark-dimmed" }],
                    ],
                  },
                }}
              />
            </div>

            <footer className="mt-16 pt-8 border-t border-border/50">
              <Link
                href="/blog"
                className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-orange-400 transition-colors"
              >
                <ArrowLeft className="w-4 h-4" />
                Back to all posts
              </Link>
            </footer>
          </article>

          <aside className="hidden lg:block">
            <div className="sticky top-24">
              <TableOfContents content={content} />
            </div>
          </aside>
        </div>
      </div>
    </div>
  );
}
