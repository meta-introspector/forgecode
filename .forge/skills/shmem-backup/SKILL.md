---
name: shmem-backup
description: >-
  Complete IPLD CAR shared memory backup pipeline: dump to disk,
  PostgreSQL import with full-text search, and Tantivy inverted index.
  Orchestrates 188K+ blocks across three persistence layers.
  Use when setting up or maintaining the DASL shmem backup infrastructure.
---

# shmem-backup — Complete Shmem Backup Pipeline

## one-liner
```bash
python3 ~/bin/shmem2disk dump --dir ~/shmem-backup/
```

## Architecture

```
  IPLD CAR Shmem (188K blocks, 2GB)
    @ipld_car_shmem unix socket
           │
           ▼ dump
  ~/shmem-backup/  (CID-named .car files)
     ├── 01/
     ├── 02/
     ├── ...
     └── index.json
           │
     ┌─────┴─────┐
     ▼            ▼
  PostgreSQL    Tantivy
  (SQL+fts)     (inverted idx)
```

## Full Pipeline

```bash
# 1. Dump shmem to disk
shmem2disk dump --dir ~/shmem-backup/

# 2. PostgreSQL
createdb dasl_shmem
shmem2disk pg-create --db dasl_shmem
shmem2disk pg-import --dir ~/shmem-backup/ --db dasl_shmem

# 3. Tantivy
shmem2disk tantivy-index --dump-dir ~/shmem-backup/ --index-dir ~/shmem-tantivy/

# 4. Query
shmem2disk pg-stats --db dasl_shmem
shmem2disk tantivy-search "preview:theorem AND is_proof:true"
```

## Query Examples

### PostgreSQL
```sql
-- Find all Lean proofs
SELECT cid, preview FROM shmem_blocks WHERE is_proof = true AND block_type = 'lean_source';

-- Full-text search
SELECT cid, preview FROM shmem_blocks 
WHERE search_vector @@ to_tsquery('Monster & theorem');

-- By codec
SELECT codec, COUNT(*) FROM shmem_blocks GROUP BY codec;

-- Largest blocks
SELECT cid, block_type, pg_size_pretty(size_bytes::bigint) 
FROM shmem_blocks ORDER BY size_bytes DESC LIMIT 10;
```

### Tantivy
```bash
shmem2disk tantivy-search "block_type:lean_source AND is_proof:true"
shmem2disk tantivy-search "preview:Monster AND block_type:lean_source"
shmem2disk tantivy-search "cid:01551220* AND block_type:rust_source"
```

## Files
- `~/bin/shmem2disk` — Python orchestration
- `~/shmem-backup/` — block dump directory (~2GB)
- `~/shmem-tantivy/` — Tantivy index directory
- `ipld-car-ipc-shmem-linux/src/bin/tantivy-indexer.rs` — Rust indexer

## See Also
- [[skills/shmem2disk]]
- [[skills/tantivy-indexer]]
- [[skills/locate2shmem]]
- [[skills/dasl-atomize]]

## Shmem Cross-References

> Generated: 2026-06-23 10:20:03 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| CID | computeCID | def |
| Complete | meme_MONSTER_WALK_INDEX_COMPLETE | theorem |
| Complete | [CompleteSpace | instance |
| Dump | dumpEnv | def |
| Monster | monsterPrime | def |
| Monster | meme_clifford_fractran_monster | theorem |
| Monster | meme_MONSTER_WALK_INDEX_COMPLETE | theorem |