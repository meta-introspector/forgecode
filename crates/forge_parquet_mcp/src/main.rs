//! forge-parquet-mcp — MCP server for git inode scanning and parquet file operations.
//!
//! Exposes tools for scanning git repository inodes (with sharding by inode%71),
//! inspecting parquet file schemas, finding Rust code that writes parquet, and
//! listing available parquet-index tool binaries.
//!
//! Intended to be used as a stdio-based MCP server in Forge's `mcpServers` config.

use std::collections::HashMap;
use std::future::Future;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
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
use serde_json::json;
use tracing_subscriber::EnvFilter;
use walkdir::WalkDir;

// ---------------------------------------------------------------------------
// Default directories for parquet-index tools
// ---------------------------------------------------------------------------

const TMUX_TOOLS_DIR: &str = "/mnt/data1/nix/vendor/tmux/target/release";
const ZOS_PARQUET_INDEX_DIR: &str = "/mnt/data1/nix/time/2024/12/10/swarms-terraform/services/submodules/zos-server/plugins/parquet-index";

// ---------------------------------------------------------------------------
// Inode scanning types (ported from import_git_inodes.rs)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct InodeKey {
    device: u64,
    inode: u64,
}

impl InodeKey {
    fn from_path(path: &Path) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        Ok(InodeKey {
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    fn shard(&self) -> u8 {
        (self.inode % 71) as u8
    }
}

fn detect_file_type(path: &Path) -> String {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.ends_with(".pack") {
            return "pack".to_string();
        } else if name.ends_with(".idx") {
            return "idx".to_string();
        }
    }

    if let Some(path_str) = path.to_str() {
        if path_str.contains(".git/objects/") && path_str.matches('/').count() >= 2 {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() >= 2 {
                let last = parts[parts.len() - 1];
                let second_last = parts[parts.len() - 2];
                if second_last.len() == 2 && last.len() == 38 {
                    return "object".to_string();
                }
            }
        }
    }

    "regular".to_string()
}

#[derive(Debug, serde::Serialize)]
struct InodeScanResult {
    total_inodes: usize,
    shard_distribution: Vec<ShardEntry>,
    type_counts: HashMap<String, usize>,
    monster_prime_shards: Vec<PrimeShardEntry>,
    aether_shard: usize,
}

#[derive(Debug, serde::Serialize)]
struct ShardEntry {
    shard: usize,
    count: usize,
}

#[derive(Debug, serde::Serialize)]
struct PrimeShardEntry {
    prime: usize,
    count: usize,
}

fn scan_git_inodes(repo_dir: &str, max_files: Option<usize>) -> Result<InodeScanResult, String> {
    let git_path = PathBuf::from(repo_dir);
    if !git_path.exists() {
        return Err(format!("Path does not exist: {repo_dir}"));
    }

    let mut shard_counts = [0usize; 71];
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    let mut total_inodes = 0usize;

    let entries: Vec<_> = WalkDir::new(&git_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    for entry in entries {
        if let Some(max) = max_files {
            if total_inodes >= max {
                break;
            }
        }

        let path = entry.path();
        if let Ok(key) = InodeKey::from_path(path) {
            let shard = key.shard();
            let file_type = detect_file_type(path);
            shard_counts[shard as usize] += 1;
            *type_counts.entry(file_type).or_insert(0) += 1;
            total_inodes += 1;
        }
    }

    // Build shard distribution (sorted by count descending)
    let mut shard_vec: Vec<(usize, usize)> = shard_counts
        .iter()
        .enumerate()
        .map(|(i, &c)| (i, c))
        .collect();
    shard_vec.sort_by_key(|&(_, c)| std::cmp::Reverse(c));
    let shard_distribution: Vec<ShardEntry> = shard_vec
        .into_iter()
        .filter(|(_, c)| *c > 0)
        .take(20)
        .map(|(shard, count)| ShardEntry { shard, count })
        .collect();

    // Monster prime shards
    let monster_primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 41, 47, 59];
    let monster_prime_shards: Vec<PrimeShardEntry> = monster_primes
        .iter()
        .map(|&p| PrimeShardEntry {
            prime: p,
            count: shard_counts[p],
        })
        .collect();

    Ok(InodeScanResult {
        total_inodes,
        shard_distribution,
        type_counts,
        monster_prime_shards,
        aether_shard: shard_counts[70],
    })
}

/// List available binaries in the tmux tools directory.
fn list_tmux_tools() -> Vec<String> {
    let dir = Path::new(TMUX_TOOLS_DIR);
    if !dir.exists() {
        return Vec::new();
    }

    let mut tools = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && !path.extension().is_some_and(|e| e == "d") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    tools.push(name.to_string());
                }
            }
        }
    }
    tools.sort();
    tools
}

