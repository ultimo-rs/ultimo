# Frontend React Hooks (TanStack Query) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate TanStack Query React hooks from the RPC registry, on top of the existing `client-gen` TypeScript client.

**Architecture:** A new emitter `generate_react_hooks` reads the existing registry metadata (`type_definitions` + `metadata.is_query`) and produces a `.ts` module: a React Context (`UltimoProvider` / internal `useUltimoClient`), a `queryKeys` factory, and one `useQuery`/`useMutation` hook per procedure. Hook input/output types are derived from the generated client's own method signatures via `Parameters<>` / `Awaited<ReturnType<>>`, so they're correct regardless of how a procedure was registered. Pure codegen — the emitted file imports the user's `@tanstack/react-query` and the generated `UltimoRpcClient`.

**Tech Stack:** Rust, the `client-gen` Cargo feature (`ts-rs` 12), TanStack Query v5 (consumer peer dep).

**Spec:** `docs/superpowers/specs/2026-08-06-frontend-react-hooks-design.md`

## Global Constraints

- All new library code lives behind the existing `client-gen` Cargo feature — **no new Cargo feature**.
- 100% safe Rust (`#![forbid(unsafe_code)]`); no new runtime dependencies.
- The emitted module uses `createElement` (not JSX) so it is a portable `.ts` file needing no JSX build config.
- Hook types are derived via `Parameters<UltimoRpcClient['name']>[0]` and `Awaited<ReturnType<UltimoRpcClient['name']>>` — never by importing named types.
- Integration tests are feature-gated with `#![cfg(feature = "client-gen")]` and use `ts-rs` (already a dev-dependency).

---

### Task 1: Export the client's type declarations

The client emits `type User = { … }` un-exported. Prepend `export ` so consumers (and the hooks module) can reference the named types. Backward-compatible (adding `export` never breaks importers).

**Files:**
- Modify: `ultimo/src/rpc.rs` — `append_type_definitions` (currently ~line 540)
- Test: `ultimo/tests/client_gen.rs`

**Interfaces:**
- Consumes: `self.type_decls` (`BTreeMap<String, String>` of TS name → `type X = { … };`).
- Produces: generated client now contains `export type X = { … };` for every collected declaration.

- [ ] **Step 1: Update the golden client test to require exported decls**

In `ultimo/tests/client_gen.rs`, change the existing output-type assertion so it requires the `export` keyword. Replace the line asserting `client.contains("type User = ")` with:

```rust
    assert!(
        client.contains("export type User = "),
        "User decl must be exported:\n{client}"
    );
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p ultimo --features client-gen --test client_gen`
Expected: FAIL — decls are currently emitted without `export`.

- [ ] **Step 3: Prepend `export ` in `append_type_definitions`**

In `ultimo/src/rpc.rs`, change the decl-emitting loop:

```rust
    /// Append collected type declarations to the generated client.
    fn append_type_definitions(&self, client: &mut String) {
        let decls = self.type_decls.lock().unwrap();
        if decls.is_empty() {
            return;
        }
        client.push_str("\n// Type Definitions\n");
        for decl in decls.values() {
            // ts-rs emits `type X = { … };`; export it so other modules
            // (and the generated hooks) can import the named type.
            client.push_str("export ");
            client.push_str(decl);
            client.push('\n');
        }
    }
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p ultimo --features client-gen --test client_gen`
Expected: PASS. (The existing `assert!(!client.contains("export interface User"))` still holds — we emit `export type`, not `export interface`.)

- [ ] **Step 5: Commit**

```bash
git add ultimo/src/rpc.rs ultimo/tests/client_gen.rs
git commit -m "feat(rpc): export generated client type declarations"
```

---

### Task 2: The React hooks emitter

Add `generate_react_hooks` (string) + `generate_react_hooks_file` (writes it), plus a `hook_name` helper. Golden-file integration test.

**Files:**
- Modify: `ultimo/src/rpc.rs` — add `hook_name` free fn + two methods on `impl RpcRegistry`
- Create: `ultimo/tests/react_hooks.rs`

**Interfaces:**
- Consumes: `self.type_definitions` (`Vec<TypeDefinition>` with `.name`), `self.metadata` (`HashMap<String, ProcedureMetadata>` with `.is_query`).
- Produces:
  - `pub fn generate_react_hooks(&self) -> String` — the hooks module text.
  - `pub fn generate_react_hooks_file(&self, output_path: &str) -> std::io::Result<()>` — writes it.
  - Both `#[cfg(feature = "client-gen")]`.

- [ ] **Step 1: Write the failing golden test**

Create `ultimo/tests/react_hooks.rs`:

