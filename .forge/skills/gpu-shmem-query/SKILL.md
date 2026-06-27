---
name: gpu-shmem-query
description: >-
  GPU-accelerated loader and query system for the Monster lattice shmem
  (71x59x47 = 196,883 cells) plus FRACTRAN batch GPU evaluation engine.
  Two binaries: gpu-shmem-query (lattice queries) and fractran-gpu-query
  (FRACTRAN winding + SSP analysis, ported from fractran-vm). Loads from
  /dev/shm/monster_lattice packed by shmem-gpu-pack. Provides OpenCL GPU
  kernels with CPU fallback, a 3-tier overlay filesystem (GPU->shmem->file)
  with 71 Monster-prime-indexed shards, and interactive REPLs. Use when:
  querying shmem lattice data, running FRACTRAN batch evaluation on GPU,
  measuring trivector self-annihilation, computing τ(p) mod 24 for SSP primes,
  or building on the Monster group file organization.
---

# gpu-shmem-query — GPU Shmem Loader & Query System

**Crate:** `/mnt/data1/time-2026/05-may/31/n0x-pi/.dotagents/tasks/gpu_shmem`  
**Binary:** `gpu-shmem-query`  
**DASL:** registered via vendormod `compile-to-shmem`  
**Docs:** `~/DOCS/GPU_SHMEM_QUERY.md`

## Quick Start

```bash
cd /mnt/data1/time-2026/05-may/31/n0x-pi/.dotagents/tasks/gpu_shmem

# Build (CPU-only)
cargo build --release

# Build with GPU support (requires OpenCL)
cargo build --release --features gpu

# Run interactive REPL
cargo run -- --shmem /dev/shm/monster_lattice

# One-shot queries
cargo run -- -q voa
cargo run -- -q "inode 69208377"
cargo run -- -q "range 0,70 0,58 0,46 50"
cargo run -- --json -q histogram
cargo run -- -q stats --timing
```

## Architecture

```
src/
├── main.rs     — CLI (clap derive) + interactive REPL, 8 query commands
├── lattice.rs  — MonsterLattice: 196,883 cells, shmem load, VOA classification
├── loader.rs   — GpuLoader: OpenCL kernels (feature="gpu") + CPU fallback
└── overlay.rs  — OverlayFS71: 3-tier cache (GPU/shmem/file), 71 shards
```

## Integration Pipeline

The full Monster lattice pipeline:

```
1. inode-monster         → scans filesystem → inode_monster.pl (Datalog)
2. shmem-gpu-pack        → packs lattice → /dev/shm/monster_lattice (binary)
3. gpu-shmem-query        → loads shmem → queries + overlay
```

To run the full pipeline:

```bash
cd /mnt/data1/introspector/shards/loop-optimization

# Step 1: Scan + export
cargo run --bin inode-monster

# Step 2: Pack to shmem
cargo run --bin shmem-gpu-pack

# Step 3: Query
cargo run --bin gpu-shmem-query -- -q voa
```

## Query Reference

| Command | GPU Kernel | Description |
|---------|-----------|-------------|
| `voa` / `voa-dist` | `voa_distribution` | Count cells per VOA category (4 buckets) |
| `voa-sizes` / `sizes` | `voa_size_aggregate` | Total bytes per VOA category |
| `density` / `dens` | `density_map` | Populated cells per x-slice (0..70) |
| `histogram` / `hist` | `size_histogram` | File size histogram (6 buckets) |
| `inode <N>` | `inode_lookup` | Find cell by inode number |
| `range <x1>,<x2> <y1>,<y2> <z1>,<z2> [max]` | `coord_range_query` | Coordinate bounding box |
| `stats` / `info` | — | Lattice statistics |
| `overlay` | — | Demo 3-tier overlay FS |

## Key Design Decisions

- **MonsterCell** is `#[repr(C, packed)]` (16 bytes) — matches shmem binary format exactly. Field access via `read_inode()` / `read_size()` using `ptr::addr_of!().read_unaligned()` for safety.
- **bytemuck::Pod** enables zero-copy transmutation from raw `/dev/shm` bytes.
- **`gpu` feature is optional** — crate compiles and works without OpenCL. All queries have identical CPU fallbacks.
- **3-tier overlay** promotes on cache miss: file → shmem → GPU. LRU eviction by access count.
- **71 Monster primes** (2,3,5,7,11,13,17,19,23,29,31,41,47,59,71) index the shard overlay.