/// List available .rs files in the zos parquet-index plugin.
fn list_zos_parquet_tools() -> Vec<String> {
    let dir = Path::new(ZOS_PARQUET_INDEX_DIR);
    if !dir.exists() {
        return Vec::new();
    }

    let mut tools = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    tools.push(name.to_string());
                }
            }
        }
    }
    tools.sort();
    tools
}

/// Search for Rust files that contain parquet writing patterns.
fn find_parquet_writers(search_dir: &str, max_results: Option<usize>) -> Vec<String> {
    let dir = Path::new(search_dir);
    if !dir.exists() {
        return Vec::new();
    }

    let writers_keywords = [
        "serializedfilewriter",
        "parquet::file::writer",
        "write_parquet",
        "to_parquet",
    ];

    let mut results = Vec::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if let Some(max) = max_results {
            if results.len() >= max {
                break;
            }
        }

        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs") {
            if let Ok(content) = std::fs::read_to_string(path) {
                let lower = content.to_lowercase();
                if writers_keywords.iter().any(|kw| lower.contains(kw)) {
                    results.push(path.to_string_lossy().to_string());
                }
            }
        }
    }

    results
}

/// Inspect a parquet file by running the tmux binary.
fn inspect_parquet_file(file_path: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {file_path}"));
    }

    let metadata = std::fs::metadata(path).map_err(|e| format!("Cannot read metadata: {e}"))?;
    let size = metadata.len();
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?");

    Ok(json!({
        "file": file_name,
        "path": file_path,
        "size_bytes": size,
        "size_display": format_size(size),
        "is_parquet": file_path.ends_with(".parquet") || file_path.ends_with(".pq")
    })
    .to_string())
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.2} {}", size, UNITS[unit_idx])
}

// ---------------------------------------------------------------------------
// Server implementation
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ParquetMcp;

impl ServerHandler for ParquetMcp {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "forge-parquet-mcp".into(),
                version: option_env!("APP_VERSION")
                    .unwrap_or(env!("CARGO_PKG_VERSION"))
                    .into(),
                title: Some("Forge Parquet & Git Inode MCP".into()),
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

fn into_result(r: Result<String, String>) -> CallToolResult {
    match r {
        Ok(text) => CallToolResult::success(vec![Content::text(text)]),
        Err(err) => CallToolResult::error(vec![Content::text(err)]),
    }
}

fn into_async_result(
    f: impl Future<Output = Result<String, String>> + Send + 'static,
) -> BoxFuture<'static, Result<CallToolResult, rmcp::ErrorData>> {
    async move {
        let r = f.await;
        Ok(into_result(r))
    }
    .boxed()
}

fn opt_arg<'a>(
    ctx: &'a ToolCallContext<'_, ParquetMcp>,
    name: &str,
) -> Option<&'a str> {
    ctx.arguments
        .as_ref()
        .and_then(|args| args.get(name))
        .and_then(|v| v.as_str())
}

