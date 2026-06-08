# Static File Serving + Response Compression Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` (inline, recommended for cost) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `serve_static`/`serve_spa` (behind `static-files` feature) and gzip/brotli `Compression` middleware (behind `compression` feature) for Ultimo v0.5.0.

**Architecture:** Static file serving adds a `Segment::Wildcard` variant to the router (enables `*name` catch-all segments), then `serve_static(prefix, dir)` registers a normal `GET prefix/*path` route whose handler calls `serve_file()`. `serve_spa(dir, fallback)` stores a `(PathBuf, String)` on `Ultimo` and `dispatch_parts` checks it when the response is 404+GET. Response compression is a `BoxedMiddleware` in `middleware::builtin` that captures `Accept-Encoding` before calling `next(ctx)` and wraps the buffered response body.

**Tech Stack:** `mime_guess = "2"` (MIME detection), `flate2 = "1"` (gzip, pure Rust via miniz_oxide), `brotli = "7"` (brotli, pure Rust). `tokio::fs` for async file I/O (already in workspace).

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `Cargo.toml` (workspace root) | Add `examples/spa-demo` to members |
| Modify | `ultimo/Cargo.toml` | Add `mime_guess`, `flate2`, `brotli` deps + `static-files`, `compression` features + `tempfile` dev-dep |
| Modify | `ultimo/src/router.rs` | Add `Segment::Wildcard`, update `parse_path` + `matches()` |
| Create | `ultimo/src/static_files.rs` | `serve_file(root, rel_path, if_none_match)` → `Result<Response>` |
| Modify | `ultimo/src/lib.rs` | `pub mod static_files` gated on `static-files` |
| Modify | `ultimo/src/app.rs` | `spa_fallback` field; `serve_static` + `serve_spa` methods; SPA check in `dispatch_parts` |
| Modify | `ultimo/src/middleware.rs` | `Compression` struct + `compression()` fn in `builtin` module |
| Create | `ultimo/tests/static_files.rs` | Integration tests for static serving |
| Create | `ultimo/tests/compression.rs` | Integration tests for compression |
| Create | `examples/spa-demo/Cargo.toml` | SPA demo crate manifest |
| Create | `examples/spa-demo/src/main.rs` | Demo: serve_spa + compression |
| Create | `examples/spa-demo/dist/index.html` | Minimal SPA HTML |
| Create | `examples/spa-demo/dist/app.js` | Minimal SPA JS |
| Create | `docs-site/docs/pages/static-files.mdx` | Static file serving docs page |
| Modify | `docs-site/docs/pages/middleware.mdx` | Add Compression section |
| Modify | `docs-site/vocs.config.ts` | Add Static Files + Compression nav entries |
| Modify | `docs-site/docs/pages/api-reference.mdx` | Document new public API items |
| Modify | `README.md` | Update feature flags table |
| Modify | `docs-site/docs/pages/roadmap.mdx` | Static Files + Compression → ✅ 0.5.0 |
| Modify | `.github/workflows/ci.yml` | Add `static-files,compression` feature test runs |

---

## Task 1: Branch + Cargo deps

**Files:** `Cargo.toml` (root), `ultimo/Cargo.toml`

- [ ] **Step 1: Create the branch**
```bash
cd /Users/ruslanelishaev/Desktop/projects/ultimo
git switch -c feat/v0.5.0-static-compression
```

- [ ] **Step 2: Add deps to `ultimo/Cargo.toml`**

In the `[dependencies]` section, after the `sha2` line, add:
```toml
# Static file serving (optional) — pure-Rust MIME detection
mime_guess = { version = "2", optional = true }

# Response compression (optional) — both pure Rust, no C deps
flate2  = { version = "1", optional = true }
brotli  = { version = "7", optional = true }
```

In the `[dev-dependencies]` section, add:
```toml
tempfile = "3"
```

In the `[features]` section, after `api-key`, add:
```toml
# Static file serving + SPA fallback
static-files = ["dep:mime_guess"]

# Response compression (gzip + brotli)
compression = ["dep:flate2", "dep:brotli"]
```

- [ ] **Step 3: Add `spa-demo` to workspace**

In the root `Cargo.toml`, find the `members` array and append `"examples/spa-demo"`:
```toml
members = [
    "ultimo",
    "examples/basic",
    "examples/rpc-modes",
    "examples/openapi-demo",
    "examples/database-sqlx",
    "examples/database-diesel",
    "examples/database-api-styles",
    "examples/websocket-chat",
    "examples/websocket-chat-react",
    "examples/session-auth",
    "examples/jwt-auth",
    "examples/spa-demo",   # ← add this
    "ultimo-cli",
    "coverage-tool",
]
```

