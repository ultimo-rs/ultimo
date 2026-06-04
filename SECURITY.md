# Security Policy

Security is one of Ultimo's two core pillars (alongside performance). We take it
seriously and welcome responsible disclosure.

## Supported versions

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | ✅ Yes             |
| < 0.3   | ❌ No (please upgrade) |

While pre-1.0, security fixes land on the latest minor release.

## Reporting a vulnerability

**Please do not open a public issue for security vulnerabilities.**

Report privately via **GitHub Security Advisories**:
[Report a vulnerability](https://github.com/ultimo-rs/ultimo/security/advisories/new)

Include where possible:
- A description of the issue and its impact
- Steps to reproduce / a proof of concept
- Affected version(s)
- Any suggested remediation

### What to expect
- **Acknowledgement** within 3 business days.
- An initial assessment and severity within 7 days.
- We'll keep you updated on remediation progress and coordinate a disclosure
  timeline with you. Credit is given to reporters unless you prefer anonymity.

## Our security posture

Ultimo is designed to be **secure by default**:

- **100% safe Rust** — the framework enforces `#![forbid(unsafe_code)]`; there is
  no `unsafe` anywhere in the library.
- **Memory safety** from Rust itself (no buffer overflows, use-after-free, data
  races in safe code).
- **Secure-by-default sessions/cookies** — `HttpOnly` + `Secure` + `SameSite`,
  256-bit random ids, anti session-fixation, server-side storage.
- **Supply-chain checks in CI** — `cargo audit` (RUSTSEC advisories) on every PR;
  a committed `Cargo.lock` for reproducible builds; `cargo-semver-checks` guards
  the public API.
- **Minimal dependencies** — a small, auditable surface.

### Defense in depth
For production, deploy Ultimo behind a managed WAF/CDN (e.g. Cloudflare, AWS WAF)
for full WAF rules, DDoS mitigation, and geo controls. Ultimo provides the
in-app building blocks (validation, rate limiting, security headers, request
guards) — the edge handles volumetric and signature-based threats.

## Disclosure

We follow coordinated disclosure: we'll work with you on a fix and a public
advisory, and won't disclose details until a fixed release is available.
