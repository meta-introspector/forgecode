---
name: forge-pipelight-mcp
description: Manage CI/CD pipeline builds using the Pipelight MCP server (forge-pipelight-mcp).
  Provides tools to check build status, list pipeline configs, view logs, and trigger
  pipeline runs. Use when you need to: (1) check ongoing build status, (2) list available
  pipeline configurations, (3) view build logs for debugging, (4) trigger a new pipeline
  run. Requires forge-pipelight-mcp registered in .mcp.json (stdio transport).
---

# Forge Pipelight MCP

Registered as `pipelight` in `.mcp.json` under `mcpServers`. Uses stdio transport —
Forge spawns it as a child process.

## Available Tools

| Tool | Description |
|------|-------------|
| `pipelight_status` | Get current pipeline build status (read-only) |
| `pipelight_list` | List available pipeline configurations with status (read-only) |
| `pipelight_logs` | Fetch recent build logs. Optional: `pipe` (name filter), `branch` (branch filter) |
| `pipelight_run` | Trigger a new pipeline run. Optional: `pipe`, `branch` |

## Usage Examples

```
Check build status:
→ "check the pipelight build status"
→ "what's the current pipeline state?"

List pipelines:
→ "list available pipelight pipelines"
→ "show me the pipeline configs"

View logs:
→ "show me recent build logs"
→ "get logs for the deploy pipeline"

Trigger a run:
→ "run the deploy pipeline"
→ "trigger a build on main branch"
```

## Architecture

- Stdio-based MCP server (child process of Forge)
- Wraps the `pipelight` CLI binary
- 2 read-only tools (status, list)
- 2 filtered read tools (logs, run)
- Supports optional `pipe` and `branch` arguments on logs and run tools
