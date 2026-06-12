import { ArrowLeft, Calendar, Clock, User } from "lucide-react";
import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { MDXRemote } from "next-mdx-remote/rsc";
import rehypePrettyCode from "rehype-pretty-code";
import rehypeSlug from "rehype-slug";
import remarkGfm from "remark-gfm";
import { TableOfContents } from "@/components/table-of-contents";
import { getAllPosts, getPostBySlug, getRelatedPosts } from "@/lib/blog";

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

  const url = `https://ultimo.dev/blog/${slug}`;

  return {
    title: `${post.meta.title} - Ultimo Blog`,
    description: post.meta.description,
    alternates: { canonical: url },
    openGraph: {
      title: post.meta.title,
      description: post.meta.description,
      type: "article",
      url,
      publishedTime: post.meta.date,
      authors: [post.meta.author],
      tags: post.meta.tags,
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
  const content = post.content.replace(/^\s*#\s+.+\n+/, "");
  const relatedPosts = getRelatedPosts(slug, 3);

  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "BlogPosting",
    headline: post.meta.title,
    description: post.meta.description,
    datePublished: post.meta.date,
    author: { "@type": "Person", name: post.meta.author },
    publisher: {
      "@type": "Organization",
      name: "Ultimo",
      url: "https://ultimo.dev",
    },
    url: `https://ultimo.dev/blog/${slug}`,
    wordCount: content.split(/\s+/).length,
    keywords: post.meta.tags.join(", "),
  };

  const breadcrumbLd = {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: [
      { "@type": "ListItem", position: 1, name: "Home", item: "https://ultimo.dev" },
      { "@type": "ListItem", position: 2, name: "Blog", item: "https://ultimo.dev/blog" },
      { "@type": "ListItem", position: 3, name: post.meta.title, item: `https://ultimo.dev/blog/${slug}` },
    ],
  };

  // HowTo schema for the tutorial post
  const howToLd = slug === "build-your-first-api-with-ultimo" ? {
    "@context": "https://schema.org",
    "@type": "HowTo",
    name: "Build Your First API with Ultimo in 10 Minutes",
    description: post.meta.description,
    totalTime: "PT10M",
    step: [
      { "@type": "HowToStep", position: 1, name: "Create Your Project", text: "Create a new Rust project and install the Ultimo CLI with cargo." },
      { "@type": "HowToStep", position: 2, name: "Configure Dependencies", text: "Add ultimo, tokio, and serde to your Cargo.toml." },
      { "@type": "HowToStep", position: 3, name: "Define Your Data Model", text: "Create Rust structs with Serialize, Deserialize, and TS derives." },
      { "@type": "HowToStep", position: 4, name: "Create REST Endpoints", text: "Define route handlers for CRUD operations using Ultimo's Context API." },
      { "@type": "HowToStep", position: 5, name: "Add JSON-RPC Methods", text: "Register RPC query and mutation handlers in the RPC registry." },
      { "@type": "HowToStep", position: 6, name: "Wire Up the Server", text: "Configure the Ultimo app with routes, RPC, and start listening." },
      { "@type": "HowToStep", position: 7, name: "Test Your API", text: "Use curl to test REST and JSON-RPC endpoints." },
      { "@type": "HowToStep", position: 8, name: "Generate TypeScript Client", text: "Run ultimo generate to create a fully typed TypeScript client." },
    ],
  } : null;

  return (
    <div className="min-h-screen selection:bg-orange-500/30">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(breadcrumbLd) }}
      />
      {howToLd && (
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(howToLd) }}
        />
      )}
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
                <span className="flex items-center gap-1.5">
                  <Clock className="w-4 h-4" />
                  {post.meta.readingTime} min read
                </span>
              </div>
            </header>

            <div className="prose prose-invert prose-orange max-w-none prose-headings:font-semibold prose-headings:tracking-tight prose-a:text-orange-400 prose-a:no-underline prose-a:hover:underline prose-code:text-orange-300 prose-pre:bg-[#0d1117] prose-pre:border prose-pre:border-border/50 prose-img:rounded-lg">
              <MDXRemote
                source={content}
                options={{
                  mdxOptions: {
                    remarkPlugins: [remarkGfm],
                    rehypePlugins: [
                      rehypeSlug,
                      [rehypePrettyCode, { theme: "github-dark-dimmed" }],
                    ],
                  },
                }}
              />
            </div>

            <footer className="mt-16 pt-8 border-t border-border/50">
              {relatedPosts.length > 0 && (
                <div className="mb-12">
                  <h2 className="text-lg font-semibold mb-4">Related Posts</h2>
                  <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                    {relatedPosts.map((related) => (
                      <Link
                        key={related.slug}
                        href={`/blog/${related.slug}`}
                        className="group block p-4 rounded-lg border border-border/50 hover:border-orange-500/30 transition-colors"
                      >
                        <h3 className="text-sm font-medium group-hover:text-orange-400 transition-colors line-clamp-2">
                          {related.title}
                        </h3>
                        <p className="text-xs text-muted-foreground mt-1">
                          {related.readingTime} min read
                        </p>
                      </Link>
                    ))}
                  </div>
                </div>
              )}
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
