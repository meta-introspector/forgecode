---
name: plocate
description: Fast indexed file search using plocate at /usr/bin/plocate. Use this instead of `grep -r` or `find` for path-based file lookups. Falls back to `fd` for real-time filesystem traversal.
---

# plocate — Indexed File Search

Instead of `find` or `grep -r` (both denied), use `plocate` for fast indexed file searches across the entire filesystem.

plocate is at `/usr/bin/plocate` (version 1.1.15). It searches a pre-built index updated by the system's `updatedb` service.

## Basic Usage

```bash
plocate <pattern>          # search for any path containing <pattern>
plocate -b <name>          # basename match (file name portion only)
plocate -c <pattern>       # count matches instead of listing them
plocate -d <dbpath>        # use a specific database file
```

## Options

| Flag | Description |
|------|-------------|
| `-b, --basename` | Search only the file name portion of paths |
| `-c, --count` | Print number of matches instead of the paths |
| `-d, --database DBPATH` | Search in a specific database |
| `-i` | Case-insensitive search |
| `-l, --limit N` | Limit output to N matches |
| `-q` | Quiet mode (no error output) |
| `-r, --regex` | Interpret pattern as a regex |
| `-w, --wholename` | Match whole path name (default) |

## Examples

```bash
plocate 'Cargo.toml'                 # find all Cargo.toml files
plocate -b 'pipelight.ts'            # find files named pipelight.ts
plocate -c '\.rs$'                   # count all .rs files
plocate -i 'nginx' 'config'          # case-insensitive, both patterns present
plocate -r 'forge.*policy'           # regex: forge followed by policy
```

## When to Use What

| Tool | Use Case |
|------|----------|
| `findo` | Instant search of 2M+ indexed files (Parquet DB) |
| `plocate` | General indexed filesystem search via mlocate DB |
| `fd` | Real-time file traversal (not indexed, but live filesystem) |

## Related

- `findo` skill — instant Parquet-indexed file search
- Permissions deny `find *` and `grep *` — use this skill instead
