"use client";

import { Github, Menu } from "lucide-react";
import Image from "next/image";
import Link from "next/link";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { siteConfig } from "@/lib/config";
import { cn } from "@/lib/utils";

export function SiteHeader() {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const handleScroll = () => {
      setScrolled(window.scrollY > 50);
    };

    window.addEventListener("scroll", handleScroll);
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  return (
    <header
      className={cn(
        "fixed top-0 z-50 w-full transition-all duration-300",
        scrolled
          ? "border-b border-border/40 bg-background/80 backdrop-blur-xl supports-[backdrop-filter]:bg-background/60"
          : "border-b border-transparent bg-transparent"
      )}
    >
      <div className="container mx-auto flex h-16 items-center justify-between px-4 sm:px-8">
        <div className="flex items-center gap-2">
          <Link href="/" className="flex items-center gap-2 font-bold text-xl">
            <div className="relative h-8 w-8 overflow-hidden">
              <Image
                src="/logo.svg"
                alt="Ultimo Logo"
                fill
                className="object-contain"
              />
            </div>
            <span className="">Ultimo</span>
          </Link>
          <nav className="hidden md:flex items-center gap-6 ml-10 text-sm font-medium text-muted-foreground">
            <Link
              href={siteConfig.nav.features}
              className="hover:text-foreground transition-colors"
            >
              Features
            </Link>
            <Link
              href="#performance"
              className="hover:text-foreground transition-colors"
            >
              Performance
            </Link>
            <Link
              href={siteConfig.nav.documentation}
              className="hover:text-foreground transition-colors"
            >
              Docs
            </Link>
            <Link
              href={siteConfig.nav.blog}
              className="hover:text-foreground transition-colors"
            >
              Blog
            </Link>
          </nav>
        </div>

        <div className="flex items-center gap-4">
          <Link
            href={siteConfig.links.github}
            target="_blank"
            className="hidden sm:block text-muted-foreground hover:text-foreground transition-colors"
          >
            <Github className="h-5 w-5" />
            <span className="sr-only">GitHub</span>
          </Link>
          <div className="hidden sm:flex gap-3">
            <Button size="sm" asChild>
              <Link href={siteConfig.nav.getStarted}>Get Started</Link>
            </Button>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="md:hidden text-muted-foreground"
          >
            <Menu className="h-6 w-6" />
          </Button>
        </div>
      </div>
    </header>
  );
}
