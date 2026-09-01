//! `ultimo mcp` — a Model Context Protocol server (stdio) that gives coding
//! agents live, typed tools for building Ultimo apps: search the docs, list and
//! read examples, scaffold projects, and introspect a project's typed RPC surface.
//!
//! IMPORTANT: the MCP protocol owns stdout; never `println!` here — logs go to
//! stderr.

mod docs;
mod examples;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::transport::stdio;
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use tokio::process::Command;

use docs::DocsCorpus;

const LLMS_FULL_URL: &str = "https://docs.ultimo.dev/llms-full.txt";

/// The Ultimo MCP server.
#[derive(Clone)]
pub struct UltimoMcp {
    #[expect(dead_code, reason = "tool_handler macro accesses this router field")]
    tool_router: ToolRouter<Self>,
    docs: Arc<DocsCorpus>,
}

impl UltimoMcp {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            docs: Arc::new(DocsCorpus::from_url(LLMS_FULL_URL)),
        }
    }
}

impl Default for UltimoMcp {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchDocsInput {
    /// What to search the Ultimo documentation for (e.g. "server-sent events",
    /// "jwt middleware", "rpc client generation").
    pub query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetDocInput {
    /// The documentation page title or slug (e.g. "sse", "Server-Sent Events",
    /// "jwt", "rpc").
    pub page: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetExampleInput {
    /// The example name (e.g. "rpc-modes", "sse", "jwt-auth").
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ScaffoldInput {
    /// Template: basic, fullstack, api-only, rpc, or production.
    pub template: String,
    /// Project name (a new directory with this name is created).
    pub name: String,
    /// Directory to create the project in. Defaults to the current directory.
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IntrospectRpcInput {
    /// Path to the Ultimo project directory (must contain a `generate-client` bin).
    pub project_path: String,
}

#[tool_router]
impl UltimoMcp {
    #[tool(
        description = "Search the Ultimo documentation and return the most relevant sections. Use this to ground yourself in the current Ultimo API before writing code, instead of relying on memory."
    )]
    async fn search_docs(
        &self,
        Parameters(SearchDocsInput { query }): Parameters<SearchDocsInput>,
    ) -> String {
        match self.docs.get().await {
            Ok(corpus) => corpus.search(&query, 5),
            Err(e) => format!("Could not fetch Ultimo docs from {LLMS_FULL_URL}: {e}"),
        }
    }

    #[tool(description = "Return the full text of a named Ultimo documentation page.")]
    async fn get_doc(&self, Parameters(GetDocInput { page }): Parameters<GetDocInput>) -> String {
        match self.docs.get().await {
            Ok(corpus) => corpus
                .page(&page)
                .unwrap_or_else(|| format!("No Ultimo docs page matches '{page}'.")),
            Err(e) => format!("Could not fetch Ultimo docs from {LLMS_FULL_URL}: {e}"),
        }
    }

    #[tool(
        description = "List the runnable Ultimo example projects with a one-line description of each."
    )]
    async fn list_examples(&self) -> String {
        examples::catalog()
    }

    #[tool(description = "Return the source of a named Ultimo example (fetched from GitHub).")]
    async fn get_example(
        &self,
        Parameters(GetExampleInput { name }): Parameters<GetExampleInput>,
    ) -> String {
        examples::source(&name).await
    }

    #[tool(
        description = "Scaffold a new Ultimo project from a template into a directory, returning the created file tree. Templates: basic, fullstack, api-only, rpc, production."
    )]
    async fn scaffold(
        &self,
        Parameters(ScaffoldInput {
            template,
            name,
            path,
        }): Parameters<ScaffoldInput>,
    ) -> String {
        scaffold(&template, &name, path.as_deref()).await
    }

    #[tool(
        description = "Introspect an Ultimo project's typed RPC surface: build its RpcRegistry (via its generate-client bin) and return the generated TypeScript client plus the procedure list. Use this to see the project's exact typed API."
    )]
    async fn introspect_rpc(
        &self,
        Parameters(IntrospectRpcInput { project_path }): Parameters<IntrospectRpcInput>,
    ) -> String {
        introspect_rpc(&project_path).await
    }
}

