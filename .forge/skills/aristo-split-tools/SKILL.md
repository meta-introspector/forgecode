---
name: aristo-split-tools
description: "Use when working with any of the 23 Aristotle split tools — merging them into a unified Rust CLI. Contains the master inventory, per-tool task directories, and the consolidation roadmap. Trigger when porting Python/shell splitters to Rust, documenting split tools, or working on the split-merge-pipeline."
---

# aristo-split-tools — Split Tool Inventory & Merge Plan

## Master Index

| # | Tool | Lang | Status | Port target | Task dir |
|---|------|------|--------|-------------|----------|
| **Core Engine** |
| 1 | SplitDecls.lean | Lean4 | Active — core engine | Never port | `splitter-engine/` |
| **Shell Wrappers** |
| 2 | split-aristotle-project.sh | bash | Active | `cmd_split --all` | `tools/split-aristotle-project-sh` |
| 3 | split-lean-project.sh | bash | Active | `cmd_split --module` | `tools/split-lean-project-sh` |
| 4 | refresh.sh | bash | Active | `cmd_pipeline` (absorbed) | `tools/refresh-sh` |
| **Python Splitters / Dedup** |
| 5 | split-source.py | Python | Active fallback | `cmd_split --source` | `tools/split-source-py` |
| 6 | dedup-split.py | Python | Active | `cmd_merge --dedup-only` | `tools/dedup-split-py` |
| 7 | dedup-split.sh | bash | Wrapper | Delete (covered by merge) | `tools/dedup-split-sh` |
| **DASL Post-Processors** |
| 8 | build-dasl-module.py | Python | Active | `cmd_dasl build-module` | `tools/build-dasl-module-py` |
| 9 | build-unified-dasl.py | Python | Active | `cmd_dasl build-unified` | `tools/build-unified-dasl-py` |
| 10 | dasl-index-scanner.py | Python | Active | `cmd_scan_index` (done) | `tools/dasl-index-scanner-py` |
| 11 | dasl-term-filter.py | Python | Active | `cmd_dasl filter` | `tools/dasl-term-filter-py` |
| 12 | fix-unified-imports.py | Python | Active | `cmd_dasl fix-imports` | `tools/fix-unified-imports-py` |
| 13 | prime-stratify.py | Python | Active | `cmd_dasl stratify` | `tools/prime-stratify-py` |
| 14 | partition-dasl.py | Python | Active | `cmd_dasl partition` | `tools/partition-dasl-py` |
| **Rust CLI Commands** |
| 15 | cmd_split | Rust | Active | Unify with --all/--source/--module | `tools/cmd-split` |
| 16 | cmd_split_all | Rust | Active | Fold into cmd_split --all | `tools/cmd-split-all` |
| 17 | cmd_merge | Rust | Active | Add --dedup-only | `tools/cmd-merge` |
| 18 | cmd_decl_table | Rust | Active | Keep as-is | `tools/cmd-decl-table` |
| 19 | cmd_refresh | Rust | Active | Fold into cmd_pipeline --skip-* | `tools/cmd-refresh` |
| 20 | cmd_pipeline | Rust | Active | Add --skip-* / --from-archive | `tools/cmd-pipeline` |
| 21 | cmd_replay | Rust | Active | Fold into cmd_pipeline --from-archive | `tools/cmd-replay` |
| **Orchestrators** |
| 22 | Makefile split targets | make | Active | Keep (thin wrappers) | `tools/makefile-split` |
| 23 | run_notebooklm_all.sh | bash | Active | `cmd_notebooklm --all` | `tools/run-notebooklm-all-sh` |
| **Supporting (private, never port)** |
| S1 | DupFinder.lean | Lean4 | Experimental | Keep private | `splitter-engine/` |
| S2 | SemanticDupFinder.lean | Lean4 | Experimental | Keep private | `splitter-engine/` |
| S3 | 20+ finder .lean files | Lean4 | Experimental | Keep private | `splitter-engine/` |

## Consolidated Plan

**Goal:** All splitting, dedup, DASL pipeline, and orchestration logic moves from
Python/shell into Rust. Lean (`SplitDecls.lean`) remains the kernel-level engine.

**Commands to DELETE from public CLI:**
- `Commands::Refresh` → `pipeline --skip-version --skip-merge`
- `Commands::Replay` → `pipeline --from-archive <dir>`
- `Commands::SplitAll` → `split --all`

