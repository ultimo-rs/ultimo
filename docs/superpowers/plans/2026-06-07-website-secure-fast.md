# Honest "Secure & Fast" Homepage — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax. This is content/copy work on an existing Next.js + Tailwind + shadcn site — preserve the existing visual language; no redesign.

**Goal:** Replace the marketing site's fabricated performance numbers with substantiated claims and add the missing Security pillar, so the homepage advertises "Secure & Fast" honestly.

**Architecture:** Re-content existing components in `website/components/*` in place; add one new `security-section.tsx` mirroring the perf section; wire it into `app/page.tsx`. No new dependencies, no layout system changes.

**Tech Stack:** Next.js (static export), Tailwind, shadcn/ui, lucide-react icons.

**Verification (no unit tests for marketing copy):**
- `cd website && npm run build` succeeds (static export).
- `grep -rn "158\|152\|0.6ms\|15x\|req/sec\|req/s" website/app website/components` returns **nothing** (no fabricated figures remain).

---

## Task 1: Honest performance section

**Files:** Modify `website/components/stats-section.tsx`

Replace the three fabricated stat blocks (158k req/sec, 0.6ms Latency, 15x Faster) and the "Requests Per Second" competitor bar chart with honest, substantiated content + a relative O(1)-routing visual.

- [ ] **Step 1:** Replace the entire `StatsSection` body. Keep the `<section id="performance">` wrapper, background blobs, and two-column grid. Left column = honest claims; right column = the routing-scaling card.

Left column intro paragraph (replace the existing `<p>`):
```tsx
<p className="text-muted-foreground text-lg mb-8 leading-relaxed">
  Ultimo is a thin layer over Hyper and Tokio — the same core that powers the
  fastest Rust servers — so you get native speed with a higher-level developer
  experience. We measure what the framework itself costs and guard it in CI.
</p>
```

Replace the three stat blocks with these three (reuse the existing icon/markup pattern — `Zap`, `Layers`, `ShieldCheck` from lucide-react; update the import to `import { GitCompareArrows, Layers, Zap } from "lucide-react";`):
```tsx
<div className="flex items-start gap-4">
  <div className="p-2 rounded-lg bg-orange-500/10 text-orange-500 border border-orange-500/20">
    <Zap className="h-5 w-5" />
  </div>
  <div>
    <h4 className="text-lg font-semibold text-foreground">O(1) routing</h4>
    <p className="text-muted-foreground text-sm">
      Constant-time route lookup — 10 routes or 10,000, the same cost.
    </p>
  </div>
</div>

<div className="flex items-start gap-4">
  <div className="p-2 rounded-lg bg-orange-500/10 text-orange-500 border border-orange-500/20">
    <Layers className="h-5 w-5" />
  </div>
  <div>
    <h4 className="text-lg font-semibold text-foreground">Built on Hyper + Tokio</h4>
    <p className="text-muted-foreground text-sm">
      The proven async HTTP core of the Rust ecosystem — no re-implementation.
    </p>
  </div>
</div>

<div className="flex items-start gap-4">
  <div className="p-2 rounded-lg bg-orange-500/10 text-orange-500 border border-orange-500/20">
    <GitCompareArrows className="h-5 w-5" />
  </div>
  <div>
    <h4 className="text-lg font-semibold text-foreground">Regression-guarded</h4>
    <p className="text-muted-foreground text-sm">
      Every framework change is benchmarked in CI — we don't get slower by accident.
    </p>
  </div>
</div>
```

Replace the right-column "Requests Per Second" card with a relative O(1)-routing visual (no absolute numbers, no competitors). Three bars for 10/100/500 routes, all equal height, conveying constant-time:
```tsx
<div className="relative">
  <div className="absolute -inset-4 bg-orange-500/20 blur-3xl rounded-full opacity-20" />
  <div className="relative rounded-xl border border-border bg-card p-6 shadow-xl">
    <h3 className="text-sm font-medium text-muted-foreground mb-2 uppercase tracking-wider">
      Route lookup time
    </h3>
    <p className="text-xs text-muted-foreground mb-6">
      In-process micro-benchmark — constant as the routing table grows.
    </p>
    <div className="space-y-4">
      {[
        { label: "10 routes" },
        { label: "100 routes" },
        { label: "500 routes" },
      ].map((row) => (
        <div key={row.label} className="space-y-2">
          <div className="flex justify-between text-sm">
            <span className="font-medium text-muted-foreground">{row.label}</span>
            <span className="text-orange-500 font-bold">O(1)</span>
          </div>
          <div className="h-2 w-full bg-secondary rounded-full overflow-hidden">
            <div className="h-full bg-gradient-to-r from-orange-500 to-red-500 w-[34%]" />
          </div>
        </div>
      ))}
    </div>
    <a
      href="https://docs.ultimo.dev/performance"
      className="mt-6 inline-block text-sm font-medium text-orange-500 hover:underline"
    >
      Reproduce it yourself → docs/performance
    </a>
  </div>
</div>
```

