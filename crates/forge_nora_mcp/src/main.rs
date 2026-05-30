//! forge-nora-mcp — MCP server wrapping the Nora artifact registry HTTP API.
//!
//! Exposes tools for searching crates, getting crate details, and checking
//! registry status. Intended to be used as a stdio-based MCP server in Forge's
//! `mcpServers` config.

use std::sync::Arc;

use futures::future::BoxFuture;
use futures::FutureExt;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::tool::ToolRoute;
use rmcp::handler::server::router::Router;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, ProtocolVersion,
    ServerCapabilities, Tool, ToolAnnotations,
};
use rmcp::model::JsonObject;
use rmcp::ServiceExt;
use reqwest::Client;
use tracing_subscriber::EnvFilter;

use std::future::Future;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const NORA_BASE_URL: &str = "http://127.0.0.1:4000";

// ---------------------------------------------------------------------------
// Server implementation
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct NoraMcp {
    client: Client,
}

impl NoraMcp {
    fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl ServerHandler for NoraMcp {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "forge-nora-mcp".into(),
                version: option_env!("APP_VERSION")
                    .unwrap_or(env!("CARGO_PKG_VERSION"))
                    .into(),
                title: Some("Forge Nora Registry MCP".into()),
                icons: None,
                website_url: None,
            },
            instructions: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Fetch a URL and return the body as text.
async fn nora_get(
    client: &Client,
    path: &str,
) -> Result<String, String> {
    let url = format!("{}{}", NORA_BASE_URL, path);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    if status.is_success() {
        Ok(body)
    } else if status == reqwest::StatusCode::NOT_FOUND {
        Err(format!("Not found (404): {path}"))
    } else {
        Err(format!("Nora returned {status}: {body:.200}"))
    }
}

/// Wrap a sync result into a tool response.
fn into_result(r: Result<String, String>) -> CallToolResult {
    match r {
        Ok(text) => CallToolResult::success(vec![Content::text(text)]),
        Err(err) => CallToolResult::error(vec![Content::text(err)]),
    }
}

/// Wrap an async result into a boxed future.
fn into_async_result(
    f: impl Future<Output = Result<String, String>> + Send + 'static,
) -> BoxFuture<'static, Result<CallToolResult, rmcp::ErrorData>> {
    async move {
        let r = f.await;
        Ok(into_result(r))
    }
    .boxed()
}

/// Extract an optional string argument from a tool call context.
fn opt_arg<'a>(
    ctx: &'a ToolCallContext<'_, NoraMcp>,
    name: &str,
) -> Option<&'a str> {
    ctx.arguments
        .as_ref()
        .and_then(|args| args.get(name))
        .and_then(|v| v.as_str())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("forge_nora_mcp=info".parse().unwrap())
                .add_directive("rmcp=warn".parse().unwrap()),
        )
        .without_time()
        .init();

    tracing::info!("starting forge-nora-mcp server (nora at {NORA_BASE_URL})");

    let server = NoraMcp::new();
    let client = server.client.clone();

    // ── registry_info ───────────────────────────────────────────────────────
    let registry_info_tool = {
        let client = client.clone();
        ToolRoute::new_dyn(
            Tool::new(
                "nora_registry_info",
                "Get Nora registry configuration including supported protocols",
                Arc::new(serde_json::json!({}).as_object().cloned().unwrap()),
            )
            .annotate(ToolAnnotations::new().read_only(true)),
            move |_ctx: ToolCallContext<'_, NoraMcp>| {
                let c = client.clone();
                into_async_result(async move {
                    nora_get(&c, "/cargo/index/config.json").await
                })
            },
        )
    };

    // ── get_crate ──────────────────────────────────────────────────────────
    let get_crate_schema: Arc<JsonObject> = Arc::new(
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Crate name to look up"
                }
            },
            "required": ["name"],
            "additionalProperties": false
        })
        .as_object()
        .cloned()
        .unwrap(),
    );

    let get_crate_tool = {
        let client = client.clone();
        ToolRoute::new_dyn(
            Tool::new(
                "nora_get_crate",
                "Get detailed information about a specific crate from the Nora registry",
                get_crate_schema,
            )
            .annotate(ToolAnnotations::new().read_only(true)),
            move |ctx: ToolCallContext<'_, NoraMcp>| {
                let c = client.clone();
                let name = opt_arg(&ctx, "name").unwrap_or("").to_string();
                into_async_result(async move {
                    nora_get(&c, &format!("/cargo/api/v1/crates/{name}")).await
                })
            },
        )
    };

    // ── search_crates ──────────────────────────────────────────────────────
    let search_schema: Arc<JsonObject> = Arc::new(
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for crate names"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default 10, max 50)"
                }
            },
            "additionalProperties": false
        })
        .as_object()
        .cloned()
        .unwrap(),
    );

    let search_crates_tool = {
        let client = client.clone();
        ToolRoute::new_dyn(
            Tool::new(
                "nora_search_crates",
                "Search crates in the Nora registry by name query",
                search_schema,
            )
            .annotate(ToolAnnotations::new().read_only(true)),
            move |ctx: ToolCallContext<'_, NoraMcp>| {
                let c = client.clone();
                let query = opt_arg(&ctx, "query").unwrap_or("").to_string();
                let limit = ctx
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("limit"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10)
                    .min(50);
                into_async_result(async move {
                    nora_get(
                        &c,
                        &format!(
                            "/cargo/api/v1/crates?q={query}&per_page={limit}"
                        ),
                    )
                    .await
                })
            },
        )
    };

    // ── registry_status ────────────────────────────────────────────────────
    let registry_status_tool = {
        ToolRoute::new_dyn(
            Tool::new(
                "nora_registry_status",
                "Health check for the Nora registry service",
                Arc::new(serde_json::json!({}).as_object().cloned().unwrap()),
            )
            .annotate(ToolAnnotations::new().read_only(true)),
            move |_ctx: ToolCallContext<'_, NoraMcp>| {
                let c = client.clone();
                into_async_result(async move {
                    // Try the status endpoint; if not available, fall back to config
                    match nora_get(&c, "/cargo/index/config.json").await {
                        Ok(body) => Ok(format!("Nora registry is healthy\n\n{body}")),
                        Err(e) => Err(format!("Nora registry is unreachable: {e}")),
                    }
                })
            },
        )
    };

    // ── Assemble router ────────────────────────────────────────────────────
    let router = Router::new(server)
        .with_tool(registry_info_tool)
        .with_tool(get_crate_tool)
        .with_tool(search_crates_tool)
        .with_tool(registry_status_tool);

    // Serve over stdio
    let running = router
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;

    // Keep the event loop alive until stdin closes
    running.waiting().await?;

    tracing::info!("server stopped");
    Ok(())
}
