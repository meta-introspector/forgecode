---
name: use-github-mcp
description: Search and interact with GitHub via the GitHub MCP Server (github-mcp-server). Use when users need to (1) search GitHub for code, repositories, issues, PRs, or users, (2) find existing MCP servers or tools in the GitHub ecosystem, (3) query the running github-mcp-server at http://127.0.0.1:8082, (4) search crates.io or other registries, or (5) discover MCP servers for specific domains (Forgejo, PostgreSQL, Parquet, DAG-CBOR, etc.).
---

# Use GitHub MCP Server

The `github-mcp-server` is a Go-based MCP server (23k+ stars) by GitHub that exposes 41+ GitHub API tools over MCP protocol, including search. It runs as a systemd service managed by system-manager on port 8082 (HTTP/SSE transport).

## Architecture

```
Client (you) --HTTP/SSE--> github-mcp-server (port 8082, systemd)
  Bearer token from gh auth token
```

Auth is handled via `Authorization: Bearer` header with a token obtained from `gh auth token`. The wrapper script in the systemd service injects the token automatically on startup.

## Interacting with the MCP Server

### Protocol: HTTP/SSE (Server-Sent Events)

The server uses SSE — each request returns `event: message\ndata: {json}\n\n`. You send JSON-RPC over HTTP POST and receive SSE-streamed responses.

### Step 1: Initialize

```bash
TOKEN=$(gh auth token)
curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"client","version":"1.0"}}}' \
  http://127.0.0.1:8082/
```

Response is SSE: `event: message\ndata: {"jsonrpc":"2.0","id":1,"result":{...}}`

### Step 2: List Available Tools

```bash
curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  http://127.0.0.1:8082/ | grep -oP '"name":"[^"]+"'
```

### Step 3: Call a Tool

```bash
curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"TOOL_NAME","arguments":{...}}}' \
  http://127.0.0.1:8082/
```

## Available Search Tools (5 tools)

| Tool | Purpose | Key Arguments |
|------|---------|---------------|
| `search_code` | Search source code | `query`, `language`, `limit` |
| `search_repositories` | Search repos | `query`, `limit` |
| `search_issues` | Search issues | `query`, `limit` |
| `search_users` | Search users | `query`, `limit` |
| `search_pull_requests` | Search PRs | `query`, `limit` |

### GitHub Search Query Syntax

Use standard GitHub search syntax:
- `filename:foo.rs` — search in files named foo.rs
- `path:/src/` — search within specific paths
- `language:rust` — filter by language
- `repo:owner/name` — scope to a repo
- `user:name` — scope to a user
- `extension:rs` — filter by file extension
- `AND`, `OR`, `NOT` operators
- Quotes for exact phrases: `"mcp server"`

Examples:
```json
{"query": "mcp-server forgEjo", "limit": 5}
{"query": "filename:Cargo.toml serde", "limit": 10}
{"query": "org:mcp-servers", "limit": 5}
```

## Available Issue/PR Tools (18+ tools)

| Tool | Purpose |
|------|---------|
| `issue_read` | Read an issue |
| `issue_write` | Create/update issues |
| `list_issues` | List issues |
| `pull_request_read` | Read a PR |
| `list_pull_requests` | List PRs |
| `create_pull_request` | Create a PR |
| `merge_pull_request` | Merge a PR |
| `update_pull_request` | Update a PR |
| `add_issue_comment` | Add comment to issue |
| `add_comment_to_pending_review` | Comment on a review |
| `add_reply_to_pull_request_comment` | Reply to PR comment |
| `pull_request_review_write` | Submit PR review |
| `request_copilot_review` | Request Copilot PR review |

## Available File/Repo Tools (9+ tools)

| Tool | Purpose |
|------|---------|
| `create_branch` | Create a branch |
| `create_or_update_file` | Create/update a file |
| `delete_file` | Delete a file |
| `get_file_contents` | Read file contents |
| `push_files` | Push multiple files |
| `fork_repository` | Fork a repo |
| `create_repository` | Create a repo |
| `list_branches` | List branches |
| `list_commits` | List commits |

## Available Release/Team Tools (8+ tools)

| Tool | Purpose |
|------|---------|
| `get_commit` | Get commit details |
| `get_latest_release` | Get latest release |
| `get_release_by_tag` | Get release by tag |
| `list_releases` | List releases |
| `list_tags` | List tags |
| `get_tag` | Get tag details |
| `get_team_members` | List team members |
| `get_teams` | List teams |

## Discovering MCP Servers in the Ecosystem

Use `search_code` and `search_repositories` to find existing MCP servers before building one:

1. **Search repositories**: `'mcp-server <domain>'` — finds projects like `raohwork/forgejo-mcp`, `bytebase/dbhub` (Postgres)
2. **Search code**: `'mcp AND <technology>'` — finds code references to MCP integrations
3. **Filter by language**: Use `+language:rust` or `language:go` to narrow

### Known MCP Server Ecosystem

| Domain | Best Option | Language | Stars | Notes |
|--------|-------------|----------|-------|-------|
| **GitHub** | `github/github-mcp-server` | Go | 23k+ | Official, 41 tools |
| **Forgejo** | `raohwork/forgejo-mcp` | Go | 55 | 103 tools |
| **PostgreSQL** | `bytebase/dbhub` | TS | 2.8k | Multi-DB, zero-dep |
| **Parquet** | `unravel-team/mcp-analyst` | Python | 18 | CSV + Parquet |
| **DAG-CBOR/IPLD** | None available | — | — | Must build from scratch |

## Handling SSE Responses

The server returns SSE format. Extract JSON with:
```bash
curl ... | grep '^data: ' | sed 's/^data: //' | python3 -m json.tool
```

Or parse programmatically:
```python
import sys, json
for line in sys.stdin:
    if line.startswith('data: '):
        data = json.loads(line[6:])
        if 'result' in data:
            # handle result
```

## Common Patterns

### Search + Extract Name/Description

```python
# From search_repositories results
data = json.loads(text)
for r in data.get('items', []):
    print(f'{r["full_name"]}  ({r.get("language","?")})')
    print(f'  {r["description"][:100]}')
    print(f'  {r["html_url"]}')
```

### Count Tools Available

```bash
curl ... -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' .../ | \
  grep -oP '"name":"[^"]+"' | wc -l
```

## Forge MCP Config

The project `.mcp.json` registers the HTTP/SSE endpoint:

```json
{
  "mcpServers": {
    "github": {
      "url": "http://127.0.0.1:8082/"
    }
  }
}
```

Using `url` mode (not `command` mode) because the server runs as a persistent systemd daemon.
