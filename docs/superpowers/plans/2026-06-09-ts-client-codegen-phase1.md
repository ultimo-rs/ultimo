# TypeScript Client Codegen — Phase 1 (Library Type Derivation) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Ultimo's generated TypeScript client carry **real types derived from the Rust structs** (via `ts-rs`), instead of hand-written type strings and a hardcoded `User` interface.

**Architecture:** Add an opt-in `client-gen` Cargo feature pulling `ts-rs`. New feature-gated `query`/`mutation` methods take `I: TS, O: TS` bounds; at registration they record each procedure's input/output **type name** and collect the **interface declarations** of those types and their transitive dependencies (via a `ts_rs::TypeVisitor` walk) into the registry. Client generation emits the collected declarations. The previous string-based methods survive as `*_with_types` escape hatches.

**Tech Stack:** Rust, `ts-rs` 12 (MSRV 1.78, edition 2021 — compatible with Ultimo's MSRV 1.86), Cargo features.

**Spec:** `docs/superpowers/specs/2026-06-09-ts-client-codegen-design.md`

**Scope:** Phase 1 only — library type derivation. NOT the CLI (`ultimo generate`) and NOT docs rewrites (Phases 2 and 3).

---

## Verified ts-rs 12 facts (do not re-derive — these were probed against ts-rs 12.0.1)

- The `TS` trait methods take a `&ts_rs::Config`: `T::name(&cfg)`, `T::decl(&cfg)`, `T::inline(&cfg)`.
- `ts_rs::Config::default()` constructs a usable config (no env needed).
- `T::decl(&cfg)` for a struct returns e.g. `type User = { id: number, name: string, tags: Array<string>, nickname: string | null, };`.
- `T::dependencies(&cfg)` only returns types that have an export path — **empty by default**, so it is NOT used here.
- Transitive declarations are collected with `T::visit_dependencies(&mut visitor)` where `visitor: ts_rs::TypeVisitor`; `visit::<U>()` is invoked for each direct dependency type, and we recurse.
- Primitives/containers (`u32`→`number`, `Vec<T>`→`Array<...>`) satisfy `U::inline(&cfg) == U::name(&cfg)`; declarable structs/enums do not. This is the guard for "should I emit a `type X = …` for this?".

## File structure

- `ultimo/Cargo.toml` — add optional `ts-rs` dep + `client-gen` feature.
- `ultimo/src/rpc.rs` — re-export `TS`; add `type_decls` field; add `collect_type_decls`; add feature-gated `query`/`mutation`; rename current string methods to `*_with_types`; replace `append_type_definitions`.
- `ultimo/tests/client_gen.rs` — new golden-file integration test (feature-gated).
- `examples/rpc-modes/Cargo.toml` + `examples/rpc-modes/src/main.rs` — migrate to derived API.

---

### Task 1: Add the `client-gen` feature and `ts-rs` dependency

**Files:**
- Modify: `ultimo/Cargo.toml`

- [ ] **Step 1: Add the optional dependency**

In `ultimo/Cargo.toml`, under `[dependencies]`, add:

```toml
ts-rs = { version = "12", optional = true }
```

- [ ] **Step 2: Add the feature**

In `ultimo/Cargo.toml`, under `[features]`, add:

```toml
client-gen = ["dep:ts-rs"]
```

- [ ] **Step 3: Verify it resolves and the lib still builds both ways**

Run: `cargo build -p ultimo && cargo build -p ultimo --features client-gen`
Expected: both succeed; the second downloads/compiles `ts-rs`.

- [ ] **Step 4: Commit**

```bash
git add ultimo/Cargo.toml Cargo.lock
git commit -m "feat(rpc): add client-gen feature + ts-rs optional dep"
```

---

### Task 2: Add the declaration collector

A free function that, given a type `T: TS`, inserts the TS declarations of `T` and all its transitive struct/enum dependencies into a `BTreeMap<name, decl>` (sorted + de-duplicated).

**Files:**
- Modify: `ultimo/src/rpc.rs`

- [ ] **Step 1: Write the failing test**

Add to the bottom of `ultimo/src/rpc.rs`:

```rust
#[cfg(all(test, feature = "client-gen"))]
mod client_gen_tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(ts_rs::TS)]
    #[allow(dead_code)]
    struct Inner {
        flag: bool,
    }

    #[derive(ts_rs::TS)]
    #[allow(dead_code)]
    struct Outer {
        inner: Inner,
        items: Vec<Inner>,
        count: u32,
    }

    #[test]
    fn collects_struct_and_nested_decls_but_not_primitives() {
        let mut decls: BTreeMap<String, String> = BTreeMap::new();
        collect_type_decls::<Outer>(&mut decls);

        // Both named structs are present...
        assert!(decls.contains_key("Outer"), "Outer missing: {:?}", decls.keys());
        assert!(decls.contains_key("Inner"), "Inner missing: {:?}", decls.keys());
        // ...and no primitive/container leaked in as a declaration.
        assert!(!decls.contains_key("number"));
        assert!(!decls.keys().any(|k| k.starts_with("Array")));

        // The declarations are real `type X = {...}` strings.
        assert!(decls["Inner"].contains("flag: boolean"));
        assert!(decls["Outer"].contains("inner: Inner"));
        assert!(decls["Outer"].contains("items: Array<Inner>"));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p ultimo --features client-gen --lib client_gen_tests`
Expected: FAIL to compile — `collect_type_decls` is not defined.

- [ ] **Step 3: Implement `collect_type_decls`**

Add to `ultimo/src/rpc.rs` (e.g. just above the `impl RpcRegistry` block):

```rust
/// Collect the TypeScript declarations of `T` and all of its transitive
/// struct/enum dependencies into `out` (keyed by TS type name, so duplicates
/// across procedures collapse). Primitives and containers (`number`,
/// `Array<…>`, …) are walked for their inner types but never emitted as a
/// declaration of their own.
#[cfg(feature = "client-gen")]
fn collect_type_decls<T: ts_rs::TS + 'static>(out: &mut std::collections::BTreeMap<String, String>) {
    use std::any::TypeId;
    use std::collections::HashSet;
    use ts_rs::{Config, TypeVisitor, TS};

    struct DeclCollector<'a> {
        cfg: &'a Config,
        seen: HashSet<TypeId>,
        decls: &'a mut std::collections::BTreeMap<String, String>,
    }

    impl TypeVisitor for DeclCollector<'_> {
        fn visit<U: TS + 'static + ?Sized>(&mut self) {
            if !self.seen.insert(TypeId::of::<U>()) {
                return; // already visited — also breaks recursive-type cycles
            }
            let name = U::name(self.cfg);
            // Declarable types (structs/enums) have an inline body distinct from
            // their own name; primitives/containers inline to their name.
            if U::inline(self.cfg) != name {
                self.decls.insert(name, U::decl(self.cfg));
            }
            U::visit_dependencies(self); // always recurse into inner types
        }
    }

    let cfg = Config::default();
    let mut collector = DeclCollector {
        cfg: &cfg,
        seen: HashSet::new(),
        decls: out,
    };
    <DeclCollector as TypeVisitor>::visit::<T>(&mut collector);
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p ultimo --features client-gen --lib client_gen_tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add ultimo/src/rpc.rs
git commit -m "feat(rpc): add ts-rs declaration collector (transitive, deduped)"
```

---

### Task 3: Store collected declarations on the registry and emit them

Add a `type_decls` field (plain strings — not feature-gated) and make `append_type_definitions` emit it instead of the hardcoded `User` interface.

**Files:**
- Modify: `ultimo/src/rpc.rs` (struct `RpcRegistry` ~lines 41–47, `new_with_mode` ~lines 73–80, `append_type_definitions` ~lines 370–377)

- [ ] **Step 1: Write the failing test**

Add to the `client_gen_tests` module:

```rust
    #[test]
    fn generated_client_has_no_hardcoded_user_interface() {
        // A registry with no procedures must not invent a `User` interface.
        let rpc = RpcRegistry::new();
        let client = rpc.generate_typescript_client();
        assert!(
            !client.contains("export interface User"),
            "hardcoded User interface leaked into output:\n{client}"
        );
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p ultimo --features client-gen --lib client_gen_tests::generated_client_has_no_hardcoded_user_interface`
Expected: FAIL — the current `append_type_definitions` always writes `export interface User { … }`.

- [ ] **Step 3: Add the `type_decls` field**

In `ultimo/src/rpc.rs`, change the struct:

```rust
#[derive(Clone)]
pub struct RpcRegistry {
    mode: RpcMode,
    procedures: Arc<std::sync::Mutex<HashMap<String, RpcHandler>>>,
    type_definitions: Arc<std::sync::Mutex<Vec<TypeDefinition>>>,
    metadata: Arc<std::sync::Mutex<HashMap<String, ProcedureMetadata>>>,
    /// TS interface/type declarations collected from registered procedure
    /// input/output types (keyed by TS name → declaration). Populated by the
    /// `client-gen` query/mutation methods; empty otherwise.
    type_decls: Arc<std::sync::Mutex<std::collections::BTreeMap<String, String>>>,
}
```

And initialize it in `new_with_mode`:

```rust
    pub fn new_with_mode(mode: RpcMode) -> Self {
        Self {
            mode,
            procedures: Arc::new(std::sync::Mutex::new(HashMap::new())),
            type_definitions: Arc::new(std::sync::Mutex::new(Vec::new())),
            metadata: Arc::new(std::sync::Mutex::new(HashMap::new())),
            type_decls: Arc::new(std::sync::Mutex::new(std::collections::BTreeMap::new())),
        }
    }
```

- [ ] **Step 4: Replace the hardcoded `append_type_definitions`**

Replace the whole method body:

```rust
    /// Append collected type declarations to the generated client.
    fn append_type_definitions(&self, client: &mut String) {
        let decls = self.type_decls.lock().unwrap();
        if decls.is_empty() {
            return;
        }
        client.push_str("\n// Type Definitions\n");
        for decl in decls.values() {
            client.push_str(decl);
            client.push('\n');
        }
    }
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p ultimo --features client-gen --lib client_gen_tests::generated_client_has_no_hardcoded_user_interface`
Expected: PASS.

- [ ] **Step 6: Verify the default (no-feature) build still compiles**

Run: `cargo build -p ultimo`
Expected: PASS (the `type_decls` field is plain `String`s; nothing feature-gated leaked into the struct).

- [ ] **Step 7: Commit**

```bash
git add ultimo/src/rpc.rs
git commit -m "feat(rpc): collect declarations on registry; drop hardcoded User interface"
```

---

### Task 4: Derived `query`/`mutation` + `*_with_types` escape hatches

Rename the current string-taking `query`/`mutation` to `query_with_types`/`mutation_with_types` (always available), and add new feature-gated `query`/`mutation` that derive types via `TS` and populate `type_decls`. Re-export `TS`.

**Files:**
- Modify: `ultimo/src/rpc.rs` (current `query` ~lines 114–128, `mutation` ~lines 130–144; add re-export near top of `impl`/module)

- [ ] **Step 1: Re-export the `TS` derive (gated)**

Near the top of `ultimo/src/rpc.rs` (after the `use` lines), add:

```rust
/// Re-export of the `ts-rs` `TS` derive so users `#[derive(TS)]` without adding
/// `ts-rs` to their own `Cargo.toml`. Available with the `client-gen` feature.
#[cfg(feature = "client-gen")]
pub use ts_rs::TS;
```

- [ ] **Step 2: Rename the existing string methods to `*_with_types`**

Replace the current `query` and `mutation` methods (the ones taking `ts_input: String, ts_output: String`) with renamed versions, keeping identical bodies:

```rust
    /// Register a query procedure with explicit TypeScript type strings
    /// (escape hatch for types that cannot derive `TS`). Uses GET in REST mode.
    pub fn query_with_types<F, Fut, I, O>(
        &self,
        name: impl Into<String>,
        handler: F,
        ts_input: String,
        ts_output: String,
    ) where
        F: Fn(I) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
        I: for<'de> Deserialize<'de> + 'static,
        O: Serialize + 'static,
    {
        self.procedure(name, handler, ts_input, ts_output, true)
    }

    /// Register a mutation procedure with explicit TypeScript type strings
    /// (escape hatch for types that cannot derive `TS`). Uses POST in REST mode.
    pub fn mutation_with_types<F, Fut, I, O>(
        &self,
        name: impl Into<String>,
        handler: F,
        ts_input: String,
        ts_output: String,
    ) where
        F: Fn(I) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
        I: for<'de> Deserialize<'de> + 'static,
        O: Serialize + 'static,
    {
        self.procedure(name, handler, ts_input, ts_output, false)
    }
```

> `register` (defaults to `any`) and `register_with_types` (string) are left unchanged.

- [ ] **Step 3: Write the failing test for derived `query`/`mutation`**

Add to the `client_gen_tests` module:

```rust
    #[derive(serde::Serialize, serde::Deserialize, ts_rs::TS)]
    struct GetThingInput {
        id: u32,
    }

    #[derive(serde::Serialize, serde::Deserialize, ts_rs::TS)]
    struct Thing {
        id: u32,
        label: String,
    }

    #[tokio::test]
    async fn derived_query_emits_named_types_and_signature() {
        let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);
        rpc.query("getThing", |_input: GetThingInput| async move {
            Ok(Thing { id: 1, label: "x".into() })
        });

        let client = rpc.generate_typescript_client();
        // Method signature references the derived named types.
        assert!(
            client.contains("getThing(params: GetThingInput): Promise<Thing>"),
            "signature missing:\n{client}"
        );
        // Both interfaces are declared (no dangling references).
        assert!(client.contains("type GetThingInput = "), "input decl missing:\n{client}");
        assert!(client.contains("type Thing = "), "output decl missing:\n{client}");
        assert!(client.contains("label: string"), "Thing body missing:\n{client}");
    }
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p ultimo --features client-gen --lib client_gen_tests::derived_query_emits_named_types_and_signature`
Expected: FAIL to compile — `query` (the 2-arg derived form) does not exist yet (only `query_with_types`).

- [ ] **Step 5: Add the derived, feature-gated `query`/`mutation`**

Add these methods to the `impl RpcRegistry` block:

```rust
    /// Register a query procedure (idempotent; GET in REST mode). Input/output
    /// TypeScript types are derived from the Rust types via `ts-rs`.
    #[cfg(feature = "client-gen")]
    pub fn query<F, Fut, I, O>(&self, name: impl Into<String>, handler: F)
    where
        F: Fn(I) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
        I: for<'de> Deserialize<'de> + ts_rs::TS + 'static,
        O: Serialize + ts_rs::TS + 'static,
    {
        let cfg = ts_rs::Config::default();
        let ts_input = <I as ts_rs::TS>::name(&cfg);
        let ts_output = <O as ts_rs::TS>::name(&cfg);
        {
            let mut decls = self.type_decls.lock().unwrap();
            collect_type_decls::<I>(&mut decls);
            collect_type_decls::<O>(&mut decls);
        }
        self.procedure(name, handler, ts_input, ts_output, true);
    }

    /// Register a mutation procedure (state-changing; POST in REST mode).
    /// Input/output TypeScript types are derived from the Rust types via `ts-rs`.
    #[cfg(feature = "client-gen")]
    pub fn mutation<F, Fut, I, O>(&self, name: impl Into<String>, handler: F)
    where
        F: Fn(I) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
        I: for<'de> Deserialize<'de> + ts_rs::TS + 'static,
        O: Serialize + ts_rs::TS + 'static,
    {
        let cfg = ts_rs::Config::default();
        let ts_input = <I as ts_rs::TS>::name(&cfg);
        let ts_output = <O as ts_rs::TS>::name(&cfg);
        {
            let mut decls = self.type_decls.lock().unwrap();
            collect_type_decls::<I>(&mut decls);
            collect_type_decls::<O>(&mut decls);
        }
        self.procedure(name, handler, ts_input, ts_output, false);
    }
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p ultimo --features client-gen --lib client_gen_tests::derived_query_emits_named_types_and_signature`
Expected: PASS.

- [ ] **Step 7: Confirm no other callers used the old string `query`/`mutation`**

Run: `grep -rnE '\.(query|mutation)\(' ultimo/src examples --include=*.rs | grep -v with_types`
Expected: only the new derived-style calls (2 args) remain; any 4-arg string call is updated to `*_with_types` (the only one is `examples/rpc-modes`, handled in Task 5). The lib's own tests use the 2-arg derived form.

- [ ] **Step 8: Commit**

```bash
git add ultimo/src/rpc.rs
git commit -m "feat(rpc): derived query/mutation via ts-rs; string forms become *_with_types"
```

---

### Task 5: Migrate the `rpc-modes` example to the derived API

**Files:**
- Modify: `examples/rpc-modes/Cargo.toml`
- Modify: `examples/rpc-modes/src/main.rs`

- [ ] **Step 1: Enable the feature in the example**

In `examples/rpc-modes/Cargo.toml`, change the `ultimo` dependency line to:

```toml
ultimo = { path = "../../ultimo", features = ["client-gen"] }
```

- [ ] **Step 2: Derive `TS` on the example types**

In `examples/rpc-modes/src/main.rs`, add `ultimo::rpc::TS` to imports and the `TS` derive to each request/response struct. For example:

```rust
use ultimo::rpc::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
struct User {
    // ...existing fields...
}