/// Shell out to this same `ultimo` binary as a subprocess. Its stdout is a
/// separate stream from the MCP server's stdout, so reusing the CLI's own
/// (stdout-printing) logic can't corrupt the protocol.
async fn scaffold(template: &str, name: &str, path: Option<&str>) -> String {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => return format!("Cannot locate the ultimo binary: {e}"),
    };
    let base = path.unwrap_or(".");
    let output = Command::new(&exe)
        .args(["new", name, "--template", template])
        .current_dir(base)
        .output()
        .await;
    match output {
        Ok(o) if o.status.success() => {
            let dir = Path::new(base).join(name);
            format!(
                "Scaffolded '{name}' ({template}) at {}:\n\n{}",
                dir.display(),
                file_tree(&dir)
            )
        }
        Ok(o) => format!(
            "Scaffold failed:\n{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => format!("Failed to run `ultimo new`: {e}"),
    }
}

async fn introspect_rpc(project_path: &str) -> String {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => return format!("Cannot locate the ultimo binary: {e}"),
    };
    let tmp = std::env::temp_dir().join(format!("ultimo-introspect-{}.ts", std::process::id()));
    let output = Command::new(&exe)
        .args(["generate", "--project", project_path, "--output"])
        .arg(&tmp)
        .output()
        .await;
    match output {
        Ok(o) if o.status.success() => match std::fs::read_to_string(&tmp) {
            Ok(client) => {
                let procs = extract_procedures(&client);
                let _ = std::fs::remove_file(&tmp);
                let list = if procs.is_empty() {
                    "(none detected)".to_string()
                } else {
                    procs.join(", ")
                };
                format!("Procedures: {list}\n\n--- generated TypeScript client ---\n{client}")
            }
            Err(e) => format!("The client was generated but could not be read: {e}"),
        },
        Ok(o) => format!(
            "Could not introspect the RPC surface:\n{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => format!("Failed to run `ultimo generate`: {e}"),
    }
}

/// Best-effort: pull the client method names out of a generated `UltimoRpcClient`.
fn extract_procedures(client_ts: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in client_ts.lines() {
        let t = line.trim();
        // Matches generated methods like `async getUser(input: ...)` / `getUser(`.
        let sig = t.strip_prefix("async ").unwrap_or(t);
        if let Some((ident, rest)) = sig.split_once('(') {
            if !rest.is_empty()
                && !ident.is_empty()
                && ident.chars().all(|c| c.is_alphanumeric() || c == '_')
                && ident.chars().next().is_some_and(|c| c.is_lowercase())
                && !matches!(
                    ident,
                    "constructor" | "if" | "for" | "while" | "switch" | "catch"
                )
            {
                names.push(ident.to_string());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// A sorted, `target/`-free list of files under `dir`, relative to it.
fn file_tree(dir: &Path) -> String {
    let mut files = Vec::new();
    collect_files(dir, dir, &mut files);
    files.sort();
    if files.is_empty() {
        "(no files)".to_string()
    } else {
        files.join("\n")
    }
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            if p.file_name()
                .is_some_and(|n| n == "target" || n == "node_modules")
            {
                continue;
            }
            collect_files(root, &p, out);
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push(rel.display().to_string());
        }
    }
}

#[tool_handler]
impl ServerHandler for UltimoMcp {
    fn get_info(&self) -> ServerInfo {
        // `ServerInfo` is `#[non_exhaustive]`, so build via Default + field set.
        let mut info = ServerInfo::default();
        info.instructions = Some(
            "Tools for building apps with the Ultimo Rust web framework: search the docs, \
             read examples, scaffold projects, and introspect a project's typed RPC surface."
                .into(),
        );
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }
}

/// Run the stdio MCP server until the client disconnects.
pub async fn run() -> anyhow::Result<()> {
    let service = UltimoMcp::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_procedures_from_generated_client() {
        let client = r#"
export class UltimoRpcClient {
  constructor(private baseUrl: string) {}
  async getUser(input: GetUserInput): Promise<User> { }
  async createUser(input: CreateUserInput): Promise<User> { }
}
"#;
        let procs = extract_procedures(client);
        assert_eq!(procs, vec!["createUser", "getUser"]);
    }
}
