---
name: aristo-tools
description: "Use when working with Aristotle Manager — the Rust CLI for polling Aristotle API, downloading Lean4 projects, splitting declarations, and generating DASL indexes. Covers both the public aristotle-cli-rs binary and the private workspace with data/scripts. Trigger when editing src/*.rs, running cargo build, processing aristotle_results, using SplitDecls.lean, or managing DASL pipelines."
---

# Aristotle Manager — Codebase & Operations Guide

## Repo Layout (as of 2026-06-22)

| Repo | Path | Purpose | Visibility |
|------|------|---------|------------|
| **Public CLI** | `~/2026/06-june/26/aristotle-cli-rs/` | Generic Rust framework, CI-ready | Public |
| **Private workspace** | `/mnt/data1/time-2026/05-may/07/arist/` | Data, scripts, DASL indexes, split results | Private |

**Rule:** Generic framework logic → public. Project-specific data, DASL corpus, and experimental tooling → private.

## Architecture

```
Aristotle API (aristotle.harmonic.fun/api/v3)
         │
         ▼
   ┌─────────────┐
   │  aristotle-  │  Rust CLI (tokio + reqwest)
   │  manager     │  Commands: fetch, download, split, merge, pipeline, index
   └──────┬──────┘
          │
          ▼
   ┌─────────────┐
   │ SplitDecls  │  Lean4 kernel-level splitter (Environment API)
   │ .lean        │  Exact dep tracking, BFS closure, iterative topo-sort
   └──────┬──────┘
          │
          ▼
   ┌─────────────┐
   │  mathlib-   │  Deduplicated per-declaration .lean + flake.nix
   │  split/     │  5,790 unique declarations from 43 projects
   └──────┬──────┘
          │
          ▼
   ┌─────────────┐
   │   DASL      │  blocks.json, arrow-lattice, cid-index, prime-bands
   │  indexes    │  Content-addressed metadata for Lean corpus
   └─────────────┘
```

## Source Modules (public CLI)

| Module | Lines | Role |
|--------|-------|------|
| `main.rs` | 1902 | CLI parser, config, poll/build/test/download orchestration |
| `fetch.rs` | 193 | Incremental fetch: poll API, skip processed IDs, download new |
| `pipeline.rs` | 247 | Full pipeline: fetch → split → verify → version → merge |
| `replay.rs` | 296 | Chronological replay of entire archive |
| `version.rs` | 202 | Git-version each project (commit with metadata) |
| `index.rs` | 230 | DASL `blocks.json` generator with category heuristics |
| `notebooklm.rs` | 114 | Export project text for NotebookLM |
| `file_index.rs` | 346 | Scan file lists / grep results, ingest Lean4 proofs |
| `tests.rs` | 535 | Integration tests |

## CLI Commands (public)

| Command | Flags | Description |
|---------|-------|-------------|
| `fetch` | `-j`, `--limit`, `--dry-run` | Incremental poll + download |
| `download` | `-j`, `--trace`, `--verbose`, `--limit` | Bulk download all projects |
| `check` | `[ID]`, `--limit` | Query project status |
| `submit` | `<prompt>`, `--project-dir`, `--wait` | Submit to Aristotle API |
| `download-result` | `<ID>`, `--output-dir` | Download one tarball |
| `build` | `--input-dir`, `--no-fail-fast`, `-v` | Run `lake build` in all projects |
| `test` | `--no-fail-fast`, `-v` | Alias for build |
| `poll` | `--download-only`, `-j` | Git-pull + build |
| `split` | `--input-dir`, `--output-dir` | Run SplitDecls on one project |
| `split-all` | `--output-dir`, `-j`, `--dry-run` | Split every project |
| `merge` | `--input-dir`, `--output-dir` | Merge per-decl `.lean` files |
| `decl-table` | `--split-dir`, `--output` | Build canonical `decl-table.json` |
| `refresh` | `-j`, `--limit` | download → split → decl-table |
| `pipeline` | `-j`, `--limit`, `--dry-run` | fetch → split → verify → version → merge |
| `replay` | `--output-dir`, `--dry-run` | Chronological rebuild |
| `version` | `--results-dir`, `--output-dir` | Git-commit each project |
| `index` | `--output` | DASL `blocks.json` |
| `scan-index` | `--index-dir`, `--output-dir`, `--prefix-filter` | Ingest file lists |
| `notebooklm` | `--project-dir` | Export for NotebookLM |
| `configure` | `set\|show` | API key and settings |
| `results` | — | Print `result.txt` |
| `clean` | — | Remove `result.txt` |