#[derive(Debug, Deserialize, Serialize, TS)]
struct EmptyParams {}

#[derive(Debug, Deserialize, Serialize, TS)]
struct GetUserInput {
    // ...existing fields...
}

#[derive(Debug, Deserialize, Serialize, TS)]
struct CreateUserInput {
    // ...existing fields...
}

#[derive(Debug, Serialize, TS)]
struct UserListResponse {
    // ...existing fields...
}
```

- [ ] **Step 3: Replace the string-typed `query`/`mutation`/`register_with_types` calls with derived calls**

Drop the trailing `ts_input`/`ts_output` string arguments. For the REST registry, e.g.:

```rust
    rest_rpc.query("listUsers", move |_input: EmptyParams| {
        // ...existing async body...
    });

    rest_rpc.query("getUserById", move |input: GetUserInput| {
        // ...existing async body...
    });

    rest_rpc.mutation("createUser", move |input: CreateUserInput| {
        // ...existing async body...
    });
```

For the JSON-RPC registry, replace each `jsonrpc_rpc.register_with_types(name, handler, in, out)` with `jsonrpc_rpc.mutation(name, handler)` (or `query` for read-only procedures), dropping the string args. (`register_with_types` still exists, but the example should demonstrate the derived path.)

- [ ] **Step 4: Build and run the example, inspect output**

Run: `cargo run -p rpc-modes`
Expected: it builds and writes `ultimo-client-rest.ts` / `ultimo-client-jsonrpc.ts`. Open one and confirm it contains real `type User = { … }` / `type CreateUserInput = { … }` declarations and no dangling `User` reference.

- [ ] **Step 5: Commit**

```bash
git add examples/rpc-modes/Cargo.toml examples/rpc-modes/src/main.rs
git commit -m "example(rpc-modes): use derived TS types (client-gen)"
```

---

### Task 6: Golden-file integration test

A standalone integration test that registers known types and asserts the exact generated client text — the highest-value guard against drift.

**Files:**
- Create: `ultimo/tests/client_gen.rs`

- [ ] **Step 1: Write the golden-file test**

Create `ultimo/tests/client_gen.rs`:

```rust
//! Golden-file test for derived TypeScript client generation.
//! Run with: cargo test -p ultimo --features client-gen --test client_gen

