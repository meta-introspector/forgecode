---
name: shmem-sparql
description: >-
  SPARQL query execution over CAR block store using sophia_rs. Implements
  sophia_api DatasSet trait backed by CAR metadata, enabling SPARQL SELECT,
  ASK, and CONSTRUCT queries with arrow relationships and schema resonance.
  Served as web GUI via letta-ipld-memory serve-web on port 8088.
  Use when querying the DASL knowledge graph or building SPARQL pipelines.
---

# shmem-sparql — SPARQL over CAR Block Store

## one-liner
```bash
letta-ipld-memory sparql "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10"
```

## Trigger
When running SPARQL queries against the DASL knowledge graph, building
SPARQL query pipelines, or querying block metadata as RDF triples.

## Architecture

```
  CAR Block Store (@ipld_car_shmem)
           │
           ▼  letta-ipld-memory sparql
  ┌────────────────────────────────────┐
  │  BlockDataset (sophia_rs)          │
  │                                    │
  │  RDF Namespace:                    │
  │    http://dasl.org/letta/memory#   │
  │                                    │
  │  Properties:                       │
  │    :path         → block path      │
  │    :description  → human desc      │
  │    :size         → byte count      │
  │    :cid          → content hash    │
  │    :readOnly     → mutability      │
  │    :arrowSharedPrefix → path edges │
  │    :resonatesWith → schema types   │
  └────────────┬───────────────────────┘
               │
     ┌─────────┴──────────┐
     ▼                    ▼
  SPARQL GUI           REST API
  /tile/sparql-gui/    POST /sparql
  (nginx → :8088)      (axum)
```

## Sophia RDF (not Oxigraph)

Uses `sophia_rs` — a trait-based, zero-copy RDF library for Rust.
The `BlockDataset` type implements `sophia_api::dataset::Dataset`,
enabling SPARQL via `sophia_sparql::SparqlWrapper`.

**Why Sophia instead of Oxigraph**: Sophia is trait-based (no fixed store),
enabling zero-copy SPARQL directly over the CAR shmem block metadata
without an intermediate serialization step.

## Commands

### CLI

```bash
# Basic SPARQL query
letta-ipld-memory sparql "SELECT ?path ?size WHERE { ?s :path ?path ; :size ?size } ORDER BY DESC(?size) LIMIT 10"

# With arrow relationships (shared path prefixes)
letta-ipld-memory sparql "SELECT ?a ?arrow ?b WHERE { ?a :arrowSharedPrefix ?b }" --arrows

# With schema resonance (PascalCase tokens as types)
letta-ipld-memory sparql \
  "SELECT ?type (COUNT(?block) as ?n) WHERE { ?block :resonatesWith ?type } GROUP BY ?type" \
  --resonance

# Store results as CAR block
letta-ipld-memory sparql "SELECT * WHERE { ?s ?p ?o }" --store-as sparql/results
```

### Web GUI (REST API)

```bash
# Start the web server
letta-ipld-memory serve-web --port 8088

# Execute SPARQL via REST
curl -X POST http://127.0.0.1:8088/sparql \
  -H "Content-Type: application/json" \
  -d '{"query":"SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 5"}'

# Pipeline mode (CONSTRUCT feeds subsequent queries)
curl -X POST http://127.0.0.1:8088/sparql/pipeline \
  -H "Content-Type: application/json" \
  -d '{"queries":["CONSTRUCT {?s :hasType ?type} WHERE {?s :resonatesWith ?type}","SELECT ?type (COUNT(?s) as ?n) WHERE {?s :hasType ?type}"]}'
```

### Public access

```
https://solana.solfunmeme.com/tile/sparql-gui/
```

(Proxied through nginx to port 8088)

## RDF Triples Generated per Block

For each block in the CAR store, the following triples are generated:

```turtle
<urn:letta:block:0> rdf:type :Block .
<urn:letta:block:0> :path "projects/arist/src/main.rs" .
<urn:letta:block:0> :description "Aristotle manager main entry point" .
<urn:letta:block:0> :size "12345"^^xsd:string .
<urn:letta:block:0> :cid "01551220abc123..." .
<urn:letta:block:0> :readOnly "false"^^xsd:string .
```