- [ ] **Step 2:** Build check. Run `cd website && npm run build`. Expected: success.
- [ ] **Step 3:** Commit.
```bash
git add website/components/stats-section.tsx
git commit -m "feat(website): honest performance section (O(1) routing, no fabricated req/s)"
```

---

## Task 2: Security pillar section

**Files:** Create `website/components/security-section.tsx`; Modify `website/app/page.tsx`

- [ ] **Step 1:** Create `website/components/security-section.tsx` mirroring the perf section's structure (section wrapper + blobs + two-column grid). Left = headline + intro; right = a capability grid. Use real capabilities only.
```tsx
import { KeyRound, Lock, ShieldCheck } from "lucide-react";

const capabilities = [
  "100% safe Rust — #![forbid(unsafe_code)], zero unsafe in the framework",
  "Secure-by-default sessions & cookies (HttpOnly/Secure/SameSite, 256-bit ids)",
  "JWT auth (HS256, alg pinned, exp validated)",
  "API-key auth with a pluggable store (SHA-256 hashed, constant-time)",
  "Authorization guards — scope checks across JWT & API-key identities",
  "CSRF protection (double-submit cookie, constant-time compare)",
  "Security-headers middleware (HSTS, X-Frame-Options, Referrer-Policy, …)",
  "Request body-size limits (DoS guard)",
  "Supply-chain CI — cargo-audit + cargo-deny on every change",
];

export function SecuritySection() {
  return (
    <section
      id="security"
      className="py-24 border-y border-border bg-background relative overflow-hidden"
    >
      <div className="absolute top-0 left-0 w-full h-full overflow-hidden -z-10 pointer-events-none">
        <div className="absolute top-[20%] left-[5%] w-[700px] h-[700px] bg-orange-500/8 blur-[130px] rounded-full" />
        <div className="absolute bottom-[20%] right-[10%] w-[500px] h-[500px] bg-red-500/8 blur-[100px] rounded-full" />
      </div>

      <div className="container px-4 md:px-6 mx-auto">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
          <div>
            <div className="inline-flex items-center gap-2 mb-6 px-3 py-1 rounded-full bg-orange-500/10 text-orange-500 border border-orange-500/20 text-sm font-medium">
              <Lock className="h-4 w-4" /> Secure by default
            </div>
            <h2 className="text-3xl md:text-4xl font-bold mb-6 tracking-tight">
              Security is a <br />
              <span className="text-gradient">first-class pillar</span>
            </h2>
            <p className="text-muted-foreground text-lg mb-8 leading-relaxed">
              Built in safe Rust with secure defaults across the stack —
              authentication, authorization, CSRF, hardened headers, and
              supply-chain checks ship in the box, not as an afterthought.
            </p>
            <a
              href="https://docs.ultimo.dev/security"
              className="inline-flex items-center gap-2 text-sm font-medium text-orange-500 hover:underline"
            >
              <ShieldCheck className="h-4 w-4" /> Read the security guide →
            </a>
          </div>

          <div className="relative">
            <div className="absolute -inset-4 bg-orange-500/20 blur-3xl rounded-full opacity-20" />
            <div className="relative rounded-xl border border-border bg-card p-6 shadow-xl">
              <h3 className="text-sm font-medium text-muted-foreground mb-6 uppercase tracking-wider flex items-center gap-2">
                <KeyRound className="h-4 w-4" /> What's built in
              </h3>
              <ul className="space-y-3">
                {capabilities.map((cap) => (
                  <li key={cap} className="flex items-start gap-2 text-sm text-foreground">
                    <ShieldCheck className="h-4 w-4 text-orange-500 mt-0.5 shrink-0" />
                    <span>{cap}</span>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
```

