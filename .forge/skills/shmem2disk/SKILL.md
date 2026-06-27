---
name: shmem2disk
description: >-
  Backup IPLD CAR shared memory blocks to disk with CID-named files,
  import into PostgreSQL with full-text search, and build Tantivy
  inverted indexes for sub-millisecond search. Use when persisting
  shmem state or building queryable block databases.
---

# shmem2disk — dump → PostgreSQL → Tantivy

## one-liner
```bash
python3 ~/bin/shmem2disk dump --dir ~/shmem-backup/ --limit 100
```

## Trigger
When backing up IPLD CAR shmem to persistent storage or building
queryable indexes (SQL full-text + Tantivy inverted index).

## Commands

| Command | What |
|---------|------|
| `dump` | Dump all shmem blocks to disk (CID-named .car files + index.json) |
| `pg-create` | Create PostgreSQL schema with tsvector full-text search |
| `pg-import` | Import dumped blocks into PostgreSQL |
| `pg-stats` | Show import statistics |
| `tantivy-index` | Build Tantivy inverted index (Rust binary, sub-ms search) |
| `tantivy-search` | Query Tantivy index |
| `stats` | Show shmem server stats |

## PostgreSQL Schema
- `shmem_blocks` — cid, codec, hash_digest, size_bytes, block_type, is_proof, json_keys, preview, content, search_vector
- `shmem_relations` — source_cid, target_cid, rel_type, metadata
- `shmem_search` — cid, token, position

## Tantivy Index Fields
- `cid` (exact match), `block_type` (faceted), `preview` (full-text)
- `is_proof` (boolean), `size_bytes` (numeric), `json_keys` (tokenized)

Example queries:
```
block_type:lean_source AND is_proof:true
preview:Monster AND block_type:lean_source
codec:0x55 AND preview:CarShmemClient
```

## Usage
```bash
# Full pipeline
shmem2disk dump --dir ~/shmem-backup/           # ~188K blocks → disk
shmem2disk pg-create --db dasl_shmem             # Create DB schema
shmem2disk pg-import --dir ~/shmem-backup/ --db dasl_shmem  # SQL import
shmem2disk tantivy-index --dump-dir ~/shmem-backup/ --index-dir ~/shmem-tantivy/
shmem2disk tantivy-search "preview:Monster AND is_proof:true"

# Quick stats
shmem2disk stats
```

## Files
- `~/bin/shmem2disk` — Python orchestration
- `ipld-car-ipc-shmem-linux/src/bin/tantivy-indexer.rs` — Rust Tantivy indexer
- `~/shmem-backup/` — block dump directory
- `~/shmem-tantivy/` — Tantivy index directory

## See Also
- [[skills/locate2shmem]]
- [[skills/dasl-atomize]]
- [[skills/ipld-car-shmem]]
- [[skills/sparql-gui-tile]]

## Shmem Cross-References

> Generated: 2026-06-23 10:20:03 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| CID | computeCID | def |
| Index | ko_class_index | def |
| Index | meme_MONSTER_WALK_INDEX_COMPLETE | theorem |
| Index | bott_orbit_rotates_ko_index | theorem |
| Monster | monsterPrime | def |
| Monster | meme_clifford_fractran_monster | theorem |
| Monster | meme_MONSTER_WALK_INDEX_COMPLETE | theorem |