import Link from "next/link";
import { Button } from "@/components/ui/button";
import { siteConfig } from "@/lib/config";

export function CTASection() {
  return (
    <section className="py-24 relative overflow-hidden">
      <div className="absolute inset-0 -z-10">
        <div className="absolute inset-0 bg-gradient-to-t from-orange-500/5 to-transparent" />
        <div className="absolute top-[30%] left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-orange-500/15 blur-[120px] rounded-full" />
      </div>

      <div className="container px-4 md:px-6 mx-auto text-center">
        <h2 className="text-3xl md:text-4xl font-bold mb-6 text-foreground">
          Ready to build your next high-performance backend?
        </h2>
        <p className="text-muted-foreground text-lg mb-10 max-w-2xl mx-auto">
          Join the developers building faster, safer, and more reliable
          applications with Ultimo.
        </p>

        <div className="flex flex-col sm:flex-row gap-4 justify-center">
          <Button
            size="lg"
            className="h-12 px-8 text-base bg-white text-black hover:bg-zinc-200"
            asChild
          >
            <Link href={siteConfig.nav.getStarted}>Get Started</Link>
          </Button>
          <Button
            size="lg"
            variant="outline"
            className="h-12 px-8 text-base border-zinc-800 hover:bg-zinc-900 text-white bg-transparent"
            asChild
          >
            <Link href={siteConfig.nav.documentation}>Read Documentation</Link>
          </Button>
        </div>
      </div>
    </section>
  );
}
