---
name: dasl-atomize
description: >-
  Split source files into atoms (file, decl, name, type, token, import),
  each content-addressed via SHA256 CID (0x55 raw). Builds dependency DAG
  and computes minimal closure from spec root declarations. Use when
  building the atom-CID lattice or generating SPARQL triples for the
  DASL knowledge graph.
---

# dasl-atomize — split → tokenize → CID → minimize

## one-liner
```bash
python3 ~/bin/dasl-atomize ~/projects/arist/splitter-engine/RequestProject/Lean4Introspector.lean \
  --roots collectRefs constDeps filterDeps bfsClosure
```

## Trigger
When building the DASL atom-CID lattice, computing minimal representations,
or generating SPARQL triples for the shmem knowledge graph.

## Atom types

| Atom | Kind | CID content | Example |
|------|------|-------------|---------|
| file | `file` | Raw file bytes | `01551220<sha256(file)>` |
| decl | `decl` | Source block + metadata | `01551220<sha256(block)>` |
| name | `name` | Declaration name string | `01551220<sha256("collectRefs")>` |
| type | `type` | Type expression | `01551220<sha256("Expr → NameSet")>` |
| token | `token` | Individual identifier | `01551220<sha256("collectRefs")>` |
| import | `import` | Import edge (from→to) | `01551220<sha256("import X")>` |

## Minimal closure
From a set of root declaration names, computes the transitive closure
of all atoms reachable via dependency edges. This is the minimal
representation needed to prove the roots.

## Usage
```bash
# Atomize a single file
dasl-atomize file.lean --roots CID Monster load_bearing_myth

# Atomize from T1 corpus
dasl-atomize --from-corpus corpus-scan.jsonl --limit 1000 --roots CID Monster Tile

# Generate SPARQL
dasl-atomize file.lean --sparql atoms.ttl
```

## Output
- `atoms.json` — atom lattice with CIDs, edges, minimal closure
- `*.ttl` — SPARQL triples for graph DB import

## See Also
- [[skills/locate2shmem]]
- [[skills/shmem2disk]]
- [[tasks/2026-06-21-dasl-ring-tasks]]

## Shmem Cross-References

> Generated: 2026-06-23 11:12:43 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| CID | computeCID | def |
| Generate | generateLeech | def |
| Generate | generateDatagram | def |
| Generate | generateShardSummary | def |
| Monster | monsterPrime | def |
| Monster | meme_clifford_fractran_monster | theorem |
| Monster | meme_MONSTER_WALK_INDEX_COMPLETE | theorem |