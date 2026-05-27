---
name: findo
description: Query 2M+ files from a Parquet index instantly using the findo binary at /home/mdupont/bin/findo. Use this instead of `find` or `grep` commands, which are now denied by the permission system.
---

# findo — Instant File Search via Parquet Index

Instead of running `find` or `grep` (both denied), use `findo` to search the indexed filesystem at `/mnt/data1/time-2026/cache/smart-scan.parquet`.

## One or More Patterns

```bash
findo rust          # all paths containing "rust" (case-insensitive)
findo rust cargo    # paths containing BOTH "rust" AND "cargo"
```

## Options

| Flag | Description |
|------|-------------|
| `-l, --limit <N>` | Limit results per source (default: 100) |
| `-f, --files` | Search only file entries (not messages) |
| `-m, --messages` | Search only message entries |
| `-r, --raw` | Raw output — paths only, no headers |

## Examples

```bash
findo -r 'findo'
findo -l 10 'nixpkgs' 'certbot'
findo -f 'forge' 'src'
findo -m 'error' 'build'
```

**When `findo` is insufficient** (e.g., the path is not indexed or you need regex), fall back to `plocate` or `fd`:
- `plocate <pattern>` — indexed by plocate daemon
- `fd <pattern>` — real-time filesystem search

## Related

- `plocate` skill — indexed file search using mlocate database
- Permissions deny `find *` and `grep *` — use this skill instead
