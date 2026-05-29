---
name: forge-parquet-mcp
title: Forge Parquet & Git Inode MCP
description: MCP server for scanning git repository inodes, inspecting parquet files, and finding parquet-writing Rust code
---

# Forge Parquet & Git Inode MCP

MCP server exposing tools for git inode analysis and parquet file operations. Runs as a stdio subprocess, registered in `.mcp.json`.

## Quick Start

```bash
# Run directly
nix run .#forge-parquet-mcp

# Or via devShell
nix develop
forge-parquet-mcp
```

## Tools

| Tool | Description | Arguments |
|------|-------------|-----------|
| `parquet_scan_git_inodes` | Walk a git repo, detect inode types (pack, idx, object, regular), shard by inode%71 | `repo_path` (required), `max_files` (optional) |
| `parquet_inspect_file` | Show file metadata and basic info for a parquet file | `file_path` (required) |
| `parquet_find_writers` | Search a directory for Rust code that writes parquet | `search_dir` (required), `max_results` (optional, default 20) |
| `parquet_list_tools` | List available parquet-index tool binaries on disk | none |

## Inode Sharding Scheme

Files are classified into 4 types:
- **pack** — `.pack` files (git packfiles)
- **idx** — `.idx` files (git pack indexes)
- **object** — files matching `.git/objects/XX/SHA` pattern (loose git objects)
- **regular** — everything else

Sharding uses `inode % 71` (the 71 shards correspond to the monster group prime factors + 1 aether shard).

## Reference Files

- **zos-server parquet-index**: `/mnt/data1/nix/time/2024/12/10/swarms-terraform/services/submodules/zos-server/plugins/parquet-index/`
- **tmux pre-built binaries**: `/mnt/data1/nix/vendor/tmux/target/release/`
