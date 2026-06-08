# Static File Serving + Response Compression — Design

**Date:** 2026-06-08
**Issues:** v0.5.0 milestone
**Status:** Approved, pre-implementation

---

## Goal

Two complementary features that together tell the "deploy your full-stack Ultimo app" story:

1. **Static file serving + SPA fallback** — serve frontend assets from disk; fall back to `index.html` for client-side-routed SPAs.
2. **Response compression** — gzip / brotli via a built-in middleware, negotiated from `Accept-Encoding`.

Both ship in one PR (`feat/v0.5.0-static-compression`), behind separate Cargo feature flags, with a shared demo example.

---

## Feature 1: Static File Serving (`static-files`)

### Cargo feature

```toml
[features]
static-files = ["dep:mime_guess"]

[dependencies]
mime_guess = { version = "2", optional = true }  # pure Rust; no C deps
```

`tokio::fs` is already available via the `tokio` workspace dep — no additional async I/O dep needed.

### Public API

```rust
// Serve ./public under /assets prefix
app.serve_static("/assets", "./public");

// SPA: all unmatched GET routes → index.html (from ./dist)
app.serve_spa("./dist", "index.html");
```

Both methods are on `Ultimo`, gated with `#[cfg(feature = "static-files")]`. They return `&mut Self` for chaining.

### Implementation

**New file:** `ultimo/src/static_files.rs` (gated on `#[cfg(feature = "static-files")]`)

