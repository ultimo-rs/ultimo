import { getAllPosts } from "@/lib/blog";

export const dynamic = "force-static";

export function GET() {
  const posts = getAllPosts();
  const siteUrl = "https://ultimo.dev";

  const items = posts
    .map(
      (post) => `    <item>
      <title><![CDATA[${post.title}]]></title>
      <link>${siteUrl}/blog/${post.slug}</link>
      <guid isPermaLink="true">${siteUrl}/blog/${post.slug}</guid>
      <description><![CDATA[${post.description}]]></description>
      <pubDate>${new Date(post.date).toUTCString()}</pubDate>
      <author>hello@ultimo.dev (${post.author})</author>
      ${post.tags.map((tag) => `<category>${tag}</category>`).join("\n      ")}
    </item>`,
    )
    .join("\n");

  const feed = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>Ultimo Blog</title>
    <link>${siteUrl}/blog</link>
    <description>Technical articles about building modern web applications with Rust and Ultimo</description>
    <language>en-us</language>
    <lastBuildDate>${new Date(posts[0]?.date ?? Date.now()).toUTCString()}</lastBuildDate>
    <atom:link href="${siteUrl}/blog/feed.xml" rel="self" type="application/rss+xml"/>
${items}
  </channel>
</rss>`;

  return new Response(feed, {
    headers: {
      "Content-Type": "application/xml",
      "Cache-Control": "public, max-age=3600, s-maxage=3600",
    },
  });
}
