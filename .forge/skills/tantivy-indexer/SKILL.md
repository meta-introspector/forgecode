---
name: tantivy-indexer
description: >-
  Rust binary that builds a Tantivy inverted index from shmem block dumps.
  Indexes CID, block_type, preview (full-text), is_proof, and size_bytes
  for sub-millisecond search. Use when building search indexes over
  shmem block metadata or running full-text queries against the DASL corpus.
---

# tantivy-indexer — Rust full-text index for shmem blocks

## one-liner
```bash
tantivy-indexer ~/shmem-backup/ ~/shmem-tantivy/
```

## Trigger
When indexing shmem dump blocks for fast full-text search.

## Build
```bash
cd /mnt/data1/time-2026/02-february/22/dasl/ipld-car-ipc-shmem-linux
cargo build --release --bin tantivy-indexer
```

## Fields indexed
- **cid**: stored + indexed (exact match, STRING)
- **block_type**: stored + indexed (faceted: lean_source, rust_source, json, etc.)
- **codec**: stored + indexed (faceted: 0x55 raw, 0x71 dag-cbor)
- **preview**: indexed + tokenized (full-text, with positions)
- **json_keys**: indexed + tokenized
- **is_proof**: stored + indexed (boolean filter)
- **size_bytes**: stored (numeric, sortable)
- **digest_short**: stored
- **proof_hint**: stored

## Usage
```bash
# Build index from dump
tantivy-indexer ~/shmem-backup/ ~/shmem-tantivy/

# Search
tantivy-indexer search ~/shmem-tantivy/ "preview:Monster AND is_proof:true"
tantivy-indexer search ~/shmem-tantivy/ "block_type:rust_source AND preview:CarShmemClient"
tantivy-indexer search ~/shmem-tantivy/ "cid:01551220* AND block_type:lean_source"
```

## Output
- `~/shmem-tantivy/meta.json` — index metadata
- `~/shmem-tantivy/*.idx` — Tantivy segment files
- Query results: CID, block_type, preview (first 100 chars)

## See Also
- [[skills/shmem2disk]]
- [[skills/locate2shmem]]
- [[skills/ipld-car-shmem]]

## Shmem Cross-References

> Generated: 2026-06-23 10:20:05 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Build | buildPrefixTree | def |
| CID | computeCID | def |
| Monster | monsterPrime | def |
| Monster | meme_clifford_fractran_monster | theorem |
| Monster | meme_MONSTER_WALK_INDEX_COMPLETE | theorem |
| Rust | meme_emoji_dao_rust | theorem |