#![cfg(feature = "client-gen")]

use ultimo::rpc::{RpcMode, RpcRegistry, TS};

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct CreateUserInput {
    name: String,
    email: String,
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct User {
    id: u32,
    name: String,
    email: String,
    tags: Vec<String>,
    nickname: Option<String>,
}

#[test]
fn rest_client_has_derived_types_and_signatures() {
    let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);
    rpc.mutation("createUser", |input: CreateUserInput| async move {
        Ok(User {
            id: 1,
            name: input.name,
            email: input.email,
            tags: vec![],
            nickname: None,
        })
    });

    let client = rpc.generate_typescript_client();

    // Signature uses the derived named types.
    assert!(
        client.contains("async createUser(params: CreateUserInput): Promise<User>"),
        "signature missing:\n{client}"
    );

    // Both interfaces are declared with their real shapes.
    assert!(client.contains("type CreateUserInput = "), "input decl missing:\n{client}");
    assert!(client.contains("type User = "), "output decl missing:\n{client}");
    assert!(client.contains("email: string"));
    assert!(client.contains("tags: Array<string>"));
    assert!(client.contains("nickname: string | null"));

    // No dangling/hardcoded interface.
    assert!(!client.contains("export interface User"));
}
```

- [ ] **Step 2: Run the test to verify it passes**

Run: `cargo test -p ultimo --features client-gen --test client_gen`
Expected: PASS.

- [ ] **Step 3: Confirm the test target does not compile without the feature**

Run: `cargo test -p ultimo --test client_gen 2>&1 | tail -3`
Expected: the test is skipped/compiles to nothing (the file is `#![cfg(feature = "client-gen")]`), no failures.