- [ ] **Step 2:** Wire it into `website/app/page.tsx` — import and render `<SecuritySection />` immediately after `<StatsSection />` so the two pillars sit together:
```tsx
import { SecuritySection } from "@/components/security-section";
// ...
<StatsSection />
<SecuritySection />
<TypeSafeSection />
```

- [ ] **Step 3:** Build check. Run `cd website && npm run build`. Expected: success.
- [ ] **Step 4:** Commit.
```bash
git add website/components/security-section.tsx website/app/page.tsx
git commit -m "feat(website): add Security pillar section"
```

---

## Task 3: Fix feature cards (Blazing Fast + add Secure by Default + drop hot-reload claim)

**Files:** Modify `website/components/features-section.tsx`

- [ ] **Step 1:** In the `core` category `features` array, replace the "Blazing Fast" feature object's `details` (remove "158,000+ requests per second" and "Sub-millisecond average latency"):
```tsx
{
  title: "Blazing Fast",
  description:
    "Native Rust speed on the Hyper + Tokio core, with a higher-level developer experience.",
  icon: Zap,
  details: [
    "O(1) constant-time routing",
    "Built on Hyper + Tokio",
    "Zero-cost abstractions, no GC",
    "Regression-guarded in CI",
  ],
},
```

- [ ] **Step 2:** Add a new "Secure by Default" feature to the `core` category `features` array (after "WebSocket Support"). Uses the already-imported `ShieldCheck` icon:
```tsx
{
  title: "Secure by Default",
  description:
    "Authentication, authorization, and hardening ship in the box — in 100% safe Rust.",
  icon: ShieldCheck,
  details: [
    "JWT + API-key auth, scope guards",
    "Sessions, CSRF, security headers",
    "#![forbid(unsafe_code)]",
    "Supply-chain CI (audit + deny)",
  ],
},
```

- [ ] **Step 3:** In the `developer-tools` category, "Developer First" feature, replace the false `"Hot reload in dev"` detail with `"Project scaffolding (ultimo new)"`.

- [ ] **Step 4:** Build check. Run `cd website && npm run build`. Expected: success.
- [ ] **Step 5:** Commit.
```bash
git add website/components/features-section.tsx
git commit -m "feat(website): honest feature cards + Secure by Default card"
```

---

## Task 4: Fix the comparison table

**Files:** Modify `website/components/comparison-section.tsx`

- [ ] **Step 1:** Remove the `performance` entry from every framework's `features` object (all 5 frameworks) and from `featureLabels`. The table no longer claims a throughput edge.

- [ ] **Step 2:** Change Ultimo's `rateLimit` and `devServer` from `true` to `"planned"` (rate limiting and hot-reload dev server are not shipped). Leave the other frameworks' values as-is.

- [ ] **Step 3:** Add a `"planned"` branch to `FeatureCell` (before the final perf fallback, which is now dead — also remove the perf fallback `return` block since the `performance` key is gone):
```tsx
if (value === "planned") {
  return (
    <div className="flex items-center justify-center gap-2">
      <Minus className="h-5 w-5 text-muted-foreground" />
      <span className="text-sm text-muted-foreground font-medium">Planned</span>
    </div>
  );
}
```
Delete the trailing `// Performance values` block that returns `{value} req/s` (no longer reachable).

- [ ] **Step 4:** Update the section intro `<p>` to reframe from speed to DX:
```tsx
<p className="text-muted-foreground text-lg">
  Axum, Actix, and friends are excellent, minimal HTTP layers. Ultimo builds on
  the same Rust core and adds the full-stack pieces — typed clients, RPC, auth —
  so you ship features instead of wiring boilerplate.
</p>
```

- [ ] **Step 5:** Update the legend caption to include the Planned marker:
```tsx
<p className="text-sm text-muted-foreground">
  <Check className="inline h-4 w-4 text-green-500 mr-1" />
  Built-in &nbsp;&nbsp;
  <Minus className="inline h-4 w-4 text-yellow-500 mr-1" />
  Requires manual setup/crates &nbsp;&nbsp;
  <Minus className="inline h-4 w-4 text-muted-foreground mr-1" />
  Planned &nbsp;&nbsp;
  <X className="inline h-4 w-4 text-red-500/50 mr-1" />
  Not available
</p>
```