```rust
//! Golden-file test for generated TanStack Query React hooks.
//! Run with: cargo test -p ultimo --features client-gen --test react_hooks

#![cfg(feature = "client-gen")]

use ultimo::rpc::{RpcMode, RpcRegistry, TS};

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct GetUserInput {
    id: u32,
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct CreateUserInput {
    name: String,
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct User {
    id: u32,
    name: String,
}

fn registry() -> RpcRegistry {
    let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);
    rpc.query("getUser", |input: GetUserInput| async move {
        Ok(User { id: input.id, name: "x".into() })
    });
    rpc.mutation("createUser", |input: CreateUserInput| async move {
        Ok(User { id: 1, name: input.name })
    });
    rpc
}

#[test]
fn hooks_module_has_provider_query_and_mutation() {
    let hooks = registry().generate_react_hooks();

    // Imports the peer dep + the generated client (not named types).
    assert!(hooks.contains("@tanstack/react-query"), "missing tanstack import:\n{hooks}");
    assert!(hooks.contains("import { UltimoRpcClient } from './client'"), "missing client import:\n{hooks}");

    // Context injection.
    assert!(hooks.contains("export function UltimoProvider"), "missing provider:\n{hooks}");
    assert!(
        hooks.contains("must be used within <UltimoProvider>"),
        "missing client guard:\n{hooks}"
    );

    // Query hook uses useQuery with a stable key and calls the client method.
    assert!(hooks.contains("export function useGetUser("), "missing query hook:\n{hooks}");
    assert!(hooks.contains("return useQuery({"), "query must use useQuery:\n{hooks}");
    assert!(hooks.contains("queryKey: queryKeys.getUser(input)"), "missing query key:\n{hooks}");
    assert!(hooks.contains("queryFn: () => client.getUser(input)"), "missing query fn:\n{hooks}");

    // Types derived from the client signature, not imported names.
    assert!(
        hooks.contains("Parameters<UltimoRpcClient['getUser']>[0]"),
        "input type not derived from client:\n{hooks}"
    );
    assert!(
        hooks.contains("Awaited<ReturnType<UltimoRpcClient['getUser']>>"),
        "output type not derived from client:\n{hooks}"
    );

    // Mutation hook uses useMutation.
    assert!(hooks.contains("export function useCreateUser("), "missing mutation hook:\n{hooks}");
    assert!(hooks.contains("return useMutation({"), "mutation must use useMutation:\n{hooks}");
    assert!(
        hooks.contains("mutationFn: (input:") && hooks.contains("client.createUser(input)"),
        "missing mutation fn:\n{hooks}"
    );

    // queryKeys covers the query but NOT the mutation.
    assert!(hooks.contains("getUser: (input:"), "missing queryKeys.getUser:\n{hooks}");
    assert!(!hooks.contains("createUser: (input:"), "mutation must not appear in queryKeys:\n{hooks}");
}

#[test]
fn generate_react_hooks_file_writes_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hooks.ts");
    registry()
        .generate_react_hooks_file(path.to_str().unwrap())
        .expect("writes file");
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("export function useGetUser("));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p ultimo --features client-gen --test react_hooks`
Expected: FAIL to compile — `generate_react_hooks` / `generate_react_hooks_file` don't exist.

- [ ] **Step 3: Add the `hook_name` helper**

In `ultimo/src/rpc.rs`, near `collect_type_decls` (both are `#[cfg(feature = "client-gen")]` free fns), add:

```rust
/// `getUser` -> `useGetUser`. Capitalizes the first character and prefixes `use`.
#[cfg(feature = "client-gen")]
fn hook_name(proc_name: &str) -> String {
    let mut chars = proc_name.chars();
    match chars.next() {
        Some(first) => format!("use{}{}", first.to_uppercase(), chars.as_str()),
        None => "use".to_string(),
    }
}
```

- [ ] **Step 4: Add `generate_react_hooks` and `generate_react_hooks_file`**

In `ultimo/src/rpc.rs`, add these methods to `impl RpcRegistry` (next to `generate_typescript_client` / `generate_client_file`):

