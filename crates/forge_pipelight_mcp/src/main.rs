//! forge-pipelight-mcp — MCP server wrapping the `pipelight` CLI for
//! non-blocking build management.
//!
//! Exposes tools for inspecting and triggering Nix/pipeline builds.
//! Intended to be used as a stdio-based MCP server in Forge's `mcpServers` config.

use std::sync::Arc;

use futures::future::BoxFuture;
use futures::FutureExt;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::tool::ToolRoute;
use rmcp::handler::server::router::Router;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{CallToolResult, Content, Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, Tool, ToolAnnotations};
use rmcp::model::JsonObject;
use rmcp::ServiceExt;
use tokio::process::Command;
use tracing_subscriber::EnvFilter;

use std::future::Future;

// ---------------------------------------------------------------------------
// Server implementation
// ---------------------------------------------------------------------------

/// The MCP server that wraps pipelight CLI commands.
#[derive(Clone)]
struct PipelightMcp;

impl ServerHandler for PipelightMcp {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "forge-pipelight-mcp".into(),
                version: option_env!("APP_VERSION")
                    .unwrap_or(env!("CARGO_PKG_VERSION"))
                    .into(),
                title: Some("Forge Pipelight MCP".into()),
                icons: None,
                website_url: None,
            },
            instructions: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tool helpers
// ---------------------------------------------------------------------------

/// Helper: run a `pipelight` subcommand and return stdout (or a descriptive
/// error).
async fn pipelight(args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("pipelight");
    cmd.args(args).kill_on_drop(true);

    let output = cmd.output().await.map_err(|e| {
        format!(
            "failed to execute `pipelight {}`: {e}. is pipelight installed?",
            args.join(" ")
        )
    })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!(
            "`pipelight {}` exited with {}:\n{stderr}",
            args.join(" "),
            output.status
        ))
    }
}

/// Extract an optional string argument from a tool call context.
fn opt_arg<'a>(ctx: &'a ToolCallContext<'_, PipelightMcp>, name: &str) -> Option<&'a str> {
    ctx.arguments
        .as_ref()
        .and_then(|args| args.get(name))
        .and_then(|v| v.as_str())
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

// ---------------------------------------------------------------------------
// Tool factory: read-only tools without arguments
// ---------------------------------------------------------------------------

fn make_readonly_tool(
    name: &'static str,
    description: &'static str,
    args: &'static [&'static str],
) -> ToolRoute<PipelightMcp> {
    let tool = Tool::new(name, description, Arc::new(serde_json::Map::new()))
        .annotate(ToolAnnotations::new().read_only(true));
    ToolRoute::new_dyn(tool, move |_ctx: ToolCallContext<'_, PipelightMcp>| {
        into_async_result(pipelight(args))
    })
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("forge_pipelight_mcp=info".parse().unwrap())
                .add_directive("rmcp=warn".parse().unwrap()),
        )
        .without_time()
        .init();

    tracing::info!("starting forge-pipelight-mcp server");

    // ── status ──────────────────────────────────────────────────────────────

    let pipelight_status = make_readonly_tool(
        "pipelight_status",
        "Get the current pipelight pipeline build status",
        &["status"],
    );

    // ── list ────────────────────────────────────────────────────────────────
    let pipelight_list = make_readonly_tool(
        "pipelight_list",
        "List available pipelight pipeline configurations with their status",
        &["list"],
    );

    // ── logs ────────────────────────────────────────────────────────────────
    let logs_schema: Arc<JsonObject> = Arc::new(
        serde_json::json!({
            "type": "object",
            "properties": {
                "pipe": {
                    "type": "string",
                    "description": "Optional pipeline name to filter logs"
                },
                "branch": {
                    "type": "string",
                    "description": "Optional branch to filter logs"
                }
            },
            "additionalProperties": false
        })
        .as_object()
        .cloned()
        .unwrap(),
    );
    let pipelight_logs = ToolRoute::new_dyn(
        Tool::new("pipelight_logs", "Fetch recent pipelight build logs. Optionally filter by pipe name or branch.", logs_schema)
            .annotate(ToolAnnotations::new().read_only(true)),
        |ctx: ToolCallContext<'_, PipelightMcp>| {
            let pipe = opt_arg(&ctx, "pipe").map(|s| s.to_string());
            let branch = opt_arg(&ctx, "branch").map(|s| s.to_string());
            into_async_result(async move {
                let mut args = vec!["logs"];
                if let Some(b) = branch.as_deref() {
                    args.push("--branch");
                    args.push(b);
                }
                if let Some(p) = pipe.as_deref() {
                    args.push(p);
                }
                pipelight(&args).await
            })
        },
    );

    // ── run ─────────────────────────────────────────────────────────────────
    let run_schema: Arc<JsonObject> = Arc::new(
        serde_json::json!({
            "type": "object",
            "properties": {
                "pipe": {
                    "type": "string",
                    "description": "Pipeline name to run (runs default if omitted)"
                },
                "branch": {
                    "type": "string",
                    "description": "Branch to run the pipeline on"
                }
            },
            "additionalProperties": false
        })
        .as_object()
        .cloned()
        .unwrap(),
    );
    let pipelight_run = ToolRoute::new_dyn(
        Tool::new("pipelight_run", "Trigger a new pipeline run. Optionally specify a pipe name or branch.", run_schema)
            .annotate(ToolAnnotations::new().destructive(true)),
        |ctx: ToolCallContext<'_, PipelightMcp>| {
            let pipe = opt_arg(&ctx, "pipe").map(|s| s.to_string());
            let branch = opt_arg(&ctx, "branch").map(|s| s.to_string());
            into_async_result(async move {
                let mut args = vec!["run"];
                if let Some(b) = branch.as_deref() {
                    args.push("--branch");
                    args.push(b);
                }
                if let Some(p) = pipe.as_deref() {
                    args.push(p);
                }
                pipelight(&args).await
            })
        },
    );

    // ── Assemble router ─────────────────────────────────────────────────────
    let router = Router::new(PipelightMcp)
        .with_tool(pipelight_status)
        .with_tool(pipelight_list)
        .with_tool(pipelight_logs)
        .with_tool(pipelight_run);

    // Serve over stdio (the standard MCP transport for subprocess servers)
    router
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;

    tracing::info!("server stopped");
    Ok(())
}
