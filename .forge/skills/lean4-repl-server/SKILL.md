---
name: lean4-repl-server
description: IPLD CAR shared-memory REPL server for Lean4 declarations. Accepts S-expressions and pre-parsed JSON, stores declarations content-addressed in shmem, supports fuzzy search and similarity queries. Use when querying declaration corpus, searching for similar declarations, or storing/retrieving formalized content.
---

# lean4-repl-server

## one-liner
```bash
curl -X POST http://127.0.0.1:8156/repl -d '(health)'
```

## Trigger
When working with declaration storage, shmem interaction, REPL queries,
or the DASL block server architecture.

## Setup
```bash
# Deployed via system-manager:
cd ~/projects/system-manager
sudo system-manager switch --flake .#all-services
```

## Steps

- [x] Rust binary `lean4-repl` on port 8156
- [x] S-expression commands: `(load)`, `(eval)`, `(search)`, `(inspect)`, `(missing)`, `(context)`, `(health)`
- [x] IPLD CAR shmem integration via `CarShmemClient::put_block`
- [x] Content-addressed declaration storage (CID per declaration)
- [x] Import tracking and missing dependency detection
- [x] Fuzzy search across loaded declarations
- [ ] `/put` endpoint — accept pre-parsed JSON from staticsplitjson (no shelling out)
- [ ] `/load` endpoint — accept pre-parsed JSON (not raw .lean needing lake)
- [ ] Full corpus population from Aristotle output
- [ ] `(compile-tool)` — compile .lean tool → store binary in shmem
- [ ] `(apply-tool)` — fetch tool from shmem, run on argument

## Files
- `~/dasl/ipld-car-ipc-shmem-linux/src/bin/lean4-repl.rs`
- `~/projects/system-manager/all-services.nix`

## Endpoints
| Method | Path | Description |
|--------|------|-------------|
| POST | `/repl` | S-expression commands |
| POST | `/load` | Load .lean file (currently shells out to lake) |
| GET | `/search?q=` | Fuzzy search loaded declarations |
| GET | `/context` | Current session context |
| GET | `/health` | Health check |
| GET | `/missing` | Missing dependency list |

## Shmem Cross-References

> Generated: 2026-06-23 10:19:59 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| CID | computeCID | def |