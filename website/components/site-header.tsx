"use client";

import { Menu, X } from "lucide-react";
import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { GitHubStarsButton } from "@/components/github-stars";
import { Button } from "@/components/ui/button";
import { siteConfig } from "@/lib/config";
import { cn } from "@/lib/utils";

const navLinks = [
  { label: "Features", href: siteConfig.nav.features },
  { label: "Performance", href: siteConfig.nav.performance },
  { label: "Docs", href: siteConfig.nav.documentation },
  { label: "Blog", href: siteConfig.nav.blog },
];

export function SiteHeader({ stars }: { stars: number | null }) {
  const [scrolled, setScrolled] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const pathname = usePathname();

  useEffect(() => {
    const handleScroll = () => setScrolled(window.scrollY > 50);
    handleScroll();
    window.addEventListener("scroll", handleScroll, { passive: true });
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  // Close the menu on navigation and on Escape.
  // biome-ignore lint/correctness/useExhaustiveDependencies: close when the route changes
  useEffect(() => {
    setMenuOpen(false);
  }, [pathname]);
  useEffect(() => {
    if (!menuOpen) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setMenuOpen(false);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [menuOpen]);

  const solid = scrolled || menuOpen;

  return (
    <header
      className={cn(
        "fixed top-0 z-50 w-full transition-colors duration-300",
        menuOpen
          ? "border-b border-border/40 bg-background"
          : solid
            ? "border-b border-border/40 bg-background/90 backdrop-blur-xl supports-[backdrop-filter]:bg-background/70"
            : "border-b border-transparent bg-transparent",
      )}
    >
      <div className="container mx-auto flex h-16 items-center justify-between px-4 sm:px-8">
        <div className="flex items-center gap-2">
          <Link href="/" className="flex items-center gap-2 font-bold text-xl">
            <div className="relative h-8 w-8 overflow-hidden">
              <Image
                src="/logo.svg"
                alt=""
                fill
                className="object-contain"
              />
            </div>
            <span>Ultimo</span>
          </Link>
          <nav
            aria-label="Main"
            className="hidden md:flex items-center gap-6 ml-10 text-sm font-medium text-muted-foreground"
          >
            {navLinks.map((link) => (
              <Link
                key={link.label}
                href={link.href}
                className="hover:text-foreground transition-colors"
              >
                {link.label}
              </Link>
            ))}
          </nav>
        </div>

        <div className="flex items-center gap-3">
          <GitHubStarsButton initial={stars} className="hidden sm:inline-flex" />
          <Button size="sm" className="hidden md:inline-flex" asChild>
            <Link href={siteConfig.nav.getStarted}>Get Started</Link>
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="md:hidden text-muted-foreground"
            aria-label={menuOpen ? "Close menu" : "Open menu"}
            aria-expanded={menuOpen}
            aria-controls="mobile-menu"
            onClick={() => setMenuOpen((open) => !open)}
          >
            {menuOpen ? <X className="h-6 w-6" /> : <Menu className="h-6 w-6" />}
          </Button>
        </div>
      </div>

      {menuOpen && (
        <nav
          id="mobile-menu"
          aria-label="Main"
          className="md:hidden border-t border-border/40 px-4 pb-6 pt-2"
        >
          <ul className="flex flex-col">
            {navLinks.map((link) => (
              <li key={link.label}>
                <Link
                  href={link.href}
                  onClick={() => setMenuOpen(false)}
                  className="block py-3 text-base font-medium text-muted-foreground hover:text-foreground transition-colors"
                >
                  {link.label}
                </Link>
              </li>
            ))}
          </ul>
          <div className="mt-4 flex flex-col gap-3">
            <Button asChild>
              <Link href={siteConfig.nav.getStarted}>Get Started</Link>
            </Button>
            <GitHubStarsButton
              initial={stars}
              className="h-10 justify-center sm:hidden"
            />
          </div>
        </nav>
      )}
    </header>
  );
}