**`serve_static(prefix, dir)`:**
- Registers a wildcard `GET {prefix}/*path` route on the router.
- The handler resolves the file path as `dir / path`, canonicalizes it, and asserts it is still inside `dir` (path traversal guard — any escape → 404, never 403, to avoid leaking existence).
- Reads the file with `tokio::fs::read()`.
- Sets `Content-Type` via `mime_guess::from_path(&file_path).first_or_octet_stream()`.
- ETag: `format!("{}-{}", metadata.len(), mtime_secs)` — no extra dep; `std::fs::Metadata`.
- If `If-None-Match` header matches ETag → 304 Not Modified (empty body).
- Missing file → 404 (not 500; only I/O errors that aren't "not found" become 500).

**`serve_spa(dir, fallback)`:**
- Stores `(dir, fallback)` in a new field `spa_fallback: Option<(PathBuf, String)>` on `Ultimo`.
- In `dispatch_parts`, after routing + middleware execution, if the response status is 404 **and** `spa_fallback` is set, replace the response with the fallback file (same MIME/ETag logic as above).
- Only applies to `GET` requests (POST/PUT/etc. 404s pass through untouched).

### Security

- **Path traversal**: `std::fs::canonicalize(resolved)` then `resolved.starts_with(canonical_root)` — fail → 404.
- **Symlinks**: canonicalize follows symlinks, so the root check still catches escapes via symlink.
- No directory listing: if path resolves to a directory → 404 (no index autodetect, explicit SPA fallback is the pattern).

### Caching headers

- `ETag`: `"{len}-{mtime_secs}"` — deterministic, cheap.
- `Last-Modified`: set from `mtime`.
- `Cache-Control`: not set by default (let the app control it; `serve_static` could gain a `.cache_control(...)` builder later).

---

## Feature 2: Response Compression (`compression`)

### Cargo feature

```toml
[features]
compression = ["dep:flate2", "dep:brotli"]

[dependencies]
flate2  = { version = "1",  optional = true }  # gzip via miniz_oxide — pure Rust
brotli  = { version = "7",  optional = true }  # brotli — pure Rust
```

Both are 100% pure Rust; no C/system libs.

### Public API

Follows the existing `Cors` / `SecurityHeaders` builder pattern exactly:

```rust
// Convenience: gzip + brotli, min 1 KB, added to builtin module
app.use_middleware(compression());

// Configured
app.use_middleware(
    Compression::new()
        .gzip()
        .brotli()
        .min_size(512)
        .build()
);
```

`compression()` is the zero-config convenience fn (same pattern as `cors()`, `security_headers()`).

### Implementation

**Lives in:** `ultimo/src/middleware.rs` `builtin` module (same file, same pattern as `SecurityHeaders`).
Gated on `#[cfg(feature = "compression")]`.

**`Compression` struct fields:**
- `gzip: bool` (default `true`)
- `brotli: bool` (default `true`)
- `min_size: usize` (default `1024`)

**Middleware logic (in `build()`):**

1. Extract `Accept-Encoding` header value from the *request* before calling `next`.
2. Call `next(ctx).await?` to get the response.
3. Decide algorithm:
   - If response already has `Content-Encoding` → pass through (never double-compress).
   - If response body < `min_size` → pass through.
   - If `Content-Type` matches a skip list → pass through.
   - Else: parse `Accept-Encoding`, prefer `br` > `gzip` > identity.
4. Compress body bytes in-place (synchronous; bodies are already buffered `Bytes`).
5. Set `Content-Encoding: br` or `Content-Encoding: gzip`.
6. Set `Vary: Accept-Encoding` (always, even if not compressed — correct per RFC 7231).
7. Update `Content-Length` to compressed size.

**Accept-Encoding parsing:** naive but correct for our use case — check `.contains("br")` and `.contains("gzip")` after splitting on `,` and trimming quality values. Quality (`q=`) negotiation is not implemented (overkill; `br` is always preferred when listed).

**Skip list:**

```rust
const SKIP_PREFIXES: &[&str] = &["image/", "audio/", "video/", "font/woff"];
const SKIP_EXACT: &[&str] = &[
    "application/zip",
    "application/gzip",
    "application/x-gzip",
    "application/octet-stream",
];
```

Content-Type check: `first_or_octet_stream().essence_str()` — no dep, just string matching.

### Compression implementation notes

- **gzip**: `flate2::write::GzEncoder` with `Compression::default()` (level 6).
- **brotli**: `brotli::CompressorWriter` with default quality (5) — balances speed vs ratio; not the max (11) which is slow.
- Both encode a `&[u8]` into a `Vec<u8>`, then wrap in `Bytes::from(vec)`.

---

## Example

**New example:** `examples/spa-demo/`

A minimal Ultimo backend that:
1. Serves a tiny hand-written "SPA" from `./dist/` (a static `index.html` + `app.js`).
2. Uses `serve_spa("./dist", "index.html")` so `/`, `/about`, `/user/123` all return `index.html`.
3. Wraps an API route `GET /api/hello` alongside the static serving.
4. Mounts `compression()` middleware so assets are served gzip/brotli-compressed.

No build step required — the `dist/` dir is committed as-is (tiny HTML+JS).

Added to `Cargo.toml` workspace `members`.

---

## Documentation surfaces (all in this PR)

| Surface | Change |
|---|---|
| `docs-site/docs/pages/static-files.mdx` | New page: serve_static + serve_spa API, SPA pattern, security note |
| `docs-site/docs/pages/middleware.mdx` | Add "Compression" section (after security headers) |
| `docs-site/vocs.config.ts` | Add "Static Files" under Introduction; "Compression" under Middleware |
| `docs-site/docs/pages/api-reference.mdx` | `serve_static`, `serve_spa`, `Compression`, `compression()` |
| `README.md` | Remove "Static Files" from coming-soon; update feature flags table |
| `docs-site/docs/pages/roadmap.mdx` | Static Files + Compression → ✅ Available 0.5.0 |

---

## Tests

All via `Ultimo::oneshot`.

### Static files (`ultimo/tests/static_files.rs`, `#[cfg(feature = "static-files")]`)

| Test | Assertion |
|---|---|
| `existing_file_returns_200_with_mime` | GET /assets/hello.txt → 200, Content-Type: text/plain |
| `missing_file_returns_404` | GET /assets/nope.txt → 404 |
| `path_traversal_is_blocked` | GET /assets/../../etc/passwd → 404 |
| `etag_is_set_on_response` | Response has ETag header |
| `conditional_get_returns_304` | 2nd GET with If-None-Match matching ETag → 304 |
| `spa_fallback_serves_index` | GET /unknown-route with serve_spa → 200, body contains index.html |
| `spa_fallback_does_not_affect_api` | POST /api/something → 404 passes through (no SPA override) |

### Compression (`ultimo/tests/compression.rs`, `#[cfg(feature = "compression")]`)

| Test | Assertion |
|---|---|
| `gzip_accepted_returns_gzip` | Accept-Encoding: gzip → Content-Encoding: gzip |
| `brotli_accepted_returns_brotli` | Accept-Encoding: br → Content-Encoding: br |
| `brotli_preferred_over_gzip` | Accept-Encoding: gzip, br → Content-Encoding: br |
| `no_accept_encoding_no_compression` | No header → no Content-Encoding |
| `image_not_compressed` | image/png response → no Content-Encoding regardless |
| `small_body_not_compressed` | Body < min_size → no Content-Encoding |
| `vary_header_always_set` | Every response has Vary: Accept-Encoding |
| `already_encoded_not_double_compressed` | Response with Content-Encoding set → passed through |

---

## Constraints

- Both features are `default = []` — existing users unaffected.
- Both deps (`mime_guess`, `flate2`, `brotli`) are pure Rust — no system lib requirements.
- `#![forbid(unsafe_code)]` maintained throughout.
- `semver-checks` CI job: only new `pub` items added (no removals), so patch bump is correct.
- No breaking changes to existing public API.

---

## Success criteria

- `cargo test -p ultimo --features "static-files,compression"` green.
- `cargo clippy -p ultimo --features "static-files,compression" -- -D warnings` clean.
- `cargo test -p ultimo --doc --features "static-files,compression"` green.
- `cargo run -p spa-demo` serves the SPA and responds with `Content-Encoding: gzip` on curl with `Accept-Encoding: gzip`.
- Path traversal test passes (security).
- All 8 doc surfaces updated.