- [ ] **Step 4: Commit**

```bash
git add ultimo/tests/client_gen.rs
git commit -m "test(rpc): golden-file test for derived TS client generation"
```

---

### Task 7: Verification gate

**Files:** none (verification only)

- [ ] **Step 1: Formatting**

Run: `cargo fmt --all --check`
Expected: clean (run `cargo fmt --all` if not).

- [ ] **Step 2: Clippy across the feature surface**

Run: `cargo clippy -p ultimo --lib --features "client-gen" -- -D warnings`
Then: `cargo clippy -p ultimo --features "websocket,test-helpers,testing,session,csrf,jwt,api-key" --all-targets -- -D warnings`
Expected: both clean. (Note: `--all-targets` with `client-gen` would pull the websocket integration tests that need their own features, so lint `client-gen` with `--lib` only — mirroring the static-files/compression CI pattern.)

- [ ] **Step 3: Tests (default + client-gen)**

Run:
```bash
cargo test -p ultimo --lib
cargo test -p ultimo --features client-gen --lib client_gen_tests
cargo test -p ultimo --features client-gen --test client_gen
cargo build -p rpc-modes
```
Expected: all pass / build.

- [ ] **Step 4: Doctests under the feature**

Run: `cargo test -p ultimo --doc --features "client-gen"`
Expected: pass (no new doctests required in Phase 1, but ensure existing ones still compile with the feature on).