## Config

Auto-created at `~/.config/aristotle-manager/config.toml`:

```toml
base_dir = "aristotles_results"
results_dir = "aristotles_results"
git_base = "aristotles_results"
max_parallel_downloads = 4
retry_wait_seconds = 10
max_retries = 3
```

**Known hard-code:** `main.rs:267` falls back to `/mnt/data1/time-2026/05-may/07/arist` when CWD matches that path. Override with `config.toml` or `--input-dir`.

## Private Workspace Contents

### Scripts (being consolidated into Rust)

| Script | Being ported to |
|--------|----------------|
| `split-source.py` | `split --source` (fallback splitter) |
| `dedup-split.py` | `merge --dedup-only` |
| `build-dasl-module.py` | `dasl build-module` |
| `build-unified-dasl.py` | `dasl build-unified` |
| `dasl-term-filter.py` | `dasl filter` |
| `fix-unified-imports.py` | `dasl fix-imports` |
| `prime-stratify.py` | `dasl stratify` |
| `partition-dasl.py` | `dasl partition` |
| `detect-stale.py` | `fetch` (already partially done) |
| `submit-chunks.py` / `submit-to-aristotle.py` | `submit --chunked` |
| `locate2proof-collect.py` | `locate2proof` |
| `split-aristotle-project.sh` | `split --all` (direct Rust, no shell) |
| `split-lean-project.sh` | `split --module` |
| `refresh.sh` | `pipeline` (absorbed) |
| `run_notebooklm_all.sh` | `notebooklm --all` |

### Data Directories (private)

| Dir | What it holds |
|-----|--------------|
| `aristotle_results/` / `aristotles_results/` | Downloaded tarballs + extracted `*_aristotle/` dirs |
| `consolidated-dasl/` | Unified DASL Lean4 corpus |
| `split-results*/` | Per-declaration split output |
| `split-by-band/` | Prime-band partitioned splits (q0–q4) |
| `merged-results/` | Post-dedup unified pool |
| `mathlib-split/` | 5,790 unique declarations (after dedup) |
| `cid-index/`, `arrow-lattice/`, `atomized-lattice/` | DASL content-addressed indexes |
| `dep-graph/`, `j-keyed-lattice/`, `mycelium-graph/`, `spec-cid-index/` | Dependency / lattice graphs |
| `findings-*/`, `locate2proof-output/` | Scan results |

### Splitter Engine (private)

- `splitter-engine/RequestProject/SplitDecls.lean` — **core engine**, never port to Rust
- `splitter-engine/RequestProject/DupFinder.lean` — exact duplicate detection
- `splitter-engine/RequestProject/SemanticDupFinder.lean` — near-duplicate detection
- 20+ analysis `.lean` files (ArrowFinder, SizeFinder, etc.) — experimental, keep private

## Build & Run

```bash
# Public CLI
cd ~/2026/06-june/26/aristotle-cli-rs
cargo build --release
./target/release/aristotle-manager --help

# Private workspace (needs nora or CARGO_REGISTRIES_CRITES_IO_INDEX override)
cd /mnt/data1/time-2026/05-may/07/arist
CARGO_REGISTRIES_CRATES_IO_INDEX="sparse+https://index.crates.io/" cargo build --release

# Or via nix
nix-shell
cargo build
```

## Make Targets (private workspace)

```bash
make test            # build + test all Lean4 projects
make poll            # git-pull + build
make split           # split all via Rust+Lean
make split-all       # batch split via shell driver
make merge           # merge split results
make decl-table      # canonical decl-table.json
make index           # DASL blocks.json
make dedup-dasl      # split ~/dasl/ Lean files
make refresh         # full pipeline (being absorbed into cmd_pipeline)
make rust-build      # cargo build --release
make help            # full target list
```

## DASL Categories (used by index + cmd_dasl build-module)

