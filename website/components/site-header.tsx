import Link from "next/link"
import Image from "next/image"
import { Github, Menu } from "lucide-react"
import { Button } from "@/components/ui/button"

export function SiteHeader() {
  return (
    <header className="fixed top-0 z-50 w-full border-b border-border/40 bg-background/80 backdrop-blur-xl supports-[backdrop-filter]:bg-background/60">
      <div className="container mx-auto flex h-16 items-center justify-between px-4 sm:px-8">
        <div className="flex items-center gap-2">
          <Link href="/" className="flex items-center gap-2 font-bold text-xl">
            <div className="relative h-8 w-8 overflow-hidden">
              <Image src="/logo.svg" alt="Ultimo Logo" fill className="object-contain" />
            </div>
            <span>Ultimo</span>
          </Link>
          <nav className="hidden md:flex items-center gap-6 ml-10 text-sm font-medium text-muted-foreground">
            <Link href="#features" className="hover:text-foreground transition-colors">
              Features
            </Link>
            <Link href="#performance" className="hover:text-foreground transition-colors">
              Performance
            </Link>
            <Link href="https://docs.ultimo.dev" className="hover:text-foreground transition-colors">
              Docs
            </Link>
            <Link href="/blog" className="hover:text-foreground transition-colors">
              Blog
            </Link>
          </nav>
        </div>

        <div className="flex items-center gap-4">
          <Link
            href="https://github.com/ultimo/ultimo"
            target="_blank"
            className="hidden sm:block text-muted-foreground hover:text-foreground transition-colors"
          >
            <Github className="h-5 w-5" />
            <span className="sr-only">GitHub</span>
          </Link>
          <div className="hidden sm:flex gap-3">
            <Button variant="ghost" size="sm" className="text-muted-foreground hover:text-foreground">
              Sign In
            </Button>
            <Button size="sm">Get Started</Button>
          </div>
          <Button variant="ghost" size="icon" className="md:hidden text-muted-foreground">
            <Menu className="h-6 w-6" />
          </Button>
        </div>
      </div>
    </header>
  )
}