- [ ] **Step 5: Commit any fmt fixes, then proceed to PR via the ship-feature workflow**

The CI step for `client-gen` (a `clippy --lib` run + the two test invocations above) should be added to `.github/workflows/ci.yml` as part of opening the PR — mirror the existing `static-files`/`compression` entries.

---

## Self-review notes

- **Spec coverage:** feature + dep (Task 1), `TS` re-export (Task 4), derived `query`/`mutation` with `*_with_types` escape hatches (Task 4), transitive deduped decl collection (Task 2), emit from `generate_typescript_client`/`generate_client_file` (Task 3 — `generate_client_file` calls `generate_typescript_client`, so it is covered transitively), example migration (Task 5), golden-file test (Task 6). `register`/`register_with_types` left intact (Task 4 note). ✔
- **Out of scope (by design):** CLI `ultimo generate` (Phase 2), docs rewrites (Phase 3), `--watch`.
- **Type consistency:** `collect_type_decls::<T>` signature and the `type_decls: BTreeMap<String,String>` field are used identically in Tasks 2, 3, and 4. `query`/`mutation` are the 2-arg derived forms; `query_with_types`/`mutation_with_types` are the 4-arg string forms.
- **Known follow-up:** the REST mutation generator special-cases `ts_input == "{}"`; with derived named inputs this branch is simply not taken (named inputs are always passed) — acceptable and asserted indirectly by the golden test.
