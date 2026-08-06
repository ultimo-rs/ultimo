# Frontend Client Adapters (TanStack Query / React) — Design

**Date:** 2026-08-06
**Status:** Approved (brainstorm), pending spec review
**Roadmap:** 0.6 — "Frontend client adapters" (the differentiator multiplier)
**Builds on:** the `client-gen` feature (TypeScript client generation from the RPC registry)

## Goal

Generate **TanStack Query React hooks** from the RPC registry, on top of the
existing generated `UltimoRpcClient`. This extends Ultimo's type-safe story all
the way into the UI: `const { data } = useGetUser({ id })` — fully typed input
and output, with caching and invalidation — instead of hand-wiring `useQuery`
around a bare client method.

## Principles

- **Pure codegen, zero new Ultimo runtime deps.** The emitted hooks `import`
  the user's own `@tanstack/react-query` (a peer dependency they already have)
  and the generated `UltimoRpcClient`. Ultimo publishes/maintains no npm runtime
  package — consistent with the codegen-first, minimal-deps identity.
- **Reuse existing registry metadata.** Each procedure already carries `name`,
  `is_query` (query vs mutation), and derived `ts_input`/`ts_output`. The hooks
  emitter needs no new registration data.
- **Mode-agnostic.** Both JSON-RPC and REST generated clients expose the same
  `client.<name>(input): Promise<Output>` method shape, so one hooks emitter
  serves both `RpcMode`s.
- **YAGNI on frameworks.** React + TanStack Query only for v1. The emitter is
  structured so Svelte/Vue become additional output targets later, not a rewrite.

## Architecture

### Rust API

A new method on `RpcRegistry`, mirroring the existing `generate_client_file`:

```rust
/// Generate a TanStack Query (React) hooks module for this registry.
/// Requires the `client-gen` feature. Emits alongside the client file.
pub fn generate_react_hooks_file(&self, output_path: &str) -> std::io::Result<()>;
```

Plus the string-returning core it wraps (mirrors `generate_typescript_client`):

```rust
pub fn generate_react_hooks(&self) -> String;
```

Gated under the existing `client-gen` feature — **no new Cargo feature**. Users
call it from their `generate-client` binary next to the client generation:

```rust
// src/bin/generate-client.rs
let rpc = my_app::build_registry();
rpc.generate_client_file("./client/client.ts")?;
rpc.generate_react_hooks_file("./client/hooks.ts")?;
```

### Generated output (`hooks.ts`)

Given a registry with query `getUser(GetUserInput) -> User` and mutation
`createUser(CreateUserInput) -> User`, the emitted module contains:

1. **Imports** — from `@tanstack/react-query` (`useQuery`, `useMutation`,
   `UseQueryOptions`, `UseMutationOptions`, `useQueryClient`) and from the
   generated client (`./client` — the `UltimoRpcClient` + the `type` exports).

> **Prerequisite (included in this work):** the client generator currently emits
> its type declarations un-exported (`type User = { … }` in
> `append_type_definitions`). For a separate hooks module to `import type` them,
> the client must emit `export type User = { … }`. Prepending `export ` to each
> emitted declaration is a small, backward-compatible change (adding `export`
> never breaks existing consumers) and is part of this spec.
2. **Client context** — the chosen injection mechanism (below).
3. **`queryKeys`** — a const object of key factories for cache invalidation:
   ```ts
   export const queryKeys = {
     getUser: (input: GetUserInput) => ['getUser', input] as const,
   };
   ```
4. **One hook per procedure:**
   - Query → `useGetUser(input, options?)`:
     ```ts
     export function useGetUser(
       input: GetUserInput,
       options?: Omit<UseQueryOptions<User>, 'queryKey' | 'queryFn'>,
     ) {
       const client = useUltimoClient();
       return useQuery({
         queryKey: queryKeys.getUser(input),
         queryFn: () => client.getUser(input),
         ...options,
       });
     }
     ```
   - Mutation → `useCreateUser(options?)`:
     ```ts
     export function useCreateUser(
       options?: UseMutationOptions<User, Error, CreateUserInput>,
     ) {
       const client = useUltimoClient();
       return useMutation({
         mutationFn: (input: CreateUserInput) => client.createUser(input),
         ...options,
       });
     }
     ```

### Client injection — React Context (chosen)

The emitted module defines a provider + internal hook:

```ts
const ClientContext = createContext<UltimoRpcClient | null>(null);

export function UltimoProvider(
  props: { client: UltimoRpcClient; children: React.ReactNode },
) {
  return <ClientContext.Provider value={props.client}>{props.children}</ClientContext.Provider>;
}

function useUltimoClient(): UltimoRpcClient {
  const client = useContext(ClientContext);
  if (!client) throw new Error('useUltimoClient must be used within <UltimoProvider>');
  return client;
}
```

The app wraps its tree once: `<UltimoProvider client={new UltimoRpcClient()}>`
(inside the app's existing `<QueryClientProvider>`). Hooks are then
import-and-use with no per-call wiring. Chosen over passing the client to every
hook (less boilerplate, idiomatic TanStack).

> Because the module contains JSX, it is emitted as `hooks.tsx`. Imports for
> `React`/`createContext`/`useContext` are included.

## CLI / example

- No CLI change required for v1 — generation is driven by the user's
  `generate-client` bin (Phase 2 convention). Wiring `generate_react_hooks_file`
  into `ultimo new` scaffolds is a follow-up, tracked with the existing
  "scaffold `generate-client`" 0.6 item.
- Add a runnable example (`examples/react-hooks` or extend `examples/react-app-rpc`)
  showing `<UltimoProvider>` + a component using `useGetUser` / `useCreateUser`.

## Error handling

- Mutations type the error channel as `Error`; the generated client already
  throws a typed error (`JsonRpcClientError` for JSON-RPC mode). v1 uses `Error`
  as the TanStack error type to stay mode-agnostic; end-to-end typed errors are
  a separate 0.9 roadmap item and will refine this later.
- `useUltimoClient` throws a clear message if used outside `<UltimoProvider>`.

## Testing

- **Golden-file test** (primary): a fixture registry with one query and one
  mutation → assert the emitted `hooks.tsx` contains: the `@tanstack/react-query`
  imports, `UltimoProvider`, `queryKeys.<name>`, a `useQuery`-based hook for the
  query, a `useMutation`-based hook for the mutation, and correct typed
  signatures referencing the derived type names.
- Reuse the `client-gen` test harness/feature-gating pattern from
  `ultimo/tests/client_gen.rs`.

## Scope / phasing

- **This spec (v1):** `generate_react_hooks` + `generate_react_hooks_file`,
  React Context injection, `queryKeys`, golden test, docs, one example.
- **Later (out of scope):** Svelte/Vue emitters; `ultimo new` scaffolding of the
  hooks call; typed-error channel (0.9); infinite-query / suspense variants.

## Out of scope

No changes to procedure registration, the RPC runtime, or the existing client
generator's output. This is additive: a new emitter reading existing metadata.