fn opt_arg_u64<'a>(
    ctx: &'a ToolCallContext<'_, ParquetMcp>,
    name: &str,
) -> Option<u64> {
    ctx.arguments
        .as_ref()
        .and_then(|args| args.get(name))
        .and_then(|v| v.as_u64())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("forge_parquet_mcp=info".parse().unwrap())
                .add_directive("rmcp=warn".parse().unwrap()),
        )
        .without_time()
        .init();

    tracing::info!("starting forge-parquet-mcp server");

    let server = ParquetMcp;

    // ── parquet_scan_git_inodes ─────────────────────────────────────────────
    let scan_schema: Arc<JsonObject> = Arc::new(
        json!({
            "type": "object",
            "properties": {
                "repo_path": {
                    "type": "string",
                    "description": "Path to the git repository to scan"
                },
                "max_files": {
                    "type": "integer",
                    "description": "Maximum files to scan (optional, default: unlimited)"
                }
            },
            "required": ["repo_path"],
            "additionalProperties": false
        })
        .as_object()
        .cloned()
        .unwrap(),
    );

    let scan_inodes_tool = ToolRoute::new_dyn(
        Tool::new(
            "parquet_scan_git_inodes",
            "Walk a git repository, detect all inodes, classify by type (pack, idx, object, regular), and shard by inode%71. Returns shard distribution and type counts.",
            scan_schema,
        )
        .annotate(ToolAnnotations::new().read_only(true)),
        move |ctx: ToolCallContext<'_, ParquetMcp>| {
            let repo = opt_arg(&ctx, "repo_path").unwrap_or(".").to_string();
            let max = opt_arg_u64(&ctx, "max_files").map(|m| m as usize);
            into_async_result(async move {
                let result = tokio::task::spawn_blocking(move || scan_git_inodes(&repo, max))
                    .await
                    .map_err(|e| format!("Task join error: {e}"))?;
                result.map(|r| serde_json::to_string_pretty(&r).unwrap_or_default())
            })
        },
    );

    // ── parquet_inspect_file ────────────────────────────────────────────────
    let inspect_schema: Arc<JsonObject> = Arc::new(
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the parquet file to inspect"
                }
            },
            "required": ["file_path"],
            "additionalProperties": false
        })
        .as_object()
        .cloned()
        .unwrap(),
    );

    let inspect_file_tool = ToolRoute::new_dyn(
        Tool::new(
            "parquet_inspect_file",
            "Inspect a parquet file: show file size, metadata, and basic info",
            inspect_schema,
        )
        .annotate(ToolAnnotations::new().read_only(true)),
        move |ctx: ToolCallContext<'_, ParquetMcp>| {
            let fp = opt_arg(&ctx, "file_path").unwrap_or("").to_string();
            into_async_result(async move {
                let result = tokio::task::spawn_blocking(move || inspect_parquet_file(&fp))
                    .await
                    .map_err(|e| format!("Task join error: {e}"))?;
                result
            })
        },
    );

    // ── parquet_find_writers ────────────────────────────────────────────────
    let writers_schema: Arc<JsonObject> = Arc::new(
        json!({
            "type": "object",
            "properties": {
                "search_dir": {
                    "type": "string",
                    "description": "Directory to search for parquet-writing Rust code"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results (optional, default: 20)"
                }
            },
            "required": ["search_dir"],
            "additionalProperties": false
        })
        .as_object()
        .cloned()
        .unwrap(),
    );

    let find_writers_tool = ToolRoute::new_dyn(
        Tool::new(
            "parquet_find_writers",
            "Search a directory tree for Rust source files that write parquet data (contain SerializedFileWriter, write_parquet, etc.)",
            writers_schema,
        )
        .annotate(ToolAnnotations::new().read_only(true)),
        move |ctx: ToolCallContext<'_, ParquetMcp>| {
            let dir = opt_arg(&ctx, "search_dir").unwrap_or(".").to_string();
            let max = opt_arg_u64(&ctx, "max_results")
                .map(|m| m as usize)
                .unwrap_or(20);
            let dir_for_closure = dir.clone();
            into_async_result(async move {
                let result = tokio::task::spawn_blocking(move || find_parquet_writers(&dir_for_closure, Some(max)))
                    .await
                    .map_err(|e| format!("Task join error: {e}"))?;
                Ok(json!({
                    "search_dir": dir,
                    "total_found": result.len(),
                    "results": result
                })
                .to_string())
            })
        },
    );

    // ── parquet_list_tools ──────────────────────────────────────────────────
    let list_tools_tool = ToolRoute::new_dyn(
        Tool::new(
            "parquet_list_tools",
            "List available parquet-index and tmux parquet tool binaries/scripts available on disk",
            Arc::new(json!({}).as_object().cloned().unwrap()),
        )
        .annotate(ToolAnnotations::new().read_only(true)),
        move |_ctx: ToolCallContext<'_, ParquetMcp>| {
            into_async_result(async move {
                let tmux_tools = tokio::task::spawn_blocking(list_tmux_tools)
                    .await
                    .map_err(|e| format!("Task join error: {e}"))?;
                let zos_tools = tokio::task::spawn_blocking(list_zos_parquet_tools)
                    .await
                    .map_err(|e| format!("Task join error: {e}"))?;
                Ok(json!({
                    "tmux_tools_dir": TMUX_TOOLS_DIR,
                    "tmux_tools": tmux_tools,
                    "zos_parquet_index_dir": ZOS_PARQUET_INDEX_DIR,
                    "zos_parquet_index_scripts": zos_tools
                })
                .to_string())
            })
        },
    );

    // ── Assemble router ─────────────────────────────────────────────────────
    let router = Router::new(server)
        .with_tool(scan_inodes_tool)
        .with_tool(inspect_file_tool)
        .with_tool(find_writers_tool)
        .with_tool(list_tools_tool);

    // Serve over stdio
    let running = router
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;

    // Keep the event loop alive until stdin closes
    running.waiting().await?;

    tracing::info!("server stopped");
    Ok(())
}
