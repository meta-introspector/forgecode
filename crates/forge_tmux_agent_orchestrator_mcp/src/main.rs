//! forge-tmux-agent-orchestrator-mcp — MCP server that orchestrates
//! other agents inside tmux sessions.
//!
//! Spawns, monitors, communicates with, and kills agents running in
//! named tmux sessions. Designed to work in tandem with tmux-mcp-rs
//! (which handles the low-level tmux pane/session operations) by
//! keeping agents isolated per session and tracking their lifecycle.
//!
//! # Workflow
//!
//! 1. `spawn_agent`  — create a tmux session, start a command shell
//! 2. `read_agent`   — capture session output (pane capture + scrollback)
//! 3. `send_to_agent` — type keystrokes / text into the agent's session
//! 4. `kill_agent`   — kill the session and clean up metadata
//! 5. `list_agents`  — list all tracked agent sessions
//! 6. `agent_status` — get status of a specific agent

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::FutureExt;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::tool::ToolRoute;
use rmcp::handler::server::router::Router;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, ProtocolVersion,
    ServerCapabilities,
};
use rmcp::model::JsonObject;
use rmcp::ServiceExt;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// Metadata about an agent running in a tmux session.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct AgentMeta {
    /// The tmux session name we assigned.
    tmux_session: String,
    /// Human-readable label for the agent.
    label: String,
    /// The command / shell that was started (e.g. `"bash"`, `"nix develop"`).
    command: String,
    /// When the agent was spawned.
    created_at: DateTime<Utc>,
    /// Optional free-form notes set by the user at spawn time.
    notes: String,
}

// ---------------------------------------------------------------------------
// Server state
// ---------------------------------------------------------------------------

struct ServerState {
    agents: HashMap<String, AgentMeta>,
}

/// The MCP server that orchestrates agent sessions.
#[derive(Clone)]
struct AgentOrchestrator {
    state: Arc<RwLock<ServerState>>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }
}

impl Default for AgentOrchestrator {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(ServerState::default())),
        }
    }
}

impl ServerHandler for AgentOrchestrator {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "forge-tmux-agent-orchestrator-mcp".into(),
                version: option_env!("APP_VERSION")
                    .unwrap_or(env!("CARGO_PKG_VERSION"))
                    .into(),
                title: Some("Forge TMUX Agent Orchestrator MCP".into()),
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

fn opt_arg<'a>(ctx: &'a ToolCallContext<'_, AgentOrchestrator>, name: &str) -> Option<&'a str> {
    ctx.arguments
        .as_ref()
        .and_then(|args| args.get(name))
        .and_then(|v| v.as_str())
}

fn req_arg<'a>(ctx: &'a ToolCallContext<'_, AgentOrchestrator>, name: &str) -> Result<&'a str, String> {
    opt_arg(ctx, name).ok_or_else(|| format!("missing required argument: `{name}`"))
}

fn into_result(r: Result<String, String>) -> CallToolResult {
    match r {
        Ok(text) => CallToolResult::success(vec![Content::text(text)]),
        Err(err) => CallToolResult::error(vec![Content::text(err)]),
    }
}

/// Wrap an async `Result<String, String>` into a boxed future returning
/// `Result<CallToolResult, rmcp::ErrorData>` — the type expected by
/// `ToolRoute::new_dyn`.
fn into_async_result(
    f: impl Future<Output = Result<String, String>> + Send + 'static,
) -> BoxFuture<'static, Result<CallToolResult, rmcp::ErrorData>> {
    async move {
        let r = f.await;
        Ok(into_result(r))
    }
    .boxed()
}

/// Run a `tmux` command and return stdout (or a descriptive error).
async fn tmux(args: &[String]) -> Result<String, String> {
    let mut cmd = Command::new("tmux");
    for a in args {
        cmd.arg(a);
    }
    cmd.kill_on_drop(true);

    let output = cmd.output().await.map_err(|e| {
        format!(
            "failed to execute `tmux {}`: {e}. is tmux installed?",
            args.join(" ")
        )
    })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!(
            "`tmux {}` exited with {}:\n{stderr}",
            args.join(" "),
            output.status
        ))
    }
}

/// Check whether a tmux session exists.
async fn session_exists(name: &str) -> bool {
    tmux(&[
        "has-session".to_string(),
        "-t".to_string(),
        name.to_string(),
    ])
    .await
    .is_ok()
}

