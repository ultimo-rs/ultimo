//! The bundled catalog of runnable Ultimo examples, and a fetcher for their
//! source. The list changes rarely, so it lives in the binary; `get_example`
//! pulls the current source from GitHub on demand.

/// (name, one-line description). `name` is the directory under `examples/`.
const EXAMPLES: &[(&str, &str)] = &[
    ("basic", "Minimal REST API."),
    (
        "rpc-modes",
        "Typed RPC → TypeScript client + React hooks (REST and JSON-RPC modes).",
    ),
    ("openapi-demo", "Generate an OpenAPI spec + Swagger UI."),
    ("database-sqlx", "Postgres persistence with SQLx."),
    ("database-diesel", "Postgres persistence with Diesel."),
    (
        "database-api-styles",
        "REST vs RPC over the same SQLx data layer.",
    ),
    (
        "websocket-chat",
        "WebSocket chat with the built-in pub/sub.",
    ),
    (
        "websocket-chat-react",
        "WebSocket chat with a React frontend.",
    ),
    ("session-auth", "Cookie sessions + login (session feature)."),
    ("jwt-auth", "JWT login → protected route (jwt feature)."),
    ("spa-demo", "Serve a single-page app with fallback routing."),
    ("streaming", "Chunked/streaming responses via ctx.stream."),
    ("sse", "Server-Sent Events via ctx.sse + EventSource."),
];

const RAW_BASE: &str = "https://raw.githubusercontent.com/ultimo-rs/ultimo/main/examples";
const TREE_BASE: &str = "https://github.com/ultimo-rs/ultimo/tree/main/examples";

/// A formatted catalog of examples for `list_examples`.
pub fn catalog() -> String {
    let mut out = String::from("Runnable Ultimo examples (run with `cargo run -p <name>`):\n\n");
    for (name, desc) in EXAMPLES {
        out.push_str(&format!("- {name} — {desc}\n"));
    }
    out.push_str("\nUse get_example(name) to read an example's source.");
    out
}

/// Fetch an example's `src/main.rs` from GitHub, or explain how to browse it.
pub async fn source(name: &str) -> String {
    let name = name.trim();
    if !EXAMPLES.iter().any(|(n, _)| *n == name) {
        let names: Vec<&str> = EXAMPLES.iter().map(|(n, _)| *n).collect();
        return format!("Unknown example '{name}'. Available: {}", names.join(", "));
    }
    let url = format!("{RAW_BASE}/{name}/src/main.rs");
    match reqwest::get(&url).await.and_then(|r| r.error_for_status()) {
        Ok(resp) => match resp.text().await {
            Ok(text) => format!("// {TREE_BASE}/{name}\n\n{text}"),
            Err(e) => fetch_hint(name, &e.to_string()),
        },
        Err(e) => fetch_hint(name, &e.to_string()),
    }
}

fn fetch_hint(name: &str, err: &str) -> String {
    format!(
        "Could not fetch {name}/src/main.rs ({err}). Browse it at {TREE_BASE}/{name} \
         (some examples have multiple files)."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_known_examples() {
        let c = catalog();
        assert!(c.contains("rpc-modes"));
        assert!(c.contains("sse"));
    }

    #[tokio::test]
    async fn source_rejects_unknown_example() {
        let out = source("does-not-exist").await;
        assert!(out.contains("Unknown example"));
    }
}
