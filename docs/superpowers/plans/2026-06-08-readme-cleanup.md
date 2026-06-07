# README Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or superpowers:executing-plans. Single-file editorial rewrite — the "tests" are grep assertions + a compile-check of the quick-start snippet.

**Goal:** Replace the 1082-line `README.md` with a concise, accurate front-door README (~200–250 lines): keep the user-facing top, delete the embedded build-spec, fix every API/version inaccuracy.

**Architecture:** One file rewrite of `README.md` to the 8-section structure in the spec. Verify by grepping for stale/fictional strings and by compiling the quick-start example against the real 0.4.0 API.

**Tech Stack:** Markdown; the real `ultimo` 0.4.0 public API.

---

## Task 1: Rewrite README.md

**Files:** Modify `README.md` (full rewrite, preserving the existing logo + badge block at the top).

- [ ] **Step 1: Preserve the header.** Keep the existing `<div align="center">` logo + badge row + nav links block (lines ~1–28) verbatim — those are correct.

- [ ] **Step 2: Replace everything from the intro down** with the 8 sections from the spec (`docs/superpowers/specs/2026-06-08-readme-cleanup-design.md`): intro → Why Ultimo → Quick start → Type-safe clients → Feature flags → CLI → Docs/Examples/Contributing/License. **Delete** Core Philosophy, Tech Stack Requirements, standalone MSRV section, Project Structure, and the entire "Feature Requirements" build-spec (### 1–11) and everything after it that is build-spec/duplicate.

- [ ] **Step 3: The quick-start code block MUST be this (compiles against 0.4.0 — note `ctx.req.param`, explicit parse error mapping):**

````markdown
```rust
use ultimo::prelude::*;

#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
}

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    let mut app = Ultimo::new();

    app.get("/users/:id", |ctx: Context| async move {
        let id: u32 = ctx
            .req
            .param("id")?
            .parse()
            .map_err(|_| UltimoError::BadRequest("invalid id".into()))?;
        ctx.json(User { id, name: format!("User {id}") }).await
    });

    println!("→ http://127.0.0.1:3000");
    app.listen("127.0.0.1:3000").await
}
```
````

Install snippet uses `ultimo = "0.4"`; mention **MSRV 1.86** in one line near Quick start.

- [ ] **Step 4: Feature-flags section — the real opt-in list:**
  `websocket`, `session`, `jwt`, `api-key`, `csrf`, `testing`, `test-helpers`,
  `sqlx-postgres` / `sqlx-mysql` / `sqlx-sqlite`, `diesel-postgres` / `diesel-mysql` / `diesel-sqlite` (all `default = []`).

- [ ] **Step 5: CLI section — real subcommands only:**
  `ultimo new <name> --template <...>`, `ultimo generate --path <dir> --output <dir> [--watch]`, `ultimo build --profile <debug|release>`. Note `ultimo dev` is experimental (hot-reload is on the roadmap). No fabricated flags.

- [ ] **Step 6: "Why Ultimo" bullets — true claims only:** automatic TypeScript client generation; REST + JSON-RPC in one app; WebSocket + pub/sub; auth (JWT, API-key, scope guards) + security hardening (safe Rust, sessions, CSRF, security headers); O(1) routing; SQLx/Diesel integration. **No** throughput numbers (link to docs `/performance`). **No** file uploads (not implemented).

- [ ] **Step 7: Commit.**
```bash
git add README.md
git commit -m "docs(readme): trim to a concise, accurate front door (closes #94)"
```

---

## Task 2: Verify accuracy

**Files:** none (assertions)

- [ ] **Step 1: No stale/fictional strings remain.** Run:
```bash
grep -nE "1\.75|version = \"0\.3\"|ultimo = \"0\.3\"|ctx\.param\(|bearer_auth|rate_limit|upload|c\.res|158|152|0\.6ms|15x|req/sec" README.md || echo "CLEAN"
```
Expected: `CLEAN` (or only legitimate prose, e.g. the word "upload" must NOT appear as a claimed feature).

- [ ] **Step 2: Compile-check the quick-start snippet** against the real API by dropping it into a scratch binary in the workspace and checking it builds:
```bash
mkdir -p /tmp/ultimo-readme-check/src
cat > /tmp/ultimo-readme-check/Cargo.toml <<'EOF'
[package]
name = "readme-check"
version = "0.0.0"
edition = "2021"
[dependencies]
ultimo = { path = "REPLACE_WITH_ABS_PATH/ultimo" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
EOF
# copy the quick-start main.rs into src/main.rs, then:
cargo check --manifest-path /tmp/ultimo-readme-check/Cargo.toml
```
Expected: compiles. (Alternatively, paste the snippet into `examples/basic` temporarily and `cargo check -p basic`, then revert.) If it doesn't compile, fix the README snippet and re-check.

- [ ] **Step 3: Line count sanity.** `wc -l README.md` → roughly 150–260 lines (down from 1082).

---

## Task 3: Ship

- [ ] Push the branch, open the PR (`docs(readme): concise, accurate front door`), watch CI, `gh pr merge --squash --admin --delete-branch`, sync `main`, and confirm **#94 closes**.

---

## Self-review

**Spec coverage:** intro/why/quick-start/type-safe/feature-flags/CLI/links → Task 1 Steps 2–6 ✓. Claim audit (no uploads, MSRV 1.86, version 0.4, `ctx.req.param`, delete build-spec) → Steps 2–3,6 + Task 2 grep ✓. Success criteria (compiles, no fabricated numbers/fictional APIs, ~200 lines) → Task 2 ✓.

**Placeholder scan:** the only intentional placeholder is `REPLACE_WITH_ABS_PATH` in the throwaway scratch check (Task 2 Step 2) — substitute `/Users/ruslanelishaev/Desktop/projects/ultimo`. README content itself has no placeholders.

**Consistency:** the quick-start uses `ctx.req.param("id")?` (real) + `.map_err(...)` for the parse (since `UltimoError` has no `From<ParseIntError>`), `ctx.json(...).await`, `Ultimo::new()`, `ultimo::Result` — all real 0.4.0 API surfaced by the prelude.