/// Generate a unique tmux session name for an agent.
fn make_session_name(label: &str) -> String {
    let ts = Utc::now().format("%H%M%S");
    let slug: String = label
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .take(12)
        .collect();
    format!("agent-{slug}-{ts}")
}

// ---------------------------------------------------------------------------
// JSON schema helpers
// ---------------------------------------------------------------------------

fn schema(properties: Vec<(&str, serde_json::Value)>, required: Vec<&str>) -> Arc<JsonObject> {
    let mut props = serde_json::Map::new();
    for (name, value) in properties {
        props.insert(name.to_string(), value);
    }
    Arc::new(
        serde_json::json!({
            "type": "object",
            "properties": props,
            "required": required,
            "additionalProperties": false,
        })
        .as_object()
        .cloned()
        .unwrap(),
    )
}

fn string_prop(description: &str) -> serde_json::Value {
    serde_json::json!({ "type": "string", "description": description })
}

fn integer_prop(description: &str, default: i64) -> serde_json::Value {
    serde_json::json!({ "type": "integer", "description": description, "default": default })
}

fn boolean_prop(description: &str, default: bool) -> serde_json::Value {
    serde_json::json!({
        "type": "boolean",
        "description": description,
        "default": default,
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
                .add_directive("forge_tmux_agent_orchestrator_mcp=info".parse().unwrap())
                .add_directive("rmcp=warn".parse().unwrap()),
        )
        .without_time()
        .init();

    tracing::info!("starting forge-tmux-agent-orchestrator-mcp server");

    let orchestrator = AgentOrchestrator::default();
    let state = orchestrator.state.clone();

    // ── spawn_agent ────────────────────────────────────────────────────────

    let spawn_tool = {
        let state = state.clone();
        ToolRoute::new_dyn(
            rmcp::model::Tool::new(
                "spawn_agent",
                "Create a new tmux session and start an agent shell in it. Returns the session name.",
                schema(
                    vec![
                        ("label", string_prop("Human-readable label for the agent (used in session name)")),
                        ("command", string_prop("Shell command to start the agent with (default: bash)")),
                        ("notes", string_prop("Optional free-form notes to attach to the agent")),
                    ],
                    vec!["label"],
                ),
            )
            .annotate(rmcp::model::ToolAnnotations::new().destructive(true)),
            move |ctx: ToolCallContext<'_, AgentOrchestrator>| {
                let label = match req_arg(&ctx, "label") {
                    Ok(v) => v.to_string(),
                    Err(e) => {
                        return futures::future::ready(
                            Ok(into_result(Err(e))),
                        ).boxed();
                    }
                };
                let command = opt_arg(&ctx, "command").unwrap_or("bash").to_string();
                let notes = opt_arg(&ctx, "notes").unwrap_or("").to_string();
                let state = state.clone();

                into_async_result(async move {
                    let session_name = make_session_name(&label);
                    let escaped_cmd = command.replace('"', "\\\"");

                    let output = tokio::process::Command::new("tmux")
                        .args([
                            "new-session", "-d", "-s", &session_name,
                            "-x", "200", "-y", "60",
                            &escaped_cmd,
                        ])
                        .output()
                        .await
                        .map_err(|e| format!("failed to run tmux new-session: {e}"))?;

                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Err(format!(
                            "tmux new-session failed ({}): {stderr}",
                            output.status
                        ));
                    }

                    let meta = AgentMeta {
                        tmux_session: session_name.clone(),
                        label: label.clone(),
                        command: command.clone(),
                        created_at: Utc::now(),
                        notes: notes.clone(),
                    };

                    state.write().await.agents.insert(session_name.clone(), meta);

                    let result = serde_json::json!({
                        "status": "ok",
                        "session": session_name,
                        "label": label,
                        "command": command,
                    });

                    Ok(result.to_string())
                })
            },
        )
    };

    // ── list_agents ────────────────────────────────────────────────────────

    let list_tool = {
        let state = state.clone();
        let tool = rmcp::model::Tool::new(
            "list_agents",
            "List all tracked agent sessions with their status (alive / dead).",
            Arc::new(serde_json::Map::new()),
        )
        .annotate(rmcp::model::ToolAnnotations::new().read_only(true));

        ToolRoute::new_dyn(tool, move |_: ToolCallContext<'_, AgentOrchestrator>| {
            let state = state.clone();
            into_async_result(async move {
                let agents = state.read().await.agents.clone();
                if agents.is_empty() {
                    return Ok("No agents found.".to_string());
                }

                let mut results = Vec::new();
                for (session, meta) in &agents {
                    let alive = session_exists(session).await;
                    let age = Utc::now()
                        .signed_duration_since(meta.created_at)
                        .num_seconds();
                    results.push(serde_json::json!({
                        "session": session,
                        "label": meta.label,
                        "command": meta.command,
                        "alive": alive,
                        "age_seconds": age,
                        "created_at": meta.created_at.to_rfc3339(),
                    }));
                }

                let output = serde_json::json!({
                    "count": results.len(),
                    "agents": results
                });
                Ok(output.to_string())
            })
        })
    };

    // ── agent_status ───────────────────────────────────────────────────────

    let status_tool = {
        let state = state.clone();
        ToolRoute::new_dyn(
            rmcp::model::Tool::new(
                "agent_status",
                "Check whether a tracked agent session is still alive and show its metadata.",
                schema(
                    vec![
                        ("session", string_prop("Tmux session name to check (returned by spawn_agent)")),
                    ],
                    vec!["session"],
                ),
            )
            .annotate(rmcp::model::ToolAnnotations::new().read_only(true)),
            move |ctx: ToolCallContext<'_, AgentOrchestrator>| {
                let session = match req_arg(&ctx, "session") {
                    Ok(v) => v.to_string(),
                    Err(e) => {
                        return futures::future::ready(
                            Ok(into_result(Err(e))),
                        ).boxed();
                    }
                };
                let state = state.clone();
                into_async_result(async move {
                    let agents = state.read().await;
                    let meta = agents.agents.get(&session).cloned();
                    drop(agents);

                    match meta {
                        Some(m) => {
                            let alive = session_exists(&session).await;
                            let age = Utc::now()
                                .signed_duration_since(m.created_at)
                                .num_seconds();
                            let result = serde_json::json!({
                                "status": if alive { "alive" } else { "dead" },
                                "session": session,
                                "label": m.label,
                                "command": m.command,
                                "alive": alive,
                                "age_seconds": age,
                                "created_at": m.created_at.to_rfc3339(),
                                "notes": m.notes,
                            });
                            Ok(result.to_string())
                        }
                        None => Err(format!("unknown session: `{session}`")),
                    }
                })
            },
        )
    };

    // ── send_to_agent ─────────────────────────────────────────────────────

    let send_tool = {
        let state = state.clone();
        ToolRoute::new_dyn(
            rmcp::model::Tool::new(
                "send_to_agent",
                "Send keystrokes (text) into an agent's tmux session.",
                schema(
                    vec![
                        ("session", string_prop("Tmux session name to send input to")),
                        ("input", string_prop("Text to send (keystrokes) to the agent session")),
                        ("enter", boolean_prop("Whether to press Enter after the input (default: true)", true)),
                    ],
                    vec!["session", "input"],
                ),
            ),
            move |ctx: ToolCallContext<'_, AgentOrchestrator>| {
                let session = match req_arg(&ctx, "session") {
                    Ok(v) => v.to_string(),
                    Err(e) => {
                        return futures::future::ready(
                            Ok(into_result(Err(e))),
                        ).boxed();
                    }
                };
                let input = match req_arg(&ctx, "input") {
                    Ok(v) => v.to_string(),
                    Err(e) => {
                        return futures::future::ready(
                            Ok(into_result(Err(e))),
                        ).boxed();
                    }
                };
                let enter = opt_arg(&ctx, "enter").unwrap_or("true") == "true";
                let state = state.clone();

                into_async_result(async move {
                    // Verify the agent exists
                    let exists = {
                        let agents = state.read().await;
                        agents.agents.contains_key(&session)
                    };
                    if !exists {
                        return Err(format!("unknown session: `{session}`"));
                    }

                    if !session_exists(&session).await {
                        return Err(format!(
                            "session `{session}` is not alive (was killed or crashed)"
                        ));
                    }

                    let mut cmd = Command::new("tmux");
                    cmd.args(["send-keys", "-t", &session, "-l", &input]);

                    if enter {
                        cmd.arg("Enter");
                    }

                    let output = cmd
                        .output()
                        .await
                        .map_err(|e| format!("failed to run tmux send-keys: {e}"))?;

                    if output.status.success() {
                        Ok(serde_json::json!({
                            "status": "ok",
                            "session": session,
                            "sent": input,
                            "enter": enter,
                        }).to_string())
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        Err(format!(
                            "tmux send-keys failed ({}): {stderr}",
                            output.status
                        ))
                    }
                })
            },
        )
    };

    // ── read_agent ────────────────────────────────────────────────────────

    let read_tool = {
        let state = state.clone();
        ToolRoute::new_dyn(
            rmcp::model::Tool::new(
                "read_agent",
                "Capture visible output and scrollback from an agent's tmux session.",
                schema(
                    vec![
                        ("session", string_prop("Tmux session name to read output from")),
                        ("lines", integer_prop("Number of lines to capture (default: 50, use -1 for all)", 50)),
                    ],
                    vec!["session"],
                ),
            )
            .annotate(rmcp::model::ToolAnnotations::new().read_only(true)),
            move |ctx: ToolCallContext<'_, AgentOrchestrator>| {
                let session = match req_arg(&ctx, "session") {
                    Ok(v) => v.to_string(),
                    Err(e) => {
                        return futures::future::ready(
                            Ok(into_result(Err(e))),
                        ).boxed();
                    }
                };
                let lines_arg = opt_arg(&ctx, "lines").and_then(|v| v.parse::<i32>().ok());
                let state = state.clone();

                into_async_result(async move {
                    let exists = {
                        let agents = state.read().await;
                        agents.agents.contains_key(&session)
                    };
                    if !exists {
                        return Err(format!("unknown session: `{session}`"));
                    }
                    if !session_exists(&session).await {
                        return Err(format!(
                            "session `{session}` is not alive (was killed or crashed)"
                        ));
                    }

                    let actual_args: Vec<String> = if lines_arg.unwrap_or(50) > 0 {
                        let lines_val = lines_arg.unwrap_or(50);
                        vec![
                            "capture-pane".into(),
                            "-t".into(),
                            session.clone(),
                            "-p".into(),
                            "-S".into(),
                            "-".into(),
                            lines_val.to_string(),
                        ]
                    } else {
                        vec![
                            "capture-pane".into(),
                            "-t".into(),
                            session.clone(),
                            "-p".into(),
                        ]
                    };

                    match tmux(&actual_args).await {
                        Ok(output) => {
                            let line_count = output.lines().count();
                            let result = serde_json::json!({
                                "session": session,
                                "lines": line_count,
                                "output": output,
                            });
                            Ok(result.to_string())
                        }
                        Err(e) => Err(e),
                    }
                })
            },
        )
    };

    // ── kill_agent ─────────────────────────────────────────────────────────

    let kill_tool = {
        let state = state.clone();
        ToolRoute::new_dyn(
            rmcp::model::Tool::new(
                "kill_agent",
                "Kill an agent's tmux session and remove its metadata.",
                schema(
                    vec![
                        ("session", string_prop("Tmux session name to kill")),
                        ("force", boolean_prop("Kill without confirmation even if untracked (default: false)", false)),
                    ],
                    vec!["session"],
                ),
            )
            .annotate(rmcp::model::ToolAnnotations::new().destructive(true)),
            move |ctx: ToolCallContext<'_, AgentOrchestrator>| {
                let session = match req_arg(&ctx, "session") {
                    Ok(v) => v.to_string(),
                    Err(e) => {
                        return futures::future::ready(
                            Ok(into_result(Err(e))),
                        ).boxed();
                    }
                };
                let _force = opt_arg(&ctx, "force").unwrap_or("false") == "true";
                let state = state.clone();

                into_async_result(async move {
                    let meta = {
                        let agents = state.read().await;
                        agents.agents.get(&session).cloned()
                    };

                    match meta {
                        Some(_m) => {
                            let kill_output = Command::new("tmux")
                                .args(["kill-session", "-t", &session])
                                .output()
                                .await
                                .map_err(|e| format!("failed to run tmux kill-session: {e}"))?;

                            if !kill_output.status.success() {
                                let stderr = String::from_utf8_lossy(&kill_output.stderr);
                                return Err(format!(
                                    "tmux kill-session failed ({}): {stderr}",
                                    kill_output.status
                                ));
                            }

                            // Remove metadata
                            state.write().await.agents.remove(&session);

                            Ok(serde_json::json!({
                                "status": "ok",
                                "session": session,
                                "action": "killed"
                            }).to_string())
                        }
                        None => Err(format!("unknown session: `{session}`")),
                    }
                })
            },
        )
    };

    // ── Assemble router ────────────────────────────────────────────────────

    let router = Router::new(orchestrator)
        .with_tool(spawn_tool)
        .with_tool(list_tool)
        .with_tool(status_tool)
        .with_tool(send_tool)
        .with_tool(read_tool)
        .with_tool(kill_tool);

    router
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;

    tracing::info!("server stopped");
    Ok(())
}
