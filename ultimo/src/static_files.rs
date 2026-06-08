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

    // Canonicalize the root so we have an absolute, symlink-resolved base.
    let canonical_root = tokio::fs::canonicalize(root)
        .await
        .map_err(|_| UltimoError::NotFound("static root not found".into()))?;

    // Strip any leading slashes or `./` from the caller-supplied relative path.
    let rel_clean = rel_path.trim_start_matches('/').trim_start_matches("./");

    // Build candidate absolute path.
    let candidate = canonical_root.join(rel_clean);

    // Canonicalize the candidate — resolves `..` and symlinks.
    // If the file doesn't exist, `canonicalize` returns an error → 404.
    let resolved = tokio::fs::canonicalize(&candidate)
        .await
        .map_err(|_| UltimoError::NotFound("file not found".into()))?;

    // Path traversal guard: resolved path must remain under root.
    if !resolved.starts_with(&canonical_root) {
        return Err(UltimoError::NotFound("file not found".into()));
    }

    // Stat the resolved path and require it to be a regular file.
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

    // Conditional GET: 304 if the client's cached ETag matches.
    if let Some(ref inm) = if_none_match {
        if inm.trim() == etag.as_str() {
            return Ok(hyper::Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Full::new(Bytes::new()))
                .unwrap());
        }
    }

    // Read file contents.
    let content = tokio::fs::read(&resolved)
        .await
        .map_err(|_| UltimoError::NotFound("file not found".into()))?;

    // MIME type from extension, defaulting to application/octet-stream.
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