```rust
    /// Generate a TanStack Query (React) hooks module for this registry.
    /// Requires the `client-gen` feature. The emitted module imports the user's
    /// `@tanstack/react-query` and the generated `UltimoRpcClient` from `./client`.
    #[cfg(feature = "client-gen")]
    pub fn generate_react_hooks(&self) -> String {
        let type_defs = self.type_definitions.lock().unwrap();
        let metadata = self.metadata.lock().unwrap();

        let mut out = String::from(
            r#"// Auto-generated TanStack Query hooks for Ultimo RPC
// DO NOT EDIT - This file is automatically generated
import { createContext, createElement, useContext, type ReactNode } from 'react';
import {
  useQuery,
  useMutation,
  type UseQueryOptions,
  type UseMutationOptions,
} from '@tanstack/react-query';
import { UltimoRpcClient } from './client';

const ClientContext = createContext<UltimoRpcClient | null>(null);

export function UltimoProvider(props: { client: UltimoRpcClient; children: ReactNode }) {
  return createElement(ClientContext.Provider, { value: props.client }, props.children);
}

function useUltimoClient(): UltimoRpcClient {
  const client = useContext(ClientContext);
  if (!client) {
    throw new Error('useUltimoClient must be used within <UltimoProvider>');
  }
  return client;
}

"#,
        );

        // queryKeys — one factory per query procedure (mutations excluded).
        out.push_str("export const queryKeys = {\n");
        for def in type_defs.iter() {
            let is_query = metadata.get(&def.name).map(|m| m.is_query).unwrap_or(false);
            if is_query {
                out.push_str(&format!(
                    "  {name}: (input: Parameters<UltimoRpcClient['{name}']>[0]) => ['{name}', input] as const,\n",
                    name = def.name
                ));
            }
        }
        out.push_str("};\n\n");

        // One hook per procedure.
        for def in type_defs.iter() {
            let is_query = metadata.get(&def.name).map(|m| m.is_query).unwrap_or(false);
            let hook = hook_name(&def.name);
            let name = &def.name;
            if is_query {
                out.push_str(&format!(
                    r#"export function {hook}(
  input: Parameters<UltimoRpcClient['{name}']>[0],
  options?: Omit<UseQueryOptions<Awaited<ReturnType<UltimoRpcClient['{name}']>>>, 'queryKey' | 'queryFn'>,
) {{
  const client = useUltimoClient();
  return useQuery({{
    queryKey: queryKeys.{name}(input),
    queryFn: () => client.{name}(input),
    ...options,
  }});
}}

"#
                ));
            } else {
                out.push_str(&format!(
                    r#"export function {hook}(
  options?: UseMutationOptions<Awaited<ReturnType<UltimoRpcClient['{name}']>>, Error, Parameters<UltimoRpcClient['{name}']>[0]>,
) {{
  const client = useUltimoClient();
  return useMutation({{
    mutationFn: (input: Parameters<UltimoRpcClient['{name}']>[0]) => client.{name}(input),
    ...options,
  }});
}}

"#
                ));
            }
        }

        out
    }

    /// Generate the React hooks module and write it to `output_path`.
    /// Requires the `client-gen` feature.
    #[cfg(feature = "client-gen")]
    pub fn generate_react_hooks_file(&self, output_path: &str) -> std::io::Result<()> {
        std::fs::write(output_path, self.generate_react_hooks())
    }
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p ultimo --features client-gen --test react_hooks`
Expected: PASS (both tests).

- [ ] **Step 6: Commit**

```bash
git add ultimo/src/rpc.rs ultimo/tests/react_hooks.rs
git commit -m "feat(rpc): generate TanStack Query React hooks (client-gen)"
```

---

### Task 3: Example, docs, CI, and verification gate

Demonstrate the emitter end-to-end, document it, wire it into CI, and run the gate.

**Files:**
- Modify: `examples/rpc-modes/src/main.rs`, `examples/rpc-modes/Cargo.toml` (already `client-gen`)
- Create: `examples/rpc-modes/ultimo-hooks.ts` (generated artifact, committed for illustration)
- Modify: `docs-site/docs/pages/typescript.mdx`
- Modify: `docs-site/docs/pages/roadmap.mdx`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `RpcRegistry::generate_react_hooks_file` from Task 2.
- Produces: a runnable example that emits `ultimo-hooks.ts`; docs + CI coverage.

- [ ] **Step 1: Emit hooks from the rpc-modes example**

In `examples/rpc-modes/src/main.rs`, right after the existing REST client generation (`rest_rpc.generate_client_file("ultimo-client-rest.ts")?;`), add:

```rust
    // Also emit TanStack Query React hooks alongside the client.
    rest_rpc.generate_react_hooks_file("ultimo-hooks.ts")?;
    println!("✅ React hooks generated: ultimo-hooks.ts");
```

- [ ] **Step 2: Generate the artifact and confirm it builds + emits**

Run: `cargo run -p rpc-modes`
Expected: the run prints the new line and writes `ultimo-hooks.ts` at the workspace root. Move it next to the example's other generated artifacts so it is committed with them:

```bash
mv ultimo-hooks.ts examples/rpc-modes/ultimo-hooks.ts
```