```rust
enum DaslCategory {
    Monster,      // "monster group", "196883", "umbral", "moonshine", "leech lattice"
    CFSG,         // "finite simple group", "cfsg", "sporadic group", "mathieu"
    DaslIpld,     // "dasl", "ipld", "dag-cbor", "car file", "cid v1", "multihash"
    FracTRAN,     // "fractran"
    LeanProof,    // "theorem", "lemma", "formalized", "verified"
    Advanced,     // "p-adic", "bernoulli", "modular form", "l-function"
    Unclassified, // fallback
}
```

## Prime Bands (used by cmd_dasl stratify)

```rust
enum PrimeBand { Q0, Q1, Q2, Q3, Q4 }

fn get_band(max_prime: u64) -> PrimeBand {
    match max_prime {
        < 100   => Q0,
        < 1000  => Q1,
        < 10000 => Q2,
        < 100000 => Q3,
        _       => Q4,
    }
}
```

## Key Paths (private workspace)

- Splitter engine: `/mnt/data1/time-2026/05-may/07/arist/splitter-engine/`
- DASL indexes: `/mnt/data1/time-2026/05-may/07/arist/{cid-index,arrow-lattice,atomized-lattice,dep-graph,j-keyed-lattice,mycelium-graph,spec-cid-index}`
- Submissions: `/mnt/data1/time-2026/05-may/07/arist/aristotle-submissions/`
- Nix flake: `/mnt/data1/time-2026/05-may/07/arist/flake.nix` (uses local `mnt/data1/git/` mirrors)

## Consolidation Roadmap

See: `~/dotagents/tasks/split-tools-consolidation-plan.md`

**Goal:** All splitting, dedup, DASL pipeline, and orchestration logic moves from Python/shell into Rust. Lean (`SplitDecls.lean`) remains the kernel-level engine.

**Phases:**
1. Shared types (`src/split/`, `src/dasl/`)
2. `cmd_split` unification (absorb `split-all`, `split-source.py`, shell wrappers)
3. `cmd_merge` + dedup (absorb `dedup-split.py`)
4. `cmd_pipeline` unification (absorb `refresh`, `replay` via `--skip-*` flags)
5. `cmd_dasl` with 7 subcommands (absorb all DASL Python scripts)
6. `cmd_locate2proof`
7. `cmd_submit --chunked`
8. Delete all Python/shell scripts, update docs

**Commands to delete from CLI:** `Refresh`, `Replay`, `SplitAll` (absorbed)
**Commands to add:** `split --source`, `split --module`, `merge --dedup-only`, `dasl *`, `locate2proof`, `submit --chunked`

## Troubleshooting

### Build fails with "cannot replace crates-io with git-index"
Global `~/.cargo/config` replaces crates.io with a local Nora mirror that lacks versions pinned in `Cargo.lock`.

```bash
# Fix: override index for this build
CARGO_REGISTRIES_CRATES_IO_INDEX="sparse+https://index.crates.io/" cargo build

# Or update lock to match nora's index
cargo update
```

### lake not found
Set `LEAN_BIN` to Lean's bin directory (e.g. `/path/to/lean/bin`), or ensure `lake` is on `PATH`. The tool searches `/nix/store/*lean*/bin/lake` as fallback.

### SplitDecls.lean not found
Set `LEAN_SPLITTER` env var, or place `SplitDecls.lean` in `../lean-split-decls/RequestProject/SplitDecls.lean` relative to CWD.

### No project directories found
Check `config.toml` `git_base` path. Projects must be `*_aristotle/` directories containing `output-final_aristotle/RequestProject/`.

## Shmem Cross-References

> Generated: 2026-06-23 11:10:58 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Build | buildPrefixTree | def |
| CLI | bott_cycling_correct | theorem |
| CLI | meme_clifford_fractran_monster | theorem |
| DASL | writeDaslFile | def |
| DASL | meme_DASL2_LITERATE | theorem |
| DASL | DaslItem | structure |
| Data | generateDatagram | def |
| Data | meme_agreements_data | theorem |
| Make | meme_CMakeLists | theorem |
| Modules | meme__gitmodules | theorem |
| Monster | monsterPrime | def |
| Monster | meme_clifford_fractran_monster | theorem |
| Monster | meme_MONSTER_WALK_INDEX_COMPLETE | theorem |
| Prime | irreducible_of_degree_eq_one_of_isRelPrime_coeff | theorem |
| Prime | monsterPrime | def |
| Prime | bott_primes | def |
| Repo | meme_emoji_dao_agreements_report | theorem |
| Repo | meme_emoji_agreements_report | theorem |