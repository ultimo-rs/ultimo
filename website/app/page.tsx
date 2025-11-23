import { ComparisonSection } from "@/components/comparison-section";
import { CTASection } from "@/components/cta-section";
import { FeaturesSection } from "@/components/features-section";
import { HeroSection } from "@/components/hero-section";
import { StatsSection } from "@/components/stats-section";
import { TypeSafeSection } from "@/components/type-safe-section";

export default function Home() {
  return (
    <div className="min-h-screen selection:bg-orange-500/30">
      <main>
        <HeroSection />
        <FeaturesSection />
        <StatsSection />
        <TypeSafeSection />
        <ComparisonSection />
        <CTASection />
      </main>
    </div>
  );
}
