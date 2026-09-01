# Ultimo MCP Server — Design

**Date:** 2026-09-01
**Status:** Approved (design)
**Target:** additive to `ultimo-cli` → patch release
**Part of:** the AI-native initiative (WS3), plan at `docs/superpowers/plans/` / `~/.claude/plans/wiggly-kindling-dove.md`

## Goal

Give a coding agent building an Ultimo app **live, typed tools** instead of guessing
from stale training data. Shipped as an `ultimo mcp` subcommand: a stdio
[Model Context Protocol](https://modelcontextprotocol.io) server the agent
(Claude Code, Cursor, …) connects to.

The agent gains five tools: search the current docs, fetch a doc page, list/get
runnable examples, scaffold a project, and — the differentiator — **introspect a
project's typed RPC surface**.

## Non-goals (v1)

- `api_lookup(symbol)` — docs search covers it (YAGNI).
- HTTP/SSE transport — stdio only (local agent tools).
- A separate published `ultimo-mcp` crate — it's a CLI subcommand, reusing the
  existing CLI distribution.
- End-to-end stdio testing in CI — the tool functions are tested directly.

## Architecture

- New subcommand `Commands::Mcp` in `ultimo-cli/src/main.rs`; new module
  `ultimo-cli/src/mcp/`.
- Server built on **`rmcp`** (the official Rust MCP SDK: `#[tool]` /
  `#[tool_router]` / `#[tool_handler]` macros, tokio, stdio transport). The CLI is
  already `#[tokio::main]` async + clap-derive, so this drops in cleanly.
- Tools are async methods on an `UltimoMcp` server struct, each `#[tool]`-annotated
  with a documented input schema (serde/schemars-derived per rmcp).

## Tools

### 1. `search_docs(query: String) -> String`
Fetch `https://docs.ultimo.dev/llms-full.txt`, split into sections (by page /
`##` heading), rank sections by query-term overlap, and return the top N (≈5)
with their page titles. Keeps the agent grounded in the deployed API.

### 2. `get_doc(page: String) -> String`
Return a named page's full text from the same corpus (match on page title/slug).

### 3. `list_examples() -> String` / `get_example(name: String) -> String`
`list_examples` returns a **bundled manifest** (name, one-line description,
GitHub URL) — examples change rarely, so a small in-binary list is fine.
`get_example` fetches the example's raw `main.rs` from
`raw.githubusercontent.com`.

### 4. `scaffold(template: String, name: String, path: Option<String>) -> String`
Run the `ultimo new` scaffolding into `path` (default: cwd), returning the created
file tree. Requires refactoring `new::run` to accept a base directory (today it
writes to cwd). Post-WS1 the output builds and includes `AGENTS.md`.

### 5. `introspect_rpc(project_path: String) -> String`
The differentiator. Reuse `generate.rs`'s runner (`cargo run --bin generate-client
-- <tmp>`) to build the project's own `RpcRegistry` and emit its typed client to a
temp file, then return: the generated TypeScript client text **plus** the
extracted procedure list (method names, query vs mutation). The agent asks "what's
my typed API surface?" and gets an exact, current answer. If the project has no
`generate-client` bin, return the same teaching error `ultimo generate` gives.

## Docs fetch + cache

A small fetch layer wraps `reqwest`:
- GET `llms-full.txt` (and, for `get_doc`, sections thereof), cache the corpus
  in-process with a TTL (e.g. 1 h).
- **The corpus source is an injectable closure** (mirroring the framework's
  `JwksClient` fetch-closure pattern in `ultimo/src/auth/jwks.rs`) so unit tests
  supply a fixed corpus string and never touch the network.

## Distribution & configuration

`ultimo mcp` serves over stdio. Documented client config (fills the stubbed MCP
section in `docs-site/docs/pages/ai-agents.mdx`):

```json
{ "mcpServers": { "ultimo": { "command": "ultimo", "args": ["mcp"] } } }
```

## Dependencies

Add to `ultimo-cli/Cargo.toml`:
- `rmcp` with the server + macros + stdio-transport features.
- `reqwest` with `rustls-tls` + `json` (no OpenSSL; matches the framework's `oidc`
  choice).

Both must pass `cargo-deny` (license allow-list; `CDLA-Permissive-2.0` already
allowed for webpki-roots) and `cargo-audit`. `tokio` (full) is already present.

## Error handling

- Network failures in `search_docs`/`get_doc`/`get_example` return a clear tool
  error ("couldn't reach docs.ultimo.dev — check connectivity"), never a panic.
- `introspect_rpc` surfaces the project's build/generate error text (actionable),
  and the missing-bin teaching error when applicable.
- `scaffold` errors (dir exists, unknown template) return the CLI's existing
  messages.

## Testing

- **Unit (`mcp` module):**
  - corpus parse + `search_docs` ranking over a fixed injected corpus (no network);
  - `get_doc` returns the right section from a fixed corpus;
  - examples manifest is well-formed (names/urls parse);
  - `scaffold` creates the expected files in a tempdir (mirrors existing `new` tests).
- **Integration:** `introspect_rpc` reuses the existing generate-client test
  harness (a tempdir project whose `generate-client` bin writes its arg) and
  asserts the returned client text + extracted names.
- No test hits the network (corpus + example fetches are injectable / skipped).

## SemVer & release

Additive to `ultimo-cli` (new subcommand, new deps) → **patch** bump.
`semver-checks` targets the `ultimo` library, unaffected. release-plz cuts the
`ultimo-cli` patch.

## Rollout / task shape (for the plan)

1. Subcommand + `rmcp` stdio server skeleton (one trivial `ping`/`search_docs` tool) — proves the transport.
2. Docs tools: fetch+cache layer (injectable), `search_docs`, `get_doc`.
3. Examples tools: bundled manifest + `list_examples`/`get_example`.
4. `scaffold` (refactor `new::run` to take a base dir) + tool.
5. `introspect_rpc` (reuse `generate.rs` runner) + tool.
6. Docs + config: fill `ai-agents.mdx` MCP section, `cli.mdx`, roadmap bump (WS5 overlap).
7. Gate + PR.

## Follow-ups (not this spec)

- `api_lookup(symbol)` for exact framework signatures.
- HTTP/SSE transport for hosted use.
- Deeper RPC introspection (typed schema graph, not just the generated client).
