---
name: locate2shmem
description: >-
  Run locate to find files, warm them into IPLD CAR shmem, auto-summarize,
  and generate SPARQL relation triples. Use when bulk-loading files into shmem.
---

# locate2shmem — locate → chunk → warm → summarize → relate

## one-liner
```bash
python3 ~/bin/locate2shmem --from-index ~/2026/06-june/26/index/lean.index2.txt --limit 500
```

## Trigger
When bulk-loading files into IPLD CAR shared memory, building corpus indices,
or cross-referencing file content with existing shmem blocks.

## Phases
1. **locate** — find files by pattern or from pre-built index
2. **chunk** — group files by size (100MB default)
3. **warm** — hash each file → CID (0x55 raw)
4. **summarize** — auto-detect file type (Lean/Rust/Python/JSON/MD), extract metadata
5. **relate** — generate SPARQL triples linking chunk CIDs to existing shmem

## Usage
```bash
locate2shmem "*.lean" --limit 100
locate2shmem --from-index ~/2026/06-june/26/index/lean.index2.txt --limit 1000
locate2shmem "*.rs" --exclude target/ --max-size 65536
```

## Output
- `/tmp/locate2shmem-meta-<N>.json` — chunk metadata + CID list
- `/tmp/locate2shmem-rels-<N>.ttl` — SPARQL relation triples
- Meta-blocks stored in shmem as CIDs

## See Also
- [[skills/shmem2disk]]
- [[skills/dasl-atomize]]

## Shmem Cross-References

> Generated: 2026-06-23 10:19:59 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| CID | computeCID | def |