**Commands to ADD to public CLI:**
- `split --source` (fallback splitter, no lake/mathlib deps)
- `split --module <name>` (single module split)
- `split --all` (batch all projects, no shell)
- `merge --dedup-only` (dedup without re-merging)
- `pipeline --skip-fetch|--skip-split|--skip-verify|--skip-version|--skip-merge|--skip-dedup`
- `pipeline --from-archive <dir>` (replay)
- `pipeline --resume` (skip processed)
- `dasl build-module|build-unified|filter|fix-imports|stratify|partition`
- `locate2proof`
- `submit --chunked`

**8 phases, ~17 days:**
1. Shared types (`src/split/`, `src/dasl/`)
2. cmd_split unification
3. cmd_merge + dedup
4. cmd_pipeline unification
5. cmd_dasl (7 subcommands)
6. cmd_locate2proof
7. cmd_submit --chunked
8. Delete all Python/shell scripts

## Per-Tool Task Directories

Each tool has its own task directory under `tools/` with:
- `SYSTEM.md` — what it does, port target, dependencies
- `AGENTS.md` — conventions for porting
- `settings.json` — theme

```
tools/
├── split-decl-lean/           # Core engine
├── split-aristotle-project-sh/
├── split-lean-project-sh/
├── refresh-sh/
├── split-source-py/
├── dedup-split-py/
├── dedup-split-sh/
├── build-dasl-module-py/
├── build-unified-dasl-py/
├── dasl-index-scanner-py/
├── dasl-term-filter-py/
├── fix-unified-imports-py/
├── prime-stratify-py/
├── partition-dasl-py/
├── cmd-split/
├── cmd-split-all/
├── cmd-merge/
├── cmd-decl-table/
├── cmd-refresh/
├── cmd-pipeline/
├── cmd-replay/
├── makefile-split/
└── run-notebooklm-all-sh/
```

## Related

- `split-tools-consolidation-plan/` — detailed phase-by-phase roadmap
- `split-tools-inventory-merge-plan/` — full inventory with overlap analysis
- `split-aristotle-public-private/` — initial repo split
- `lean4-declaration-splitter/` — skill for the Lean splitter engine
- `split-merge-pipeline/` — operational pipeline task
- `split-merge-build-dasl/` — DASL verification build task
- `aristotle-manager/` — public CLI skill
- `aristo-consolidation/` — consolidation rules

## Cross-Validation

Before merging any splitter, run the cross-validation suite:
`tasks/split-cross-validation/`

```bash
bash ~/dotagents/tasks/split-cross-validation/run-one.sh /path/to/project
```

This runs all splitters on the same corpus, diffs outputs, checks syntax,
and produces `CROSS-VALIDATION-REPORT.md`.

**Rule:** Do not port a splitter to Rust until its output matches the reference
`SplitDecls.lean` output on at least 3 projects (tiny, medium, large).

## Rust Splitter Cross-Validation (2026-06-22)

Harness: `tasks/split-cross-validation/run_all_rust_splitters.py`

```
rust-matrix/
├── rust-matrix.csv       # 26 experiment rows (2 splitters × 13 targets)
├── rust-matrix.md        # human-readable report
├── decl-splitter-build.log
└── split-decls-rs-build.log
```

**Findings:**

| Splitter | Type | Target code | Build | Runs | Notes |
|---|---|---|---|---|---|
| `decl-splitter` | Rust .rs file splitter (syn AST) | 13 targets | OK (0.1s) | 13/13 | Outputs per-declaration `.rs` files. Small test targets produce empty `_decl_module_invocation.rs` because lib.rs contains no top-level declarations. |
| `split-decls-rs` | Rust crate overlay splitter | 13 targets | OK (0.1s) | 10/13 | Outputs wrapped workspace with `src/decls/`. 3 failures due to missing external path deps when copying (`lib-introspector-core`, `cargo-lock-import`, `rustc_arena`). |

**Classification of 102 rust_split2.txt entries:**
- **True declaration splitters**: `decl-splitter`, `split-decls-rs`, `StaticSplit.lean`, `split-source.py`
- **Other data/text splitters (not comparable)**: `ragit/matrix_splitter.rs` (JSON), `split-chat` (chat logs), `monomcp/file_split*` (semantic call-graph), `pastebin/splitter.rs` (text), `swiftide/treesitter/splitter.rs` (tree-sitter trees)
- **Noise / duplicates / vendored copies**: 68 entries (clippy/rust-analyzer test data, nix-store paths, vendored deps, build artifacts)

## Shmem Cross-References

> Generated: 2026-06-23 11:10:58 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Index | ko_class_index | def |
| Index | meme_MONSTER_WALK_INDEX_COMPLETE | theorem |
| Index | bott_orbit_rotates_ko_index | theorem |
| Rust | meme_emoji_dao_rust | theorem |
| Split | splitWitnessIntoShards | def |