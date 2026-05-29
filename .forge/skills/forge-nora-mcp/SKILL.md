---
name: forge-nora-mcp
description: Query the Nora self-hosted artifact registry for crate search, crate details,
  and registry status. Nora runs at http://127.0.0.1:4000 and serves the cargo registry
  protocol (sparse index, API v1). Use when you need to: (1) search for crates in the
  local Nora registry, (2) get details about a specific crate including versions and
  metadata, (3) check if the Nora registry is healthy and get its configuration, (4)
  verify crate availability before running nix builds. Requires forge-nora-mcp registered
  in .mcp.json (stdio transport).
---

# Forge Nora MCP

Registered as `nora` in `.mcp.json` under `mcpServers`. Uses stdio transport —
Forge spawns it as a child process. Wraps Nora's HTTP API (`http://127.0.0.1:4000`).

## Available Tools

| Tool | Description |
|------|-------------|
| `nora_registry_info` | Get Nora registry configuration (cargo index config) |
| `nora_get_crate` | Get detailed info for a specific crate by name |
| `nora_search_crates` | Search crates by query string (supports `query`, `limit` params) |
| `nora_registry_status` | Health check: returns OK or error if Nora is unreachable |

## Usage Examples

```
Check registry health:
→ "is nora running?"
→ "check the nora registry status"

Search crates:
→ "search nora for crates named forge"
→ "find crates in nora matching 'serde'"
→ "search for the top 5 crates matching 'ipld'"

Get crate info:
→ "show me the details of the forge crate in nora"
→ "what versions of serde are in nora?"
→ "get crate info for tokio"

Registry config:
→ "what protocols does nora support?"
→ "show the nora registry config"
```

## Architecture

- Stdio-based MCP server (child process of Forge)
- Periodically queries Nora HTTP API at port4000
- All tools are read-only (no publish/destroy operations)
- Nora supports 13 registry protocols: cargo, docker, npm, pypi, maven, go, nuget, gems, pub, conan, terraform, ansible, raw