- [ ] **Step 4: Verify deps resolve**
```bash
cargo check -p ultimo --features "static-files,compression"
```
Expected: compile error about missing modules/types (fine — we haven't written the code yet), but no "failed to find crate" errors.

- [ ] **Step 5: Commit**
```bash
git add Cargo.toml ultimo/Cargo.toml
git commit -m "chore(deps): add mime_guess, flate2, brotli for static-files + compression features"
```

---

## Task 2: Router wildcard support (TDD)

**Files:** `ultimo/src/router.rs`

The router's `Route` only has `Static` and `Param` segments. A wildcard segment (`*name`) captures all remaining path segments joined by `/`. It must be the last segment of a pattern.

- [ ] **Step 1: Write failing unit tests** — add these inside the existing `#[cfg(test)] mod tests` block at the bottom of `router.rs`:

```rust
#[test]
fn wildcard_matches_single_segment() {
    let route = Route::new("/assets/*path");
    let params = route.matches("/assets/style.css").unwrap();
    assert_eq!(params["path"], "style.css");
}

#[test]
fn wildcard_matches_nested_path() {
    let route = Route::new("/assets/*path");
    let params = route.matches("/assets/css/theme/main.css").unwrap();
    assert_eq!(params["path"], "css/theme/main.css");
}

#[test]
fn wildcard_requires_prefix_match() {
    let route = Route::new("/assets/*path");
    assert!(route.matches("/other/style.css").is_none());
}

#[test]
fn wildcard_does_not_match_prefix_only() {
    // /assets alone has no trailing segment — wildcard requires at least one
    let route = Route::new("/assets/*path");
    assert!(route.matches("/assets").is_none());
}

#[test]
fn wildcard_does_not_affect_static_key() {
    // Routes with wildcards must be in the dynamic (slow-path) scan
    let route = Route::new("/assets/*path");
    assert!(route.static_key().is_none());
}

#[test]
fn wildcard_specificity_equals_static_prefix_count() {
    let route = Route::new("/assets/public/*path");
    // "assets" and "public" are static; wildcard is 0
    assert_eq!(route.specificity(), 2);
}
```

- [ ] **Step 2: Run to verify they fail**
```bash
cargo test -p ultimo --lib wildcard 2>&1 | tail -20
```
Expected: all 6 tests fail with "called `Option::unwrap()` on a `None` value" or "assertion failed".

- [ ] **Step 3: Add `Segment::Wildcard` variant**

In `router.rs`, find the `enum Segment` block and add the new variant:
```rust
#[derive(Debug, Clone, PartialEq)]
enum Segment {
    /// Static path segment
    Static(String),
    /// Dynamic parameter segment (`:name`)
    Param(String),
    /// Catch-all wildcard segment (`*name`) — must be the last segment;
    /// captures all remaining path segments joined by `/`.
    Wildcard(String),
}
```

- [ ] **Step 4: Parse `*name` in `parse_path`**

In `Route::parse_path`, replace the current map closure:
```rust
fn parse_path(path: &str) -> Vec<Segment> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|segment| {
            if let Some(stripped) = segment.strip_prefix('*') {
                Segment::Wildcard(stripped.to_string())
            } else if let Some(stripped) = segment.strip_prefix(':') {
                Segment::Param(stripped.to_string())
            } else {
                Segment::Static(segment.to_string())
            }
        })
        .collect()
}
```

- [ ] **Step 5: Update `matches()` to handle wildcards**

Replace the entire `Route::matches` method:
```rust
pub fn matches(&self, path: &str) -> Option<Params> {
    let path_segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Check whether the last segment is a wildcard
    let has_wildcard = matches!(self.segments.last(), Some(Segment::Wildcard(_)));

    if has_wildcard {
        let prefix = &self.segments[..self.segments.len() - 1];
        // Need at least len(prefix)+1 path segments so the wildcard captures something
        if path_segs.len() <= prefix.len() {
            return None;
        }
        let mut params = HashMap::new();
        for (route_seg, path_seg) in prefix.iter().zip(path_segs.iter()) {
            match route_seg {
                Segment::Static(expected) => {
                    if expected.as_str() != *path_seg {
                        return None;
                    }
                }
                Segment::Param(name) => {
                    params.insert(name.clone(), (*path_seg).to_string());
                }
                Segment::Wildcard(_) => unreachable!("wildcard must be the last segment"),
            }
        }
        // Capture the remainder as a single slash-joined string
        if let Some(Segment::Wildcard(name)) = self.segments.last() {
            let rest = path_segs[prefix.len()..].join("/");
            params.insert(name.clone(), rest);
        }
        return Some(params);
    }

    // Original logic: exact segment count required
    if path_segs.len() != self.segments.len() {
        return None;
    }
    let mut params = HashMap::new();
    for (route_seg, path_seg) in self.segments.iter().zip(path_segs.iter()) {
        match route_seg {
            Segment::Static(expected) => {
                if expected.as_str() != *path_seg {
                    return None;
                }
            }
            Segment::Param(name) => {
                params.insert(name.clone(), (*path_seg).to_string());
            }
            Segment::Wildcard(_) => unreachable!("wildcard only at end"),
        }
    }
    Some(params)
}
```

- [ ] **Step 6: Update `specificity()` to count wildcards as 0**

The existing implementation filters for `Segment::Static(_)` — wildcard segments aren't `Static` so they already count as 0. No change needed. Verify by reading the method:
```rust
fn specificity(&self) -> usize {
    self.segments
        .iter()
        .filter(|s| matches!(s, Segment::Static(_)))
        .count()
}
```
✓ Already correct.

- [ ] **Step 7: Verify `static_key()` returns None for wildcards**

The existing `static_key()` returns `None` on the first non-`Static` segment. `Wildcard` is not `Static`, so it already causes `None`. No change needed.

- [ ] **Step 8: Run the wildcard tests — they should pass now**
```bash
cargo test -p ultimo --lib wildcard
```
Expected: all 6 pass.

- [ ] **Step 9: Run all lib tests to check for regressions**
```bash
cargo test -p ultimo --lib
```
Expected: all pass.

- [ ] **Step 10: Commit**
```bash
git add ultimo/src/router.rs
git commit -m "feat(router): add wildcard segment support (*name catches remaining path)"
```

---

## Task 3: Static file core (`serve_file`)

**Files:** Create `ultimo/src/static_files.rs`, modify `ultimo/src/lib.rs`

- [ ] **Step 1: Create the module file** at `ultimo/src/static_files.rs`:

```rust
//! Static file serving utilities.
//!
//! Enabled by the `static-files` Cargo feature.

use crate::{error::UltimoError, response::Response};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{header, StatusCode};
use std::path::Path;

/// Serve a single file from `root / rel_path`.
///
/// - Detects MIME type from the file extension via `mime_guess`.
/// - Sets `ETag: "{size}-{mtime_secs}"`.
/// - Returns 304 Not Modified if `if_none_match` matches the computed ETag.
/// - Returns 404 (as `Err(UltimoError::NotFound)`) if the file is missing,
///   is a directory, or if `rel_path` would escape `root` (path traversal).
pub(crate) async fn serve_file(
    root: &Path,
    rel_path: &str,
    if_none_match: Option<String>,
) -> crate::error::Result<Response> {
    use std::time::UNIX_EPOCH;

    // Canonicalize the root so we have an absolute, symlink-resolved base
    let canonical_root = tokio::fs::canonicalize(root)
        .await
        .map_err(|_| UltimoError::NotFound("static root not found".into()))?;

    // Strip any leading slashes or `./` from the caller-supplied relative path
    let rel_clean = rel_path.trim_start_matches('/').trim_start_matches("./");

    // Build candidate absolute path
    let candidate = canonical_root.join(rel_clean);

    // Canonicalize the candidate — this resolves `..` and symlinks.
    // If the file doesn't exist, `canonicalize` returns an error → 404.
    let resolved = tokio::fs::canonicalize(&candidate)
        .await
        .map_err(|_| UltimoError::NotFound("file not found".into()))?;

    // Path traversal guard: resolved path must remain under root
    if !resolved.starts_with(&canonical_root) {
        return Err(UltimoError::NotFound("file not found".into()));
    }

    // Stat the resolved path and require it to be a regular file
    let metadata = tokio::fs::metadata(&resolved)
        .await
        .map_err(|_| UltimoError::NotFound("file not found".into()))?;

    if !metadata.is_file() {
        return Err(UltimoError::NotFound("file not found".into()));
    }

    // Compute ETag: "{file_size}-{mtime_as_unix_seconds}"
    let mtime_secs = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let etag = format!("\"{}-{}\"", metadata.len(), mtime_secs);

    // Conditional GET: 304 if the client's cached ETag matches
    if let Some(ref inm) = if_none_match {
        if inm.trim() == etag.as_str() {
            return Ok(hyper::Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Full::new(Bytes::new()))
                .unwrap());
        }
    }

    // Read file contents
    let content = tokio::fs::read(&resolved)
        .await
        .map_err(|_| UltimoError::NotFound("file not found".into()))?;

    // MIME type from extension, defaulting to application/octet-stream
    let mime = mime_guess::from_path(&resolved)
        .first_or_octet_stream()
        .to_string();

    Ok(hyper::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ETAG, etag)
        .header(header::CONTENT_LENGTH, content.len())
        .body(Full::new(Bytes::from(content)))
        .unwrap())
}
```

- [ ] **Step 2: Expose the module in `ultimo/src/lib.rs`**

After the `#[cfg(feature = "auth")] pub mod auth;` block, add:
```rust
#[cfg(feature = "static-files")]
pub(crate) mod static_files;
```

- [ ] **Step 3: Verify it compiles**
```bash
cargo check -p ultimo --features "static-files"
```
Expected: no errors.

- [ ] **Step 4: Commit**
```bash
git add ultimo/src/static_files.rs ultimo/src/lib.rs
git commit -m "feat(static-files): add serve_file core (ETag, MIME, path traversal guard)"
```

---

## Task 4: `serve_static` on `Ultimo` (TDD)

**Files:** `ultimo/src/app.rs`, `ultimo/tests/static_files.rs`

- [ ] **Step 1: Create the test file** at `ultimo/tests/static_files.rs` with the first two tests:

```rust
//! Integration tests for static file serving.
//! Run with: cargo test -p ultimo --features "static-files" --test static_files

#![cfg(feature = "static-files")]

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request as HyperRequest;
use tempfile::TempDir;
use ultimo::prelude::*;

fn empty() -> Full<Bytes> {
    Full::new(Bytes::new())
}

/// Write a file into `dir` and return its path.
async fn write_fixture(dir: &TempDir, name: &str, content: &[u8]) {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.unwrap();
    }
    tokio::fs::write(&path, content).await.unwrap();
}

#[tokio::test]
async fn existing_file_returns_200_with_correct_mime() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "hello.txt", b"hello world").await;

    let mut app = Ultimo::new_without_defaults();
    app.serve_static("/assets", dir.path());

    let req = HyperRequest::builder()
        .uri("/assets/hello.txt")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;

    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("text/plain")
    );
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), b"hello world");
}

#[tokio::test]
async fn missing_file_returns_404() {
    let dir = TempDir::new().unwrap();

    let mut app = Ultimo::new_without_defaults();
    app.serve_static("/assets", dir.path());

    let req = HyperRequest::builder()
        .uri("/assets/nope.txt")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 404);
}
```

- [ ] **Step 2: Run to verify tests fail**
```bash
cargo test -p ultimo --features "static-files" --test static_files 2>&1 | tail -10
```
Expected: compile error — `serve_static` method doesn't exist yet.

- [ ] **Step 3: Add `spa_fallback` field to `Ultimo` struct in `app.rs`**

Find the `pub struct Ultimo {` block and add the new field after `trust_proxy`:
```rust
#[cfg(feature = "static-files")]
spa_fallback: Option<(std::path::PathBuf, String)>,
```

- [ ] **Step 4: Initialize `spa_fallback` in `Ultimo::new()` and `Ultimo::new_without_defaults()`**

In both `new()` and `new_without_defaults()`, add the field to the struct literal:
```rust
#[cfg(feature = "static-files")]
spa_fallback: None,
```

- [ ] **Step 5: Add `serve_static` method to `Ultimo` in `app.rs`**

After the `use_middleware` method, add:
```rust
/// Serve static files from `dir` under the URL prefix `prefix`.
///
/// Registers a `GET {prefix}/*path` route. Responds with the correct
/// `Content-Type`, sets an `ETag`, and handles `If-None-Match` → 304.
/// Path traversal attempts return 404.
///
/// ```rust,no_run
/// use ultimo::prelude::*;
///
/// let mut app = Ultimo::new();
/// app.serve_static("/assets", "./public");
/// ```
#[cfg(feature = "static-files")]
pub fn serve_static(
    &mut self,
    prefix: &str,
    dir: impl Into<std::path::PathBuf>,
) -> &mut Self {
    let root = dir.into();
    let pattern = format!("{}/*path", prefix.trim_end_matches('/'));
    self.get(&pattern, move |ctx: Context| {
        let root = root.clone();
        async move {
            let rel = ctx.req.param("path")?.to_string();
            let inm = ctx.req.header("if-none-match");
            crate::static_files::serve_file(&root, &rel, inm).await
        }
    });
    self
}
```

- [ ] **Step 6: Run the two tests — they should pass**
```bash
cargo test -p ultimo --features "static-files" --test static_files existing_file missing_file
```
Expected: both pass.

- [ ] **Step 7: Commit**
```bash
git add ultimo/src/app.rs ultimo/tests/static_files.rs
git commit -m "feat(static-files): add serve_static method to Ultimo"
```

---

## Task 5: `serve_spa` + remaining static tests (TDD)

**Files:** `ultimo/src/app.rs`, `ultimo/tests/static_files.rs`

- [ ] **Step 1: Add remaining tests to `ultimo/tests/static_files.rs`**

Append to the existing test file:
```rust
#[tokio::test]
async fn path_traversal_is_blocked() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "secret.txt", b"secret").await;

    let mut app = Ultimo::new_without_defaults();
    app.serve_static("/assets", dir.path());

    // The URL-decoded path would be ../../secret.txt — must be blocked
    let req = HyperRequest::builder()
        .uri("/assets/..%2F..%2Fsecret.txt")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn etag_is_set_and_304_on_match() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "file.txt", b"content").await;

    let mut app = Ultimo::new_without_defaults();
    app.serve_static("/assets", dir.path());

    // First request — grab ETag
    let req = HyperRequest::builder()
        .uri("/assets/file.txt")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    let etag = res
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .expect("ETag header missing")
        .to_string();

    // Rebuild app (oneshot consumes it)
    let dir2 = dir; // keep TempDir alive
    let mut app2 = Ultimo::new_without_defaults();
    app2.serve_static("/assets", dir2.path());

    // Second request with If-None-Match → 304
    let req2 = HyperRequest::builder()
        .uri("/assets/file.txt")
        .header("if-none-match", &etag)
        .body(empty())
        .unwrap();
    let res2 = app2.oneshot(req2).await;
    assert_eq!(res2.status(), 304);
}

#[tokio::test]
async fn nested_path_is_served() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "css/main.css", b"body { color: red; }").await;

    let mut app = Ultimo::new_without_defaults();
    app.serve_static("/assets", dir.path());

    let req = HyperRequest::builder()
        .uri("/assets/css/main.css")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("text/css")
    );
}

#[tokio::test]
async fn spa_fallback_serves_index_for_unknown_routes() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "index.html", b"<!DOCTYPE html><html></html>").await;

    let mut app = Ultimo::new_without_defaults();
    app.get("/api/hello", |ctx: Context| async move {
        ctx.json(serde_json::json!({ "ok": true })).await
    });
    app.serve_spa(dir.path(), "index.html");

    let req = HyperRequest::builder()
        .uri("/unknown-route")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(body.starts_with(b"<!DOCTYPE html>"));
}

#[tokio::test]
async fn spa_fallback_does_not_intercept_api_404() {
    // POST /unknown → 404, NOT redirected to index.html (only GET is caught)
    let dir = TempDir::new().unwrap();
    write_fixture(&dir, "index.html", b"<!DOCTYPE html>").await;

    let mut app = Ultimo::new_without_defaults();
    app.serve_spa(dir.path(), "index.html");

    let req = HyperRequest::builder()
        .method("POST")
        .uri("/unknown")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 404);
}
```

- [ ] **Step 2: Run to verify new tests fail**
```bash
cargo test -p ultimo --features "static-files" --test static_files spa 2>&1 | tail -10
```
Expected: compile error — `serve_spa` doesn't exist yet.

- [ ] **Step 3: Add `serve_spa` method to `app.rs`**

After `serve_static`, add:
```rust
/// Serve a SPA (Single Page Application) from `dir`.
///
/// Any `GET` request that returns 404 (no matching route) is answered with
/// `dir/fallback` instead — enabling client-side routing.
///
/// Mount API routes **before** calling `serve_spa` so they take precedence.
///
/// ```rust,no_run
/// use ultimo::prelude::*;
///
/// let mut app = Ultimo::new();
/// app.get("/api/hello", |ctx: Context| async move {
///     ctx.json(serde_json::json!({ "ok": true })).await
/// });
/// app.serve_spa("./dist", "index.html");
/// ```
#[cfg(feature = "static-files")]
pub fn serve_spa(
    &mut self,
    dir: impl Into<std::path::PathBuf>,
    fallback: &str,
) -> &mut Self {
    self.spa_fallback = Some((dir.into(), fallback.to_string()));
    self
}
```

- [ ] **Step 4: Add SPA fallback check to `dispatch_parts` in `app.rs`**

At the end of `dispatch_parts`, just before the final `return response;` statement, add:

```rust
// SPA fallback: if the response is 404 and this is a GET request,
// and a SPA root is configured, serve the fallback file instead.
#[cfg(feature = "static-files")]
if response.status() == hyper::StatusCode::NOT_FOUND
    && parts.method == hyper::Method::GET
{
    if let Some((ref spa_dir, ref spa_file)) = self.spa_fallback {
        if let Ok(fallback_resp) =
            crate::static_files::serve_file(spa_dir, spa_file, None).await
        {
            return fallback_resp;
        }
    }
}
```

> **Note:** you need to find the final `response` return in `dispatch_parts`. Look for the last `response` or equivalent — it may be a variable returned at the very end of the function. The snippet above inserts just before that final return.

- [ ] **Step 5: Run all static file tests**
```bash
cargo test -p ultimo --features "static-files" --test static_files
```
Expected: all 7 pass.

- [ ] **Step 6: Run lib tests to confirm no regressions**
```bash
cargo test -p ultimo --lib
```
Expected: all pass.

- [ ] **Step 7: Commit**
```bash
git add ultimo/src/app.rs ultimo/tests/static_files.rs
git commit -m "feat(static-files): add serve_spa + SPA fallback in dispatch_parts"
```

---

## Task 6: Compression middleware (TDD)

**Files:** `ultimo/src/middleware.rs`, `ultimo/tests/compression.rs`

- [ ] **Step 1: Create `ultimo/tests/compression.rs`** with all 8 tests:

```rust
//! Integration tests for response compression middleware.
//! Run with: cargo test -p ultimo --features "compression" --test compression

#![cfg(feature = "compression")]

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request as HyperRequest;
use ultimo::middleware::builtin::{compression, Compression};
use ultimo::prelude::*;

fn empty() -> Full<Bytes> {
    Full::new(Bytes::new())
}

/// A body large enough to exceed the default 1024-byte min_size floor.
fn large_text() -> String {
    "Hello, compressed world! ".repeat(100)
}

fn app_with_text_route() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/text", |ctx: Context| async move {
        ctx.json(serde_json::json!({ "data": large_text() })).await
    });
    app
}

#[tokio::test]
async fn gzip_accept_returns_gzip_encoding() {
    let app = app_with_text_route();
    let req = HyperRequest::builder()
        .uri("/text")
        .header("accept-encoding", "gzip")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers().get("content-encoding").and_then(|v| v.to_str().ok()),
        Some("gzip")
    );
}

#[tokio::test]
async fn brotli_accept_returns_brotli_encoding() {
    let app = app_with_text_route();
    let req = HyperRequest::builder()
        .uri("/text")
        .header("accept-encoding", "br")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers().get("content-encoding").and_then(|v| v.to_str().ok()),
        Some("br")
    );
}

#[tokio::test]
async fn brotli_preferred_over_gzip_when_both_listed() {
    let app = app_with_text_route();
    let req = HyperRequest::builder()
        .uri("/text")
        .header("accept-encoding", "gzip, br")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(
        res.headers().get("content-encoding").and_then(|v| v.to_str().ok()),
        Some("br")
    );
}

#[tokio::test]
async fn no_accept_encoding_no_compression() {
    let app = app_with_text_route();
    let req = HyperRequest::builder()
        .uri("/text")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    assert!(res.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn image_content_type_not_compressed() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/img", |_ctx: Context| async move {
        Ok(hyper::Response::builder()
            .status(200)
            .header("content-type", "image/png")
            .body(Full::new(Bytes::from(vec![0u8; 2048])))
            .unwrap())
    });

    let req = HyperRequest::builder()
        .uri("/img")
        .header("accept-encoding", "gzip")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);
    assert!(res.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn small_body_not_compressed() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/tiny", |ctx: Context| async move {
        ctx.json(serde_json::json!({ "ok": true })).await  // < 1024 bytes
    });

    let req = HyperRequest::builder()
        .uri("/tiny")
        .header("accept-encoding", "gzip")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert!(res.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn vary_header_always_set() {
    let app = app_with_text_route();
    let req = HyperRequest::builder()
        .uri("/text")
        .body(empty())  // no Accept-Encoding
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(
        res.headers().get("vary").and_then(|v| v.to_str().ok()),
        Some("Accept-Encoding")
    );
}

#[tokio::test]
async fn already_encoded_response_not_double_compressed() {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(compression());
    app.get("/pre-encoded", |_ctx: Context| async move {
        Ok(hyper::Response::builder()
            .status(200)
            .header("content-type", "text/plain")
            .header("content-encoding", "gzip")  // already encoded
            .body(Full::new(Bytes::from(vec![0u8; 2048])))
            .unwrap())
    });

    let req = HyperRequest::builder()
        .uri("/pre-encoded")
        .header("accept-encoding", "gzip")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    // Must not double-compress — only one content-encoding value
    let ce = res.headers().get("content-encoding").and_then(|v| v.to_str().ok());
    assert_eq!(ce, Some("gzip"));
}

#[tokio::test]
async fn gzip_body_is_decodable() {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let app = app_with_text_route();
    let req = HyperRequest::builder()
        .uri("/text")
        .header("accept-encoding", "gzip")
        .body(empty())
        .unwrap();
    let res = app.oneshot(req).await;
    assert_eq!(res.status(), 200);

    let compressed = res.into_body().collect().await.unwrap().to_bytes();
    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut decoded = String::new();
    decoder.read_to_string(&mut decoded).expect("gzip decode failed");
    let json: serde_json::Value = serde_json::from_str(&decoded).unwrap();
    assert!(json["data"].as_str().unwrap().starts_with("Hello"));
}
```

- [ ] **Step 2: Run to verify tests fail to compile**
```bash
cargo test -p ultimo --features "compression" --test compression 2>&1 | tail -10
```
Expected: compile error — `compression`, `Compression` not found in `middleware::builtin`.

- [ ] **Step 3: Implement `Compression` in `ultimo/src/middleware.rs`**

In the `pub mod builtin` block, after the `security_headers()` function, add (still inside `builtin`):

```rust
/// Response compression middleware (gzip + brotli).
///
/// Negotiates the best encoding from the request's `Accept-Encoding` header.
/// Brotli is preferred over gzip when both are accepted.
///
/// Skips compression when:
/// - The response body is smaller than `min_size` bytes (default: 1024).
/// - The `Content-Type` is a binary format (images, audio, video, zip, …).
/// - The response already carries a `Content-Encoding` header.
///
/// Always sets `Vary: Accept-Encoding` on every response (required by RFC 7231
/// so caches know the response varies by encoding).
///
/// ```
/// # use ultimo::Ultimo;
/// let mut app = Ultimo::new_without_defaults();
/// app.use_middleware(ultimo::middleware::builtin::compression());
/// // or configured:
/// app.use_middleware(
///     ultimo::middleware::builtin::Compression::new()
///         .gzip()
///         .brotli()
///         .min_size(512)
///         .build(),
/// );
/// ```
#[cfg(feature = "compression")]
#[derive(Debug, Clone)]
pub struct Compression {
    gzip: bool,
    brotli: bool,
    min_size: usize,
}

#[cfg(feature = "compression")]
impl Default for Compression {
    fn default() -> Self {
        Self {
            gzip: true,
            brotli: true,
            min_size: 1024,
        }
    }
}

#[cfg(feature = "compression")]
impl Compression {
    /// Create with defaults (gzip + brotli enabled, min_size = 1024).
    pub fn new() -> Self {
        Self::default()
    }
    /// Enable gzip compression.
    pub fn gzip(mut self) -> Self {
        self.gzip = true;
        self
    }
    /// Enable brotli compression.
    pub fn brotli(mut self) -> Self {
        self.brotli = true;
        self
    }
    /// Minimum response body size in bytes to compress (default: 1024).
    /// Responses smaller than this are passed through unmodified.
    pub fn min_size(mut self, bytes: usize) -> Self {
        self.min_size = bytes;
        self
    }

    /// Build the middleware.
    pub fn build(self) -> BoxedMiddleware {
        use brotli::CompressorWriter;
        use flate2::{write::GzEncoder, Compression as GzLevel};
        use hyper::header::{CONTENT_ENCODING, CONTENT_LENGTH, VARY};
        use std::io::Write;

        let gzip_enabled = self.gzip;
        let brotli_enabled = self.brotli;
        let min_size = self.min_size;

        Arc::new(move |ctx, next| {
            Box::pin(async move {
                // Capture Accept-Encoding BEFORE consuming ctx with next()
                let accept_enc = ctx
                    .req
                    .header("accept-encoding")
                    .unwrap_or_default()
                    .to_lowercase();

                let mut res = next(ctx).await?;

                // Always set Vary (RFC 7231 §7.1.4 — response varies by encoding)
                res.headers_mut().insert(
                    VARY,
                    hyper::header::HeaderValue::from_static("Accept-Encoding"),
                );

                // Skip if already encoded
                if res.headers().contains_key(CONTENT_ENCODING) {
                    return Ok(res);
                }

                // Collect the body bytes (already buffered Full<Bytes>)
                let (parts, body) = res.into_parts();
                let body_bytes = body.collect().await?.to_bytes();

                // Skip below min_size
                if body_bytes.len() < min_size {
                    return Ok(hyper::Response::from_parts(
                        parts,
                        Full::new(body_bytes),
                    ));
                }

                // Skip binary content types
                let ct = parts
                    .headers
                    .get(hyper::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_lowercase();

                const SKIP_PREFIXES: &[&str] =
                    &["image/", "audio/", "video/", "font/woff"];
                const SKIP_EXACT: &[&str] = &[
                    "application/zip",
                    "application/gzip",
                    "application/x-gzip",
                    "application/octet-stream",
                ];
                let skip = SKIP_PREFIXES.iter().any(|p| ct.starts_with(p))
                    || SKIP_EXACT.iter().any(|e| ct.starts_with(e));

                if skip {
                    return Ok(hyper::Response::from_parts(
                        parts,
                        Full::new(body_bytes),
                    ));
                }

                // Choose algorithm: prefer brotli > gzip > none
                let use_brotli = brotli_enabled && accept_enc.contains("br");
                let use_gzip =
                    !use_brotli && gzip_enabled && accept_enc.contains("gzip");

                if use_brotli {
                    let mut compressed = Vec::new();
                    {
                        let mut writer =
                            CompressorWriter::new(&mut compressed, 4096, 5, 22);
                        writer.write_all(&body_bytes).unwrap();
                    }
                    let len = compressed.len();
                    let mut res =
                        hyper::Response::from_parts(parts, Full::new(Bytes::from(compressed)));
                    res.headers_mut().insert(
                        CONTENT_ENCODING,
                        hyper::header::HeaderValue::from_static("br"),
                    );
                    res.headers_mut().insert(
                        CONTENT_LENGTH,
                        hyper::header::HeaderValue::from_str(&len.to_string()).unwrap(),
                    );
                    Ok(res)
                } else if use_gzip {
                    let mut compressed = Vec::new();
                    {
                        let mut encoder =
                            GzEncoder::new(&mut compressed, GzLevel::default());
                        encoder.write_all(&body_bytes).unwrap();
                        encoder.finish().unwrap();
                    }
                    let len = compressed.len();
                    let mut res =
                        hyper::Response::from_parts(parts, Full::new(Bytes::from(compressed)));
                    res.headers_mut().insert(
                        CONTENT_ENCODING,
                        hyper::header::HeaderValue::from_static("gzip"),
                    );
                    res.headers_mut().insert(
                        CONTENT_LENGTH,
                        hyper::header::HeaderValue::from_str(&len.to_string()).unwrap(),
                    );
                    Ok(res)
                } else {
                    // No matching encoding — pass through unmodified
                    Ok(hyper::Response::from_parts(parts, Full::new(body_bytes)))
                }
            })
        })
    }
}

/// Compression middleware with defaults (gzip + brotli, min 1 KB).
///
/// Convenience alias for `Compression::new().build()`.
///
/// ```
/// # use ultimo::Ultimo;
/// let mut app = Ultimo::new_without_defaults();
/// app.use_middleware(ultimo::middleware::builtin::compression());
/// ```
#[cfg(feature = "compression")]
pub fn compression() -> BoxedMiddleware {
    Compression::new().build()
}
```

> **Import note:** The `Compression::build` closure uses `brotli` and `flate2` types. These are `dep:brotli` and `dep:flate2` in Cargo.toml (available when `compression` feature is on). The `Arc`, `BoxedMiddleware`, `Context`, `Next` types are already in scope from the outer `builtin` module's parent.

- [ ] **Step 4: Run compression tests**
```bash
cargo test -p ultimo --features "compression" --test compression
```
Expected: all 9 pass.

- [ ] **Step 5: Run all lib tests**
```bash
cargo test -p ultimo --lib
```
Expected: all pass.

- [ ] **Step 6: Commit**
```bash
git add ultimo/src/middleware.rs ultimo/tests/compression.rs
git commit -m "feat(compression): add gzip + brotli response compression middleware"
```

---

## Task 7: `spa-demo` example

**Files:** `examples/spa-demo/`

- [ ] **Step 1: Create `examples/spa-demo/Cargo.toml`**

```toml
[package]
name = "spa-demo"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "spa-demo"
path = "src/main.rs"

[dependencies]
ultimo = { path = "../../ultimo", features = ["static-files", "compression"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 2: Create `examples/spa-demo/dist/index.html`**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Ultimo SPA Demo</title>
</head>
<body>
  <div id="app"></div>
  <script src="/app.js"></script>
</body>
</html>
```

- [ ] **Step 3: Create `examples/spa-demo/dist/app.js`**

```javascript
// Minimal SPA: fetch /api/hello and display the result
fetch("/api/hello")
  .then((r) => r.json())
  .then((data) => {
    document.getElementById("app").textContent = data.message;
  });
```

- [ ] **Step 4: Create `examples/spa-demo/src/main.rs`**

```rust
//! SPA Demo — serves a Vite-style SPA from `./dist` with gzip/brotli compression.
//!
//! Run with:  cargo run -p spa-demo
//! Then open: http://127.0.0.1:3000
//!
//! All routes under /assets are served as static files from `./dist`.
//! Every other GET that has no API route falls back to `./dist/index.html`
//! so the SPA's client-side router can handle it.

use ultimo::middleware::builtin::{compression, logger};
use ultimo::prelude::*;

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    tracing_subscriber::fmt::init();

    let mut app = Ultimo::new();

    // Global middleware: logger then compression
    app.use_middleware(logger());
    app.use_middleware(compression());

    // API route — served before the SPA fallback
    app.get("/api/hello", |ctx: Context| async move {
        ctx.json(serde_json::json!({ "message": "Hello from Ultimo!" })).await
    });

    // Serve static assets (CSS, JS, images) from ./dist under /assets
    app.serve_static("/assets", "./dist");

    // SPA fallback: all other GET routes → index.html
    app.serve_spa("./dist", "index.html");

    println!("→  http://127.0.0.1:3000");
    app.listen("127.0.0.1:3000").await
}
```

- [ ] **Step 5: Verify the example compiles**
```bash
cargo check -p spa-demo
```
Expected: no errors.

- [ ] **Step 6: Commit**
```bash
git add examples/spa-demo/
git commit -m "feat(examples): add spa-demo (static serving + SPA fallback + compression)"
```

---

## Task 8: Documentation

**Files:** docs-site pages, vocs config, README, roadmap

- [ ] **Step 1: Create `docs-site/docs/pages/static-files.mdx`**

```mdx
# Static Files

Serve frontend assets — CSS, JavaScript, images — directly from disk, and
support Single Page Application (SPA) routing with a fallback to `index.html`.

Requires the `static-files` Cargo feature:

```toml
ultimo = { version = "0.5", features = ["static-files"] }
```

## Serving a directory

`serve_static(prefix, dir)` registers a `GET {prefix}/*` route that reads
files from `dir` on disk.

```rust
use ultimo::prelude::*;

let mut app = Ultimo::new();

// GET /assets/style.css → reads ./public/style.css
app.serve_static("/assets", "./public");

app.listen("127.0.0.1:3000").await
```

**Response headers set automatically:**
- `Content-Type` — detected from the file extension.
- `ETag` — `"{size}-{mtime_secs}"`, used for conditional GET.
- `Content-Length`.

**Conditional GET:** If the client sends `If-None-Match` matching the current
ETag, the server returns `304 Not Modified` with an empty body, saving
bandwidth.

## SPA fallback

For Single Page Applications where the client-side router handles URLs (e.g.
React Router, Vue Router), every `GET` request that doesn't match an API route
should return `index.html`. Use `serve_spa`:

```rust
use ultimo::prelude::*;

let mut app = Ultimo::new();

// API routes first — they take precedence
app.get("/api/user", |ctx: Context| async move {
    ctx.json(serde_json::json!({ "name": "Ada" })).await
});

// Serve static assets explicitly
app.serve_static("/assets", "./dist/assets");

// Fallback: any unmatched GET → ./dist/index.html
app.serve_spa("./dist", "index.html");

app.listen("127.0.0.1:3000").await
```

> `serve_spa` only intercepts `GET` requests that returned 404.
> `POST`, `PUT`, `DELETE`, etc. 404s are passed through unchanged.

## Security

Path traversal is prevented at the file-system level: the resolved path is
canonicalized and must remain inside the configured root directory. Requests
like `GET /assets/../../etc/passwd` return 404.

Directory listing is not supported — requesting a path that resolves to a
directory returns 404.

## Pairing with compression

Mount the `compression()` middleware before serving static files to enable
gzip/brotli on text assets:

```rust
use ultimo::middleware::builtin::compression;
use ultimo::prelude::*;

let mut app = Ultimo::new();
app.use_middleware(compression());
app.serve_static("/assets", "./dist/assets");
app.serve_spa("./dist", "index.html");
```

See [Compression](/middleware#compression) for details.
```

- [ ] **Step 2: Add Compression section to `docs-site/docs/pages/middleware.mdx`**

After the "### Security headers" section and before "## Writing custom middleware", insert:

```mdx
### Compression

Requires the `compression` Cargo feature:

```toml
ultimo = { version = "0.5", features = ["compression"] }
```

Compress response bodies with gzip or brotli, negotiated from the client's
`Accept-Encoding` header. Brotli is preferred when both are accepted.

```rust
use ultimo::middleware::builtin::compression;

app.use_middleware(compression());
```

Configured, with the `Compression` builder:

```rust
use ultimo::middleware::builtin::Compression;

app.use_middleware(
    Compression::new()
        .gzip()
        .brotli()
        .min_size(512) // skip bodies smaller than 512 bytes
        .build(),
);
```

Responses are **not** compressed when:
- The body is smaller than `min_size` bytes (default: 1024).
- The `Content-Type` is a binary format: `image/*`, `audio/*`, `video/*`,
  `application/zip`, `application/octet-stream`, etc.
- The response already has a `Content-Encoding` header.

`Vary: Accept-Encoding` is always added (required by RFC 7231 so HTTP caches
serve the correct version to each client).
```

- [ ] **Step 3: Add nav entries to `docs-site/vocs.config.ts`**

Find the sidebar items array. Add "Static Files" under the Introduction group (after "Performance"):
```ts
{ text: 'Static Files', link: '/static-files' },
```

Add a "Compression" entry under the Middleware group or after the middleware item:
```ts
{ text: 'Compression', link: '/middleware#compression' },
```
(Adjust placement to match the existing sidebar structure.)

- [ ] **Step 4: Add new items to `docs-site/docs/pages/api-reference.mdx`**

Find the middleware section and add:

```mdx
### `compression()` / `Compression` *(feature: `compression`)*

```rust
// Convenience — gzip + brotli, min 1 KB
app.use_middleware(compression());

// Configured
app.use_middleware(
    Compression::new()
        .gzip()
        .brotli()
        .min_size(512)
        .build(),
);
```

Find the `Ultimo` methods section and add:

```mdx
### `serve_static(prefix, dir)` *(feature: `static-files`)*

```rust
app.serve_static("/assets", "./public");
```

Registers a `GET {prefix}/*` route that reads files from `dir`.

### `serve_spa(dir, fallback)` *(feature: `static-files`)*

```rust
app.serve_spa("./dist", "index.html");
```

Serves `dir/fallback` for any `GET` request that would otherwise return 404.
```

- [ ] **Step 5: Update `README.md` feature flags table**

Find the feature flags table and update the `static-files` and `compression` rows (they currently say "Planned" or may be missing). Replace or add:

```markdown
| `static-files` | Static file serving + SPA fallback |
| `compression` | Response compression (gzip + brotli) |
```

- [ ] **Step 6: Update `docs-site/docs/pages/roadmap.mdx`**

In the Feature Status table, find the rows for Static Files and Compression (currently `📋 Planned`) and update to:

```markdown
| Static Files              | ✅ Available     | 0.5.0   |
| Compression               | ✅ Available     | 0.5.0   |
```

Also add them to the v0.5.0 shipped section in the Version Timeline.

- [ ] **Step 7: Commit all docs**
```bash
git add docs-site/ README.md
git commit -m "docs: add static-files and compression pages, update roadmap + API reference"
```

---

## Task 9: CI update + verification gate

**Files:** `.github/workflows/ci.yml`

- [ ] **Step 1: Add static-files + compression to CI test runs**

In `.github/workflows/ci.yml`, find the job that runs library tests (the `cargo test -p ultimo --lib` step) and the integration test steps. Add parallel test runs for the new features:

```yaml
- name: Test static-files feature
  run: cargo test -p ultimo --features "static-files" --test static_files

- name: Test compression feature
  run: cargo test -p ultimo --features "compression" --test compression
```

Also add both features to the clippy step's feature list:
```yaml
cargo clippy -p ultimo --features "websocket,test-helpers,testing,session,csrf,jwt,api-key,static-files,compression" --all-targets -- -D warnings
```

- [ ] **Step 2: Run the full local verification gate**
```bash
cd /Users/ruslanelishaev/Desktop/projects/ultimo

cargo fmt --all --check

cargo clippy -p ultimo \
  --features "websocket,test-helpers,testing,session,csrf,jwt,api-key,static-files,compression" \
  --all-targets -- -D warnings

cargo test -p ultimo --lib

cargo test -p ultimo --features "websocket,test-helpers,testing,session,csrf"

cargo test -p ultimo --features "static-files" --test static_files

cargo test -p ultimo --features "compression" --test compression

cargo test -p ultimo --doc --features "websocket,testing,session,csrf,static-files,compression"

cargo check -p spa-demo
```

Expected: all green.

- [ ] **Step 3: Commit CI changes**
```bash
git add .github/workflows/ci.yml
git commit -m "ci: add static-files and compression feature test runs"
```

---

## Task 10: PR + merge

- [ ] **Step 1: Push the branch**
```bash
git push -u origin feat/v0.5.0-static-compression
```

- [ ] **Step 2: Open the PR**
```bash
gh pr create \
  --title "feat(v0.5.0): static file serving + SPA fallback + response compression" \
  --body "$(cat <<'EOF'
## Summary

- **`static-files` feature**: `app.serve_static(prefix, dir)` serves files from disk with ETag caching (304 support), MIME detection, and path traversal prevention. `app.serve_spa(dir, fallback)` catches unmatched GET routes and serves `index.html` for SPA client-side routing.
- **`compression` feature**: `Compression` middleware / `compression()` convenience fn compresses response bodies with gzip or brotli (negotiated via `Accept-Encoding`), skips binary types and bodies below `min_size`.
- **Router**: added `Segment::Wildcard` (`*name`) to enable prefix-matched catch-all routes.
- **Example**: `examples/spa-demo` — minimal full-stack demo with both features.
- **Docs**: new `/static-files` page, compression section in middleware docs, API reference, roadmap updated to 0.5.0.
- Both features are pure-Rust, `default = []` opt-in, zero breaking changes.

## Test plan

- [ ] `cargo test -p ultimo --features "static-files" --test static_files` — 7 tests green
- [ ] `cargo test -p ultimo --features "compression" --test compression` — 9 tests green
- [ ] `cargo test -p ultimo --lib` — no regressions
- [ ] `cargo check -p spa-demo` — example compiles
- [ ] All 17+ CI checks green

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 3: Watch CI**
```bash
gh pr checks --watch
```
Expected: all checks green. Fix forward if anything fails.

- [ ] **Step 4: Merge**
```bash
gh pr merge --squash --admin --delete-branch
```

- [ ] **Step 5: Sync main**
```bash
git switch main && git pull
git branch -D feat/v0.5.0-static-compression 2>/dev/null || true
```

---

## Self-Review

**Spec coverage check:**

| Spec requirement | Task covering it |
|---|---|
| `static-files` feature flag + `mime_guess` dep | Task 1 |
| `serve_static(prefix, dir)` API | Task 4 |
| `serve_spa(dir, fallback)` API | Task 5 |
| Wildcard route support for multi-segment paths | Task 2 |
| `serve_file` core: ETag, MIME, path traversal | Task 3 |
| 304 on If-None-Match match | Task 3 + Task 5 (test) |
| Path traversal guard → 404 | Task 3 + Task 5 (test) |
| SPA only intercepts GET | Task 5 (test: `spa_fallback_does_not_intercept_api_404`) |
| `compression` feature + `flate2`/`brotli` deps | Task 1 |
| `Compression` builder: `.gzip()`, `.brotli()`, `.min_size()` | Task 6 |
| `compression()` convenience fn | Task 6 |
| Skip binary MIME types | Task 6 (impl + test) |
| Skip body < min_size | Task 6 (impl + test) |
| Skip already-encoded responses | Task 6 (impl + test) |
| `Vary: Accept-Encoding` always set | Task 6 (impl + test) |
| Prefer brotli > gzip | Task 6 (impl + test) |
| `spa-demo` example | Task 7 |
| `static-files.mdx` docs page | Task 8 |
| Compression section in `middleware.mdx` | Task 8 |
| API reference updated | Task 8 |
| README feature flags updated | Task 8 |
| Roadmap → ✅ 0.5.0 | Task 8 |
| CI test runs added | Task 9 |

All spec requirements covered. ✓
