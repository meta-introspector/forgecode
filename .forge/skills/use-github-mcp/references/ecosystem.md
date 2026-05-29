# MCP Server Ecosystem Reference

Known MCP servers discovered via GitHub search, organized by domain.

## GitHub

| Repo | Lang | Stars | Tools |
|------|------|-------|-------|
| `github/github-mcp-server` | Go | 23k+ | 41 (search, issues, PRs, files) |

## Forgejo

| Repo | Lang | Stars | Notes |
|------|------|-------|-------|
| `raohwork/forgejo-mcp` | Go | 55 | 103 tools (repos, issues, PRs, orgs) |
| `goern/forgejo-mcp` | Go | 71 | Mirror of upstream |
| `Sqcows/forgejo-mcp` | TypeScript | 6 | 103 tools |
| `nsvk13/forgejo-mcp-server` | JavaScript | 7 | Claude Desktop integration |

## PostgreSQL / Databases

| Repo | Lang | Stars | Notes |
|------|------|-------|-------|
| `bytebase/dbhub` | TypeScript | 2,851 | Zero-dep, Postgres+MySQL+SQLite+SQL Server |
| `pgEdge/pgedge-postgres-mcp` | Go | 171 | PostgreSQL MCP with NL agent CLI |
| `subnetmarco/pgmcp` | Go | — | Minimal Go Postgres MCP |
| `googleapis/mcp-toolbox` | Go | 15,378 | Multi-database (BigQuery, etc.) |

## Parquet / Data Analysis

| Repo | Lang | Stars | Notes |
|------|------|-------|-------|
| `unravel-team/mcp-analyst` | Python | 18 | CSV + Parquet analysis |
| `markmhendrickson/mcp-server-parquet` | Python | 0 | Minimal |
| `MiguelAzevedoHS/MCP4Parquet` | Java | 0 | Uses DuckDB |

## DAG-CBOR / IPLD

No dedicated MCP servers exist. Must build from scratch using `libipld-core` + `rmcp` (Rust).
