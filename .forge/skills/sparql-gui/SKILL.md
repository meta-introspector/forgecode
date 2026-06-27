---
name: sparql-gui
description: >-
  🔍 SPARQL GUI — DASL package
---

# 🔍 SPARQL GUI

**Package:** `sparql-gui`
**Completeness:** 33% (2/11)
**Priority:** immediate

## Status

| 🪞 Git mirror | 📋 planned |
| 🌲 Worktree | ✅ done |
| 🌿 Branch | 🔧 wip |
| 🧠 Skill | ✅ done |
| 📦 Nora pkg | — na |
| ❄️ Nix flake | 📋 planned |
| ⚙️ System-manager | 📋 planned |
| 🌐 Nginx proxy | 📋 planned |
| 🧩 Tile GUI | 📋 planned |
| 🎛️ Admin tile | 📋 planned |
| 📝 Documentation | 🔧 wip |

## Missing Dimensions

- 🪞 **Git mirror**: Create bare mirror
- ❄️ **Nix flake**: Create flake.nix with crane + omaster refs
- ⚙️ **System-manager**: Create system-manager config
- 🌐 **Nginx proxy**: Add nginx reverse proxy
- 🧩 **Tile GUI**: Build tile GUI
- 🎛️ **Admin tile**: Add /admin endpoints

## Actions

- `git clone --mirror <url> /mnt/data1/git/.../<name>.git`
- `nix flake init && add crane + omaster inputs`
- `create system-manager-config.nix with systemd service`
- `add location block to /etc/nginx/locations.d/<name>.conf`
- `create interactive HTML + JSON API endpoints`
- `add /admin/status, warm triggers, cache control`

## Related

- System blueprint: `/blueprint/`
- Admin panel: `/admin/`

## Shmem Cross-References

> Generated: 2026-06-23 10:20:04 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| — | No matches in shmem for 6 keywords | — |
| — | Searchable terms: Actions, Dimensions, GUI, Missing, Related, SPARQL | — |