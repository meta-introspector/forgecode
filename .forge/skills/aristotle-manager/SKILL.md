---
name: aristotle-manager
description: >-
  Aristotle Manager — Rust CLI for polling Aristotle API, downloading Lean4 projects,
  splitting declarations, generating DASL indexes. This is the public CLI; the
  private workspace holds data/scripts. Use when editing src/*.rs, running cargo
  build, processing aristotle_results, using SplitDecls.lean, or managing DASL
  pipelines. Trigger for fetch, download, split, merge, pipeline, replay, index,
  scan-index, notebooklm, dasl, locate2proof, submit commands.
---

# aristotle-manager — Public CLI + Private Workspace

## Repo Layout

| Repo | Path | Purpose | Visibility |
|------|------|---------|------------|
| **Public CLI** | `~/2026/06-june/26/aristotle-cli-rs/` | Generic Rust framework | Public |
| **Private workspace** | `/mnt/data1/time-2026/05-may/07/arist/` | Data, scripts, DASL indexes | Private |

**Rule:** Generic framework logic → public. Project-specific data, DASL corpus, experimental tooling → private.

## Source Modules (public)

| Module | Lines | Role |
|--------|-------|------|
| `main.rs` | 1902 | CLI parser, config, poll/build/test/download |
| `fetch.rs` | 193 | Incremental fetch: poll API, skip processed IDs |
| `pipeline.rs` | 247 | fetch → split → verify → version → merge |
| `replay.rs` | 296 | Chronological replay of archive |
| `version.rs` | 202 | Git-version each project |
| `index.rs` | 230 | DASL `blocks.json` generator |
| `notebooklm.rs` | 114 | Export for NotebookLM |
| `file_index.rs` | 346 | Scan file lists, ingest Lean4 proofs |
| `tests.rs` | 535 | Integration tests |

## CLI Commands (public)

| Command | Flags | Description |
|---------|-------|-------------|
| `fetch` | `-j`, `--limit`, `--dry-run` | Incremental poll + download |
| `download` | `-j`, `--trace`, `--verbose`, `--limit` | Bulk download |
| `check` | `[ID]`, `--limit` | Query project status |
| `submit` | `<prompt>`, `--project-dir`, `--wait` | Submit to Aristotle |
| `download-result` | `<ID>`, `--output-dir` | Download one tarball |
| `build` | `--input-dir`, `--no-fail-fast`, `-v` | `lake build` all projects |
| `test` | `--no-fail-fast`, `-v` | Alias for build |
| `poll` | `--download-only`, `-j` | Git-pull + build |
| `split` | `--input-dir`, `--output-dir` | Run SplitDecls on one project |
| `split-all` | `--output-dir`, `-j`, `--dry-run` | Split every project |
| `merge` | `--input-dir`, `--output-dir` | Merge per-decl `.lean` files |
| `decl-table` | `--split-dir`, `--output` | Build `decl-table.json` |
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

**Known hard-code:** `main.rs:267` falls back to `/mnt/data1/time-2026/05-may/07/arist` when CWD matches. Override with `--input-dir`.

## Private Workspace Scripts (being consolidated into Rust)

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
| `detect-stale.py` | `fetch` (partial) |
| `submit-chunks.py` / `submit-to-aristotle.py` | `submit --chunked` |
| `locate2proof-collect.py` | `locate2proof` |
| `split-aristotle-project.sh` | `split --all` (direct Rust) |
| `split-lean-project.sh` | `split --module` |
| `refresh.sh` | `pipeline` (absorbed) |
| `run_notebooklm_all.sh` | `notebooklm --all` |

## Private Data Directories

| Dir | What it holds |
|-----|--------------|
| `aristotle_results/` / `aristotles_results/` | Downloaded tarballs + extracted `*_aristotle/` |
| `consolidated-dasl/` | Unified DASL Lean4 corpus |
| `split-results*/` | Per-declaration split output |
| `split-by-band/` | Prime-band partitioned splits (q0–q4) |
| `merged-results/` | Post-dedup unified pool |
| `mathlib-split/` | 5,790 unique declarations |
| `cid-index/`, `arrow-lattice/`, `atomized-lattice/` | DASL content-addressed indexes |
| `dep-graph/`, `j-keyed-lattice/`, `mycelium-graph/`, `spec-cid-index/` | Dependency / lattice graphs |
| `findings-*/`, `locate2proof-output/` | Scan results |

## Splitter Engine (private, never port to Rust)

- `splitter-engine/RequestProject/SplitDecls.lean` — core engine (kernel API, exact deps)
- `splitter-engine/RequestProject/DupFinder.lean` — exact duplicate detection
- `splitter-engine/RequestProject/SemanticDupFinder.lean` — near-duplicate detection
- 20+ analysis `.lean` files (ArrowFinder, SizeFinder, etc.) — experimental

## Build & Run

```bash
# Public CLI
cd ~/2026/06-june/26/aristotle-cli-rs
cargo build --release

# Private workspace (uses nora; override if needed)
cd /mnt/data1/time-2026/05-may/07/arist
CARGO_REGISTRIES_CRATES_IO_INDEX="sparse+https://index.crates.io/" cargo build --release
# or
nix-shell && cargo build
```

## Make Targets (private)

```bash
make test            # build + test
make poll            # git-pull + build
make split           # split all via Rust+Lean
make split-all       # batch split
make merge           # merge split results
make decl-table      # canonical decl-table.json
make index           # DASL blocks.json
make dedup-dasl      # split ~/dasl/ Lean files
make refresh         # full pipeline (being absorbed)
make rust-build      # cargo build --release
make help            # full target list
```

## DASL Categories

```rust
enum DaslCategory { Monster, CFSG, DaslIpld, FracTRAN, LeanProof, Advanced, Unclassified }
```

## Prime Bands

```rust
enum PrimeBand { Q0, Q1, Q2, Q3, Q4 }
// Q0: max_prime < 100, Q1: < 1000, Q2: < 10000, Q3: < 100000, Q4: else
```

## Related Skills

- [[aristo-consolidation]] — Plan to merge all splitters into Rust
- [[aristotle-splitter]] — SplitDecls engine details
- [[aristotle-mathlib-split]] — Unified dedup pool
- [[aristo-tools]] — Downloading and processing Aristotle projects
- [[lean4]] — Lean 4 theorem proving

## Troubleshooting

### Build fails: cannot replace crates-io with git-index
Global `~/.cargo/config` uses a local Nora mirror missing some versions.
```bash
CARGO_REGISTRIES_CRATES_IO_INDEX="sparse+https://index.crates.io/" cargo build
cargo update   # regenerate lock against nora
```

### lake not found
Set `LEAN_BIN` to Lean's bin dir, or ensure `lake` is on `PATH`. Falls back to `/nix/store/*lean*/bin/lake`.

### SplitDecls.lean not found
Set `LEAN_SPLITTER` env var, or place at `../lean-split-decls/RequestProject/SplitDecls.lean`.

### No project directories found
Check `config.toml` `git_base`. Projects must be `*_aristotle/` dirs with `output-final_aristotle/RequestProject/`.

## Shmem Cross-References

> Generated: 2026-06-23 11:11:00 | REPL: http://localhost:8156 | Declarations loaded: 366

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