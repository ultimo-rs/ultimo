import { KeyRound, Lock, ShieldCheck } from "lucide-react";

const capabilities = [
  "100% safe Rust — #![forbid(unsafe_code)], zero unsafe in the framework",
  "Secure-by-default sessions & cookies (HttpOnly/Secure/SameSite, 256-bit ids)",
  "JWT auth (HS256, algorithm pinned, exp validated)",
  "API-key auth with a pluggable store (SHA-256 hashed, constant-time)",
  "Authorization guards — scope checks across JWT & API-key identities",
  "CSRF protection (double-submit cookie, constant-time compare)",
  "Security-headers middleware (HSTS, X-Frame-Options, Referrer-Policy, …)",
  "Request body-size limits (DoS guard)",
  "Supply-chain CI — cargo-audit + cargo-deny on every change",
];

export function SecuritySection() {
  return (
    // biome-ignore lint/correctness/useUniqueElementIds: anchor target for nav
    <section
      id="security"
      className="py-24 border-b border-border bg-background relative overflow-hidden"
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
                  <li
                    key={cap}
                    className="flex items-start gap-2 text-sm text-foreground"
                  >
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
