import fs from "fs";
import matter from "gray-matter";
import path from "path";

const postsDirectory = path.join(process.cwd(), "content/posts");

function getReadingTime(content: string): number {
  const words = content.replace(/```[\s\S]*?```/g, "").split(/\s+/).length;
  return Math.max(1, Math.round(words / 200));
}

export interface PostMeta {
  slug: string;
  title: string;
  description: string;
  date: string;
  author: string;
  tags: string[];
  image?: string;
  readingTime: number;
}

export function getAllPosts(): PostMeta[] {
  if (!fs.existsSync(postsDirectory)) {
    return [];
  }

  const files = fs
    .readdirSync(postsDirectory)
    .filter((f) => f.endsWith(".mdx"));

  const posts = files.map((filename) => {
    const slug = filename.replace(/\.mdx$/, "");
    const filePath = path.join(postsDirectory, filename);
    const fileContents = fs.readFileSync(filePath, "utf8");
    const { data, content } = matter(fileContents);

    return {
      slug,
      title: data.title ?? slug,
      description: data.description ?? "",
      date: data.date ?? "",
      author: data.author ?? "Ultimo Team",
      tags: data.tags ?? [],
      image: data.image,
      readingTime: getReadingTime(content),
    } satisfies PostMeta;
  });

  return posts.sort(
    (a, b) => new Date(b.date).getTime() - new Date(a.date).getTime(),
  );
}

export function getPostBySlug(slug: string) {
  const filePath = path.join(postsDirectory, `${slug}.mdx`);

  if (!fs.existsSync(filePath)) {
    return null;
  }

  const fileContents = fs.readFileSync(filePath, "utf8");
  const { data, content } = matter(fileContents);

  return {
    meta: {
      slug,
      title: data.title ?? slug,
      description: data.description ?? "",
      date: data.date ?? "",
      author: data.author ?? "Ultimo Team",
      tags: data.tags ?? [],
      image: data.image,
      readingTime: getReadingTime(content),
    } satisfies PostMeta,
    content,
  };
}

export function getRelatedPosts(slug: string, limit = 3): PostMeta[] {
  const posts = getAllPosts();
  const current = posts.find((p) => p.slug === slug);
  if (!current) return posts.filter((p) => p.slug !== slug).slice(0, limit);

  return posts
    .filter((p) => p.slug !== slug)
    .map((p) => ({
      post: p,
      score: p.tags.filter((t) => current.tags.includes(t)).length,
    }))
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
    .map((r) => r.post);
}
