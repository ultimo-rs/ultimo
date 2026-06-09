# TypeScript Client Codegen — Design

**Date:** 2026-06-09
**Status:** Approved (brainstorm), pending spec review
**Codex review item:** #4 (strengthen the generated TypeScript story)

## Problem

Ultimo's headline promise is "automatic TypeScript client generation — no
hand-written types, no drift." Today that promise is only half true.

The RPC registry auto-generates the client *methods* (call signatures, endpoint
mapping, query-vs-mutation), but the **types are hand-written strings** supplied
by the developer at registration:

```rust
rest_rpc.query(
    "getUserById",
    handler,
    "{ id: number }".to_string(),   // TS input — hand-written
    "User".to_string(),             // TS output — hand-written, and `User`
                                    // is never defined anywhere
);
```

Consequences:

- The generated client references TS interfaces (e.g. `User`) that **do not
  exist** unless the developer also hand-writes them.
- The Rust struct's shape is never translated to TypeScript — so types drift the
  moment a Rust field changes. This is the exact opposite of the promise.
- `register(name, handler)` (the no-types convenience) defaults both types to
  `any`, silently discarding type safety.

Separately, the CLI plumbing is dishonest (`ultimo generate` builds the project
and hunts for an artifact the user's own `main` must have written; `--watch` is a
stub). That was partly addressed by the CLI-honesty pass (PR #105), but the real
fix is to make generation derive real types and actually run.

## Goal

Make "automatic TypeScript clients" true: TypeScript types are **derived from
the Rust types**, eliminating the hand-written strings, and `ultimo generate`
actually produces a complete, type-correct client.

Non-goals (this effort): multi-language clients (Python/Go/…) — deferred to
0.8.0; a bundler/dev-server integration; OpenAPI-driven generation.

## Decisions

| Axis | Decision | Rationale |
|---|---|---|
| Ambition | Real type derivation, **phased** | "Honest plumbing only" leaves the core gap (types not derived). Phasing keeps PRs reviewable. |
| Type mechanism | **Adopt `ts-rs`** (feature-gated) | Mature, pure-Rust derive crate. Handles serde attrs, generics, `Option`/`Vec`, enums→unions. Reinventing a serde-compatible Rust→TS deriver is a large, bug-prone maintenance sink. Accepts one optional external dep against the "minimal deps" ethos. |
| Method/wiring collection | **Keep the runtime registry** | It already produces correct client methods; only the *type* half is broken. |
| Generation trigger | **Convention binary** (`src/bin/generate-client.rs`) run by `ultimo generate` | No source parsing, no artifact-hunting. The CLI executes the real registry. Explicit, debuggable, testable. |
| Packaging | New **`client-gen`** Cargo feature (off by default) | Consistent with everything being opt-in. `ts-rs` is optional behind it. |
| Back-compat | Breaking API change (pre-1.0 → minor bump); keep a `*_with_types` string escape hatch | Foreign/exotic types that can't derive `TS` still work. |

## Architecture

### Type derivation (the heart)

`ts-rs`'s `TS` derive is re-exported as `ultimo::rpc::TS` so users don't depend
on `ts-rs` directly:

```rust
use ultimo::rpc::TS; // re-export of ts_rs::TS

#[derive(Serialize, Deserialize, TS)]
struct GetUserInput { id: u32 }

#[derive(Serialize, Deserialize, TS)]
struct User { id: u32, name: String }
```

The registration methods drop their `String` type arguments and gain `TS`
bounds:

```rust
// before: query(name, handler, ts_in: String, ts_out: String)
// after:
pub fn query<F, Fut, I, O>(&self, name: impl Into<String>, handler: F)
where
    I: for<'de> Deserialize<'de> + TS + 'static,
    O: Serialize + TS + 'static,
    /* ... */
```

At registration the registry uses `ts-rs` to capture, for each procedure:

- the **input/output type names** (`I::name()`, `O::name()`) for the method
  signature, and
- the **full interface declarations** for those types and their transitive
  dependencies (via `ts-rs` dependency walking), collected and de-duplicated.

`generate_typescript_client()` / `generate_client_file()` then emit:

1. the collected `interface`/`type` declarations (so referenced types are
   defined), then
2. the client class with fully-typed methods.

Escape hatch for types that cannot derive `TS` (foreign crates, dynamic shapes):

```rust
pub fn query_with_types<F, Fut, I, O>(
    &self, name: impl Into<String>, handler: F,
    ts_input: String, ts_output: String,
) { /* current string-based behavior */ }
```

### Generation trigger

- `ultimo new` scaffolds `src/bin/generate-client.rs`: builds the registry
  exactly as `main` does (shared `build_registry()` fn in the template) and calls
  `rpc.generate_client_file("<output>")`.
- `ultimo generate --project <dir> --output <path>` runs
  `cargo run --bin generate-client` in `<dir>` and writes/relocates the result to
  `<path>`.
- If no `generate-client` bin exists, `ultimo generate` fails with an actionable
  message pointing at the convention (and how to add it).

This replaces the current "build then hunt for `ultimo-client.ts`" logic.

### Packaging

- New feature: `client-gen = ["dep:ts-rs"]` in `ultimo/Cargo.toml`, off by
  default.
- `ultimo::rpc::TS` re-export gated on `client-gen`.

Because `ts-rs` is optional, the `TS` trait bound cannot exist when `client-gen`
is off. Type capture must happen at registration time (the registry type-erases
the handler immediately after, so the generic `I`/`O` are gone afterward) — so
the bound has to live on the registration method. We resolve this cleanly by
splitting the surface, not by cfg-duplicating one method name:

| Method | Bound | Availability |
|---|---|---|
| `query` / `mutation` | `I: TS, O: TS` (types derived) | `#[cfg(feature = "client-gen")]` only |
| `query_with_types` / `mutation_with_types` | none (explicit TS strings) | always |
| `register` | none (types default to `any`) | always |

So with `client-gen` **on**, the ergonomic derived methods (`query`/`mutation`)
are the happy path; the `*_with_types` escape hatch covers foreign/exotic types.
With `client-gen` **off**, codegen-less users use `*_with_types` or `register`
exactly as before — they are unaffected. (Final method names / whether to also
offer a derived `register` are settled in the Phase 1 plan; the principle is
fixed here.)

## Phasing (separate PRs)

### Phase 1 — Library type derivation
- Add `client-gen` feature + `ts-rs` optional dep; re-export `TS`.
- Change `register/query/mutation` to derive types via `TS`; add
  `*_with_types` escape hatches.
- Registry collects derived interface declarations + transitive deps, de-duped.
- `generate_typescript_client`/`generate_client_file` emit interfaces + client.
- Update the `rpc-modes` example to the derived API.
- **Tests:** golden-file test — a fixture crate with known structs asserts the
  exact generated `.ts` (interfaces + client), covering `Option`, `Vec`, nested
  structs, enums, and serde `rename`.

### Phase 2 — CLI integration
- `ultimo generate` runs the convention bin (`cargo run --bin generate-client`)
  and writes `--output`; remove the artifact-hunting code.
- `ultimo new` scaffolds `src/bin/generate-client.rs` + a shared
  `build_registry()`.
- **Tests:** `assert_cmd` integration tests for `ultimo new`, `ultimo generate`,
  and `--help` output (also closes Codex #10).

### Phase 3 — Docs & polish
- Rewrite `docs-site/docs/pages/typescript.mdx` and `cli.mdx` against the real
  derived flow; update `README` quick-start.
- Optional `ultimo generate --watch` (re-run on source change).

## Error handling

- `generate` without a `generate-client` bin → clear error + remedy.
- A registered type missing `#[derive(TS)]` → compile error (the bound enforces
  it); documented with the `*_with_types` escape hatch as the alternative.
- `ts-rs` covers `Option`→`T | null`, `Vec`→`T[]`, enums→unions, and serde
  `rename`/`tag` attributes; these are asserted by the golden-file test rather
  than assumed.

## Testing strategy

- **Golden-file** (Phase 1): fixture types → exact expected `.ts`. The single
  highest-value test — it proves the promise end to end and guards drift.
- **CLI integration** (Phase 2): `assert_cmd` over `new`/`generate`/`--help`.
- **Doctests**: the `query`/`mutation` examples in rustdoc compile under
  `client-gen`.

## Open questions

None blocking. Phase 1 is independently shippable and is a satisfying first
release (the library promise becomes true even before the CLI polish).
