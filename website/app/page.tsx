import { SiteHeader } from "@/components/site-header"
import { HeroSection } from "@/components/hero-section"
import { FeaturesSection } from "@/components/features-section"
import { StatsSection } from "@/components/stats-section"
import { TypeSafeSection } from "@/components/type-safe-section"
import { CTASection } from "@/components/cta-section"
import { SiteFooter } from "@/components/site-footer"

export default function Home() {
  return (
    <div className="min-h-screen bg-black text-white selection:bg-orange-500/30">
      <SiteHeader />
      <main>
        <HeroSection />
        <FeaturesSection />
        <StatsSection />
        <TypeSafeSection />
        <CTASection />
      </main>
      <SiteFooter />
    </div>
  )
}