- [ ] **Step 6:** Build check. Run `cd website && npm run build`. Expected: success.
- [ ] **Step 7:** Commit.
```bash
git add website/components/comparison-section.tsx
git commit -m "feat(website): accurate comparison table (drop perf row, mark Planned)"
```

---

## Task 5: Hero subhead + verifiable badges

**Files:** Modify `website/components/hero-section.tsx`

- [ ] **Step 1:** Read the file. Update the subhead paragraph to name the pillars, e.g.:
```tsx
Secure by default, fast on the Hyper + Tokio core, and type-safe end to end —
with automatic TypeScript clients generated from your Rust API.
```

- [ ] **Step 2:** Add a verifiable badge strip below the hero CTAs (shields.io static/dynamic badges — all true). Place it after the existing button row:
```tsx
<div className="mt-8 flex flex-wrap items-center gap-2 justify-center">
  <img alt="crates.io" src="https://img.shields.io/crates/v/ultimo?color=orange" />
  <img alt="CI" src="https://img.shields.io/github/actions/workflow/status/ultimo-rs/ultimo/ci.yml?branch=main&label=CI" />
  <img alt="license" src="https://img.shields.io/crates/l/ultimo" />
  <img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.86-blue" />
  <img alt="safe Rust" src="https://img.shields.io/badge/unsafe-forbidden-success" />
</div>
```
Match the hero's existing centering/spacing classes; adjust wrapper if the hero is left-aligned.

- [ ] **Step 3:** Build check. Run `cd website && npm run build`. Expected: success.
- [ ] **Step 4:** Commit.
```bash
git add website/components/hero-section.tsx
git commit -m "feat(website): pillar-focused hero subhead + verifiable badges"
```

---

## Task 5b: Remove fabricated number from the features page

**Files:** Modify `website/app/features/page.tsx`

- [ ] **Step 1:** Read the file; remove/replace the `"152k+ req/sec throughput"` list item with a true claim, e.g. `"O(1) constant-time routing"`.
- [ ] **Step 2:** Build check + commit.
```bash
git add website/app/features/page.tsx
git commit -m "feat(website): drop fabricated throughput from features page"
```

---

## Task 6: Final verification + ship

**Files:** none

- [ ] **Step 1:** Assert no fabricated figures remain anywhere:
```bash
grep -rn "158\|152\|145\|0.6ms\|15x\|req/sec\|req/s\|153,1\|132,000" website/app website/components || echo "CLEAN"
```
Expected: `CLEAN`.
- [ ] **Step 2:** Full build: `cd website && npm run build`. Expected: success.
- [ ] **Step 3:** Push the branch, open the PR (conventional title `feat(website): honest "Secure & Fast" homepage`), watch CI (incl. the Vercel preview), confirm the preview renders, then `gh pr merge --squash --admin --delete-branch` and sync `main`.

---

## Self-review

**Spec coverage:**
- Remove fabricated stats + req/s chart → Task 1 ✓
- Add Security pillar section → Task 2 ✓
- Honest "Blazing Fast" card + Secure card + fix hot-reload → Task 3 ✓
- Comparison: drop perf row, Planned for rate-limit/dev-server, reframe → Task 4 ✓
- Hero subhead + badges → Task 5 ✓
- features/page.tsx fabricated number → Task 5b ✓
- Type-Safe section unchanged (not in plan — correct, keep as-is) ✓
- Success criteria (no fabricated numbers, build passes) → Task 6 ✓

**Placeholder scan:** No TBD/TODO; copy is final. Hero (Task 5) and features/page (5b) require reading the file first because their exact surrounding markup isn't captured here — the *new content* is fully specified, only the insertion anchor is read at execution.

**Consistency:** New `SecuritySection` export name matches the `app/page.tsx` import in Task 2. `"planned"` value (Task 4 Step 2) matches the new `FeatureCell` branch (Step 3). lucide-react icons referenced (`GitCompareArrows`, `Layers`, `Zap`, `ShieldCheck`, `Lock`, `KeyRound`, `Minus`, `Check`, `X`) are all valid lucide names.
