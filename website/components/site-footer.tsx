import Image from "next/image";
import Link from "next/link";
import { siteConfig } from "@/lib/config";

type FooterLink = { label: string; href: string; external?: boolean };

const columns: { title: string; links: FooterLink[] }[] = [
  {
    title: "Product",
    links: [
      { label: "Features", href: siteConfig.nav.features },
      { label: "Performance", href: siteConfig.nav.performance },
      { label: "Roadmap", href: siteConfig.nav.roadmap },
      { label: "Changelog", href: siteConfig.nav.changelog },
    ],
  },
  {
    title: "Resources",
    links: [
      { label: "Documentation", href: siteConfig.nav.documentation },
      { label: "API Reference", href: siteConfig.nav.apiReference },
      { label: "Examples", href: siteConfig.nav.examples, external: true },
      { label: "Blog", href: siteConfig.nav.blog },
    ],
  },
  {
    title: "Community",
    links: [
      { label: "GitHub", href: siteConfig.links.github, external: true },
      {
        label: "Discussions",
        href: siteConfig.links.discussions,
        external: true,
      },
      {
        label: "Contributing",
        href: siteConfig.links.contributing,
        external: true,
      },
      { label: "crates.io", href: siteConfig.links.crates, external: true },
    ],
  },
];

export function SiteFooter() {
  return (
    <footer className="border-t border-border bg-background pt-16 pb-8">
      <div className="container px-4 md:px-6 mx-auto">
        <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-8 mb-12">
          <div className="col-span-2 lg:col-span-2">
            <Link
              href="/"
              className="flex items-center gap-2 font-bold text-xl mb-4"
            >
              <div className="relative h-6 w-6">
                <Image src="/logo.svg" alt="" fill />
              </div>
              <span>Ultimo</span>
            </Link>
            <p className="text-muted-foreground text-sm max-w-xs">
              A Rust web framework for REST, JSON-RPC and WebSockets, with typed
              TypeScript clients generated from your Rust code.
            </p>
          </div>

          {columns.map((column) => (
            <div key={column.title}>
              <h3 className="font-semibold text-foreground mb-4">
                {column.title}
              </h3>
              <ul className="space-y-2 text-sm text-muted-foreground">
                {column.links.map((link) => (
                  <li key={link.label}>
                    <Link
                      href={link.href}
                      {...(link.external
                        ? { target: "_blank", rel: "noopener noreferrer" }
                        : {})}
                      className="hover:text-orange-500 transition-colors"
                    >
                      {link.label}
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>

        <div className="border-t border-border pt-8 text-sm text-muted-foreground">
          <p>
            © {new Date().getFullYear()} Ultimo ·{" "}
            <Link
              href="https://github.com/ultimo-rs/ultimo/blob/main/LICENSE"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-foreground transition-colors"
            >
              MIT License
            </Link>
          </p>
        </div>
      </div>
    </footer>
  );
}
