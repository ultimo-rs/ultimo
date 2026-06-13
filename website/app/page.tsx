import { BlogSection } from "@/components/blog-section";
import { ComparisonSection } from "@/components/comparison-section";
import { CTASection } from "@/components/cta-section";
import { FeaturesSection } from "@/components/features-section";
import { HeroSection } from "@/components/hero-section";
import { SecuritySection } from "@/components/security-section";
import { StatsSection } from "@/components/stats-section";
import { TypeSafeSection } from "@/components/type-safe-section";
import { UseCasesSection } from "@/components/use-cases-section";

export default function Home() {
  return (
    <div className="min-h-screen selection:bg-orange-500/30">
      <main>
        <HeroSection />
        <UseCasesSection />
        <FeaturesSection />
        <StatsSection />
        <SecuritySection />
        <TypeSafeSection />
        <ComparisonSection />
        <BlogSection />
        <CTASection />
      </main>
    </div>
  );
}
