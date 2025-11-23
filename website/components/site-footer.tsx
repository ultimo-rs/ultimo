import { Disc, Github, Twitter } from "lucide-react";
import Image from "next/image";
import Link from "next/link";
import { siteConfig } from "@/lib/config";

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
                <Image src="/logo.svg" alt="Ultimo" fill />
              </div>
              <span>Ultimo</span>
            </Link>
            <p className="text-muted-foreground text-sm max-w-xs mb-6">
              A modern, high-performance web framework for Rust. Type-safe,
              fast, and developer friendly.
            </p>
            <div className="flex gap-4">
              <Link
                href={siteConfig.links.github}
                target="_blank"
                className="text-muted-foreground hover:text-foreground transition-colors"
              >
                <Github className="h-5 w-5" />
              </Link>
              <Link
                href={siteConfig.links.twitter}
                target="_blank"
                className="text-muted-foreground hover:text-foreground transition-colors"
              >
                <Twitter className="h-5 w-5" />
              </Link>
              <Link
                href={siteConfig.links.discord}
                target="_blank"
                className="text-muted-foreground hover:text-foreground transition-colors"
              >
                <Disc className="h-5 w-5" />
              </Link>
            </div>
          </div>

          <div>
            <h4 className="font-semibold text-foreground mb-4">Product</h4>
            <ul className="space-y-2 text-sm text-muted-foreground">
              <li>
                <Link
                  href={siteConfig.nav.features}
                  className="hover:text-orange-500 transition-colors"
                >
                  Features
                </Link>
              </li>
              <li>
                <Link
                  href="/#performance"
                  className="hover:text-orange-500 transition-colors"
                >
                  Performance
                </Link>
              </li>
              <li>
                <Link
                  href={siteConfig.nav.roadmap}
                  className="hover:text-orange-500 transition-colors"
                >
                  Roadmap
                </Link>
              </li>
              <li>
                <Link
                  href={siteConfig.nav.changelog}
                  className="hover:text-orange-500 transition-colors"
                >
                  Changelog
                </Link>
              </li>
            </ul>
          </div>

          <div>
            <h4 className="font-semibold text-foreground mb-4">Resources</h4>
            <ul className="space-y-2 text-sm text-muted-foreground">
              <li>
                <Link
                  href={siteConfig.nav.documentation}
                  className="hover:text-orange-500 transition-colors"
                >
                  Documentation
                </Link>
              </li>
              <li>
                <Link
                  href={siteConfig.nav.apiReference}
                  className="hover:text-orange-500 transition-colors"
                >
                  API Reference
                </Link>
              </li>
              <li>
                <Link
                  href={siteConfig.nav.examples}
                  className="hover:text-orange-500 transition-colors"
                >
                  Examples
                </Link>
              </li>
              <li>
                <Link
                  href={siteConfig.nav.blog}
                  className="hover:text-orange-500 transition-colors"
                >
                  Blog
                </Link>
              </li>
            </ul>
          </div>

          <div>
            <h4 className="font-semibold text-foreground mb-4">Community</h4>
            <ul className="space-y-2 text-sm text-muted-foreground">
              <li>
                <Link
                  href={siteConfig.links.github}
                  target="_blank"
                  className="hover:text-orange-500 transition-colors"
                >
                  GitHub
                </Link>
              </li>
              <li>
                <Link
                  href={siteConfig.links.discord}
                  target="_blank"
                  className="hover:text-orange-500 transition-colors"
                >
                  Discord
                </Link>
              </li>
              <li>
                <Link
                  href={siteConfig.links.twitter}
                  target="_blank"
                  className="hover:text-orange-500 transition-colors"
                >
                  Twitter
                </Link>
              </li>
              <li>
                <Link
                  href="#"
                  className="hover:text-orange-500 transition-colors"
                >
                  Contributing
                </Link>
              </li>
            </ul>
          </div>
        </div>

        <div className="border-t border-border pt-8 flex flex-col md:flex-row justify-between items-center gap-4">
          <p className="text-muted-foreground text-sm">
            © {new Date().getFullYear()} Ultimo Framework. MIT License.
          </p>
          <div className="flex gap-6 text-sm text-muted-foreground">
            <Link href="#" className="hover:text-foreground">
              Privacy Policy
            </Link>
            <Link href="#" className="hover:text-foreground">
              Terms of Service
            </Link>
          </div>
        </div>
      </div>
    </footer>
  );
}