Open `examples/rpc-modes/ultimo-hooks.ts` and confirm it contains `UltimoProvider`, `useGetUserById` (or the example's query hooks), and `useCreateUser`.

- [ ] **Step 3: Document the hooks in the TypeScript page**

In `docs-site/docs/pages/typescript.mdx`, add a section (place it after the client-usage section):

````mdx
## React hooks (TanStack Query)

Generate typed TanStack Query hooks alongside the client — types flow all the
way into your components. In your `generate-client` binary:

```rust
rpc.generate_client_file("./src/lib/client.ts")?;
rpc.generate_react_hooks_file("./src/lib/hooks.ts")?;
```

Wrap your app once (inside your existing `QueryClientProvider`):

```tsx
import { UltimoProvider } from './lib/hooks';
import { UltimoRpcClient } from './lib/client';

<UltimoProvider client={new UltimoRpcClient()}>
  <App />
</UltimoProvider>
```

Then use the generated hooks — fully typed input and output:

```tsx
import { useGetUser, useCreateUser, queryKeys } from './lib/hooks';

function Profile({ id }: { id: number }) {
  const { data, isLoading } = useGetUser({ id });          // data: User | undefined
  const create = useCreateUser();                          // create.mutate({ name })
  // invalidate: queryClient.invalidateQueries({ queryKey: queryKeys.getUser({ id }) })
}
```

Queries become `useQuery` hooks (`useGetUser`), mutations become `useMutation`
hooks (`useCreateUser`), and `queryKeys` provides stable keys for cache
invalidation. Requires `@tanstack/react-query` v5 in your frontend.
````

- [ ] **Step 4: Reconcile the roadmap**

In `docs-site/docs/pages/roadmap.mdx`, update the Feature Status row `Frontend Client Adapters (TanStack Query / hooks)` from `📋 Planned | 0.6.0` to `✅ Available | 0.6.0`, and in the `### v0.6.0` timeline section change the frontend-client-adapters bullet to a shipped one:

```mdx
- ✅ **Frontend client adapters** — TanStack Query / React hooks (`generate_react_hooks_file`, `UltimoProvider`, `queryKeys`)
```

- [ ] **Step 5: Add the hooks test to CI**

In `.github/workflows/ci.yml`, in the `Client-gen tests (client-gen feature)` step, add the new test target so the block runs:

```yaml
      - name: Client-gen tests (client-gen feature)
        run: |
          cargo test -p ultimo --features "client-gen" --lib client_gen_tests
          cargo test -p ultimo --features "client-gen" --test client_gen
          cargo test -p ultimo --features "client-gen" --test react_hooks
          cargo build -p rpc-modes
```

- [ ] **Step 6: Run the verification gate**

Run:
```bash
cargo fmt --all --check
cargo clippy -p ultimo --lib --features "client-gen" -- -D warnings
cargo test -p ultimo --features client-gen --test client_gen
cargo test -p ultimo --features client-gen --test react_hooks
cargo build -p rpc-modes
bash scripts/check-versions.sh
```
Expected: all clean/pass. (Run `cargo fmt --all` first if the format check flags anything.)

- [ ] **Step 7: Commit**

```bash
git add examples/rpc-modes docs-site/docs/pages/typescript.mdx docs-site/docs/pages/roadmap.mdx .github/workflows/ci.yml
git commit -m "docs+example+ci: React hooks (rpc-modes artifact, TS docs, roadmap, CI)"
```

---

## Self-review notes

- **Spec coverage:** `generate_react_hooks` + `generate_react_hooks_file` (Task 2), export type decls prerequisite (Task 1), Context injection `UltimoProvider`/`useUltimoClient` (Task 2), `queryKeys` (Task 2), per-procedure `useQuery`/`useMutation` (Task 2), golden test (Task 2), example (Task 3), docs (Task 3). Feature-gated under `client-gen`; no new Cargo feature. ✔
- **Deviation from spec (improvement):** the spec floated `.tsx` + JSX and importing named types; the plan uses `createElement` (portable `.ts`, no JSX build config) and derives types via `Parameters`/`ReturnType` (robust for inline/`any` types, and it means the "export type decls" change is a nice-to-have for consumers rather than a hard hooks prerequisite — Task 1 still does it since it's independently useful and spec'd). Same design intent (Context provider, typed hooks).
- **Type consistency:** `generate_react_hooks` / `generate_react_hooks_file` / `hook_name` names and signatures are identical across Task 2 and Task 3. Hooks reference `UltimoRpcClient` (matches the generated client's class name) and `client.<name>(input)` (matches the generated method shape `async <name>(params): Promise<...>`).
- **Out of scope (per spec):** Svelte/Vue emitters, `ultimo new` scaffolding of the hooks call, typed-error channel, a full React frontend demo app.
