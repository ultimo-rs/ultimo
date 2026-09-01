//! `ultimo mcp` — a Model Context Protocol server (stdio) that gives coding
//! agents live, typed tools for building Ultimo apps: search the docs, list and
//! read examples, scaffold projects, and introspect a project's typed RPC surface.
//!
//! IMPORTANT: the MCP protocol owns stdout; never `println!` here — logs go to
//! stderr.

mod docs;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::transport::stdio;
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use serde::Deserialize;
use std::sync::Arc;

use docs::DocsCorpus;

const LLMS_FULL_URL: &str = "https://docs.ultimo.dev/llms-full.txt";

/// The Ultimo MCP server.
#[derive(Clone)]
pub struct UltimoMcp {
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
}

#[tool_handler]
impl ServerHandler for UltimoMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Tools for building apps with the Ultimo Rust web framework: search the docs, \
                 read examples, scaffold projects, and introspect a project's typed RPC surface."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// Run the stdio MCP server until the client disconnects.
pub async fn run() -> anyhow::Result<()> {
    let service = UltimoMcp::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