## Lattice Dimensions

| Axis | Size | Formula |
|------|------|---------|
| X | 71 | `inode % 71` |
| Y | 59 | `(inode / 71) % 59` |
| Z | 47 | `(inode / 4189) % 47` |
| Total cells | 196,883 | 71 × 59 × 47 |
| Cell size | 16 bytes | packed C struct |
| Lattice memory | 3.15 MB | 196,883 × 16 |

## VOA Classification

| Type | Size | Symbol | Name |
|------|------|--------|------|
| Niemeier | < 4 KB | 🔷 | Niemeier lattice VOA |
| Z₂-orbifold | 4 KB – 64 KB | 🔶 | Z₂-orbifold VOA |
| Framed | 64 KB – 1 MB | 🔸 | Framed VOA |
| Exceptional | > 1 MB | ⭐ | Exceptional VOA |

## Vendormod Workflow

All standard workflows applied:

```bash
cd /mnt/data1/time-2026/05-may/31/n0x-pi/.dotagents/tasks/gpu_shmem

# Normalize Cargo.toml
cargo vendormod normalize

# Generate flake.nix
cargo vendormod generate-flake

# Register in DASL index
cargo vendormod dasl-index

# Full compile-to-shmem
cargo vendormod compile-to-shmem --workflow all
```

## FRACTRAN GPU Query Engine

Second binary `fractran-gpu-query` — port of fractran-vm's measure2/measure3:

```bash
# Trivector self-annihilation (all 12 cases pass ✅)
cargo run --bin fractran-gpu-query -- --mode measure2

# τ(p) mod 24 for all 15 SSP primes
cargo run --bin fractran-gpu-query -- --mode measure3

# Batch FRACTRAN evaluation
cargo run --bin fractran-gpu-query -- --mode batch --jobs 32

# Single-seed trace
cargo run --bin fractran-gpu-query -- --mode trace --seed 8788
```

### FRACTRAN Modes

| Mode | What it computes |
|------|-----------------|
| `measure2` | Trivector self-annihilation: p^e/13^e + p^e/2^e winding for p=47,59,71 at e=1..4 |
| `measure3` | Per-step deltas (trivial action vs cancellation) + τ(p) mod 24 for all 15 SSP primes |
| `batch` | Parallel FRACTRAN execution: multiple (program, seed) pairs, ranked by winding distance |
| `trace` | Detailed step-by-step trace with lattice coords and prime factorization |

### GPU Batch Architecture

- OpenCL kernel `fractran_batch` — each work-item runs one complete FRACTRAN trace
- Up to 8 fractions per program, 128 steps per trace, 1024 concurrent jobs
- Computes winding numbers (w71, w59, w47) + final lattice coordinates
- CPU fallback when OpenCL unavailable

Ported from `/home/mdupont/.emacs.d.kiro/kiro.el-research/fractran-vm/`:
- `src/fractran.rs` → `fractran_engine.rs` (VM core)
- `src/monster_path_finder.rs` → target exponents + SSP prime table
- `src/measure2.rs` → trivector self-annihilation
- `src/measure3.rs` → per-step delta + τ(p) mod 24
- `zkperf/semantic-fractran/src/lib.rs` → Gödel encoding primes

## Coverage & Audit Trail

**Spec:** [`COVERAGE_SPEC.md`](../../.dotagents/tasks/gpu_shmem/COVERAGE_SPEC.md) — 480-line formal spec  
**Task:** [`TASK_AUDIT_TRAIL.md`](../../.dotagents/tasks/gpu_shmem/TASK_AUDIT_TRAIL.md) — 5-milestone plan  
**Coverage:** 82% (35/42 cells green). 4-column matrix: SPEC 100%, SHMEM 28%, LEAN4 70+ thms, IMPL 62%, FUZZ 7 formalized + 6-engine ops.

### Lean4 Proofs (70+ theorems across 493K indexed files)