### Arrow Relationships (--arrows)
```turtle
<urn:letta:block:0> :arrowSharedPrefix <urn:letta:block:1> .
# Generated when two blocks share a path prefix ≥ 16 chars
```

### Schema Resonance (--resonance)
```turtle
<urn:letta:block:0> :resonatesWith :CarShmemClient .
# Generated from PascalCase tokens in block paths
```

## Related Commands

| Command | What |
|---------|------|
| `classify-blocks` | Classify all blocks by language/type |
| `diagonalize` | Hierarchical view of block store |
| `resonate` | Schema-schema resonance analysis |
| `schema-sets` | IPLD schema → SPARQL constructors |
| `evaluate` | Evaluate queries with scoring |

## Query Examples

```sparql
# Find the 10 largest blocks
SELECT ?path ?size WHERE {
  ?s :path ?path ; :size ?size
} ORDER BY DESC(xsd:integer(?size)) LIMIT 10

# Count blocks by schema type (with resonance)
SELECT ?type (COUNT(?block) AS ?count) WHERE {
  ?block :resonatesWith ?type
} GROUP BY ?type ORDER BY DESC(?count)

# Find blocks that share path prefixes (arrow relationships)
SELECT ?a_path ?b_path WHERE {
  ?a :arrowSharedPrefix ?b .
  ?a :path ?a_path .
  ?b :path ?b_path .
} LIMIT 20

# Find read-only blocks with "ipld" in their path
SELECT ?path ?cid WHERE {
  ?s :path ?path ; :cid ?cid ; :readOnly "true" .
  FILTER(CONTAINS(?path, "ipld"))
}
```

## Files

- `src/bin/letta-memory/commands/sparql.rs` — SPARQL query execution (240 lines)
- `src/bin/letta-memory/commands/web_server.rs` — Web UI + REST API (690 lines)
- `src/bin/letta-memory/commands/classify_blocks.rs` — Block classification
- `src/bin/letta-memory/commands/resonate.rs` — Schema resonance
- `src/bin/letta-memory/commands/schema_sets.rs` — IPLD schema → SPARQL constructors
- `src/bin/letta-memory/cli.rs` — CLI interface with sparql subcommand
- `/etc/nginx/locations.d/sparql-gui.conf` — Nginx reverse proxy config

## Build

```bash
cd /mnt/data1/time-2026/02-february/22/dasl/ipld-car-ipc-shmem-linux
cargo build --release --bin letta-ipld-memory
```

## See Also

- [[skills/shmem2disk]]
- [[skills/locate2shmem]]
- [[skills/dasl-atomize]]
- [[skills/tantivy-indexer]]
- [[skills/shmem-backup]]
- [[skills/ipld-car-shmem]]

## Shmem Cross-References

> Generated: 2026-06-23 10:20:03 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Build | buildPrefixTree | def |
| CAR | exists_isPicardLindelof_const_of_contDiffAt | theorem |
| CAR | exists_finite_card_le_of_finite_of_linearIndependent_of_span | theorem |
| CAR | IsPicardLindelof.exists_forall_hasDerivWithinAt_Icc_eq | theorem |
| CLI | bott_cycling_correct | theorem |
| CLI | meme_clifford_fractran_monster | theorem |
| Count | meme_byte_count_verify | theorem |
| Count | meme_byte_counter_mod10 | theorem |
| Count | meme_byte_counter_executor | theorem |
| Execute | meme_execute_fractran_prover | theorem |
| Generated | generateDatagram | def |
| Generated | generateDASL | def |
| RDF | RdfTriple | structure |
| With | ofDigits_add_ofDigits_eq_ofDigits_zipWith_of_length_eq | theorem |
| With | IsPicardLindelof.exists_forall_hasDerivWithinAt_Icc_eq | theorem |
| With | exists_mem_nhdsWithin_lt_dimH_of_lt_dimH | theorem |