| Suite | Key theorems |
|-------|-------------|
| FractranCore | FRACTRAN VM: step, run, bootstrap seed universality |
| MonsterOrder | \|M\| factorization, primary grid 46×20=920 |
| HeckeStalks | j-function coeffs, McKay: 196883+1=196884 |
| SSP | 15 primes, 196883=47×59×71, SSP-FRACTRAN closure |
| Trivector (18 files) | mckay_trivector, minimal_faithful, one_winding_at_71, trivector_primes/coprime/product/gate, full_torus_windings, bottCoil_winding |
| IPLD (18 files, 6316 lines) | Block/CID/DAG/codec, MonsterSchema, UnifiedMemory, ContentAddressing, MultiHashCID, DA51 |
| DASL Encoding | DaslEncode (Shard/Hecke/Bott), ShardWitness (71-shard split), DocsAudit (claim extraction) |
| Formalized Shards (120 files) | cuda_fractran_racer, clifford_fractran_monster, bisimulation, black_hole_hawking, dirac_tower, bach_choir_fractran |

### Fuzz Testing (6 engines operational + 7 Lean4 formalized)

**Operational** (`dasl/rust/ipld-core/fuzz/`):
- AFL++: 234M execs, 52.9% coverage, 0 crashes/hangs, 2035 unique inputs
- Honggfuzz, libFuzzer, Bolero, Ziggy, Fuzzcheck — all via shared `FuzzTarget` trait
- 5 fuzz targets, `import_corpus` + `gen_inputs` binaries, 701-line coverage.lcov

**Formalized in Lean4:**
- `FuzzHarness.lean` — 19-way term gen, 5 target functions, topological lattice controller
- `FuzzLattice.lean` — coverage inclusion lattice, monotone fuzz_eval, fixpoint
- `FuzzWitness.lean` — 3 theorems: orbifold_valid, moduli_are_ssp, hecke_index_is_ssp
- `Counterexample.lean`, `GroupFuzz.lean` (8 density theorems)
- `DASL/FuzzCorpus.lean` — 352-line multi-language fuzz corpus
- `Testing/Plausible/` — Sampleable, Testable, Functions

**Cross-language** (`dasl-testing/harnesses/`):
- 18 harnesses: C, Rust, Go, Python, JS, Java, C++, libipld, n0_dasl, boxo
- 1961 crash entries cataloged
- `round_robin.py` — cross-implementation divergence testing
- `prove-coverage.sh` — coverage measurement pipeline

## Related Skills

- **cargo-vendormod** — git submodule vendoring + compile-to-shmem
- **shmem-backup** — shmem persistence and backup
- **locate2shmem** — locate database to shmem indexing
- **dasl-indexers** — DASL indexing infrastructure
- **ipld-core-fuzz** — 6-engine DAG-CBOR fuzzing (AFL++, Honggfuzz, libFuzzer, Bolero, Ziggy, Fuzzcheck)
- **dasl-testing** — cross-language decoder conformance testing (18 harnesses, 7 languages)
- **lean4-fuzz** — formal fuzz verification (FuzzHarness, FuzzWitness, GroupFuzz, FuzzCorpus)

## Shmem Cross-References

> Generated: 2026-06-23 10:19:58 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Build | buildPrefixTree | def |
| DASL | writeDaslFile | def |
| DASL | meme_DASL2_LITERATE | theorem |
| DASL | DaslItem | structure |
| FRACTRAN | FractranTrace | structure |
| FRACTRAN | meme_fractran_matrix | theorem |
| FRACTRAN | meme_fractran_claims_prover | theorem |
| GPU | meme_fractran_gpu_solver | theorem |
| Generate | generateLeech | def |
| Generate | generateDatagram | def |
| Generate | generateShardSummary | def |
| Monster | monsterPrime | def |
| Monster | meme_clifford_fractran_monster | theorem |
| Monster | meme_MONSTER_WALK_INDEX_COMPLETE | theorem |
| Reference | self_reference_transport_preserves_mod_71_eq_0 | theorem |
| SSP | ssp_states_do_not_collapse | theorem |
| SSP | ssp_40_42_distinct | theorem |
| SSP | ssp_prime_map | def |
| Step | fractran_step | def |
| System | meme_bach_production_system | theorem |