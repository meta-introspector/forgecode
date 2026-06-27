---
name: aristo-consolidation
description: "Use when consolidating the Aristotle split pipeline — merging Python/shell splitting tools into the Rust CLI, unifying cmd_split/cmd_split_all, porting DASL post-processors, and deleting legacy scripts. Trigger when working on split commands, dedup, DASL pipeline, or the consolidation plan at ~/dotagents/tasks/split-tools-consolidation-plan.md."
---

# Aristotle Split Tools Consolidation

## Goal

Move ALL splitting, dedup, DASL pipeline, and orchestration logic from Python/shell
into Rust. Lean (`SplitDecls.lean`) remains the kernel-level engine. Python/shell
survive only as thin throwaway helpers during transition.

## Current State (2026-06-22)

| Layer | Count | Language |
|-------|-------|----------|
| Core engine | 1 (`SplitDecls.lean`) | Lean4 — **never port to Rust** |
| Shell wrappers | 3 scripts | bash — being deleted |
| Python splitters/dedup | 3 scripts | Python — being ported |
| DASL post-processors | 7 scripts | Python — being ported |
| Rust CLI split commands | 7 commands | Rust — being unified |

## Public CLI Target State

| Command | Absorbs | Status |
|---------|---------|--------|
| `split --all` | `split-all`, `split-aristotle-project.sh` | Planned |
| `split --source` | `split-source.py` | Planned |
| `split --module` | `split-lean-project.sh` | Planned |
| `merge --dedup-only` | `dedup-split.py` | Planned |
| `pipeline --skip-*` | `refresh`, `replay` | Planned |
| `dasl build-module` | `build-dasl-module.py` | Planned |
| `dasl build-unified` | `build-unified-dasl.py` | Planned |
| `dasl filter` | `dasl-term-filter.py` | Planned |
| `dasl fix-imports` | `fix-unified-imports.py` | Planned |
| `dasl stratify` | `prime-stratify.py` | Planned |
| `dasl partition` | `partition-dasl.py` | Planned |
| `locate2proof` | `locate2proof-collect.py` | Planned |
| `submit --chunked` | `submit-chunks.py` + `submit-to-aristotle.py` | Planned |

## Private Repo Target State

```
private/
├── splitter-engine/          # KEPT — SplitDecls.lean + finders
├── scripts/ (DELETE after porting)
│   ├── split-aristotle-project.sh
│   ├── split-lean-project.sh
│   ├── dedup-split.sh
│   ├── refresh.sh
│   ├── run_notebooklm_all.sh
│   └── dasl-pipeline/*.py
└── data/                     # aristotle_results, split-results, mathlib-split
```

## Commands to DELETE from Public CLI

| Variant | Replacement |
|---------|-------------|
| `Commands::Refresh` | `pipeline --skip-version --skip-merge` |
| `Commands::Replay` | `pipeline --from-archive <dir>` |
| `Commands::SplitAll` | `split --all` |

## Execution Phases

1. **Phase 0** — Shared types (`src/split/types.rs`, `src/dasl/types.rs`) + module stubs
2. **Phase 1** — `cmd_split` unification (absorb `split-all`, `split-source.py`, shell wrappers)
3. **Phase 2** — `cmd_merge` + dedup (absorb `dedup-split.py`)
4. **Phase 3** — `cmd_pipeline` unification (absorb `refresh`, `replay` via flags)
5. **Phase 4** — `cmd_dasl` with 7 subcommands (absorb all DASL Python scripts)
6. **Phase 5** — `cmd_locate2proof`
7. **Phase 6** — `cmd_submit --chunked`
8. **Phase 7** — Delete all Python/shell scripts, update Makefile/docs

**Total: ~17 days.** Phase 4 can be parallelized.

## Rules

- **Never port SplitDecls.lean to Rust.** It is the kernel engine.
- **DASL post-processors should be private.** They operate on private data shapes.
- **Delete scripts only after Rust version produces identical output.**
- **Keep `split-source.py` logic as `split --source` fallback mode** (no lake/mathlib deps).
- **Every new command must have `--dry-run`.**

## Verification Checklist

After each phase:
- [ ] `cargo build --release` succeeds
- [ ] `cargo test` passes (add tests for each ported function)
- [ ] `cargo run -- <command> --help` shows correct docs
- [ ] Rust output matches old Python output on sample data
- [ ] Old script deleted only after parity confirmed

## Per-Tool Task Directories

Each tool has its own task directory under `tasks/tools/`:
```
tasks/tools/
├── split-decl-lean/           # Core engine (never port)
├── split-aristotle-project-sh/ # -> cmd_split --all
├── split-lean-project-sh/     # -> cmd_split --module
├── refresh-sh/                # -> cmd_pipeline (absorbed)
├── split-source-py/           # -> cmd_split --source
├── dedup-split-py/            # -> cmd_merge --dedup-only
├── dedup-split-sh/            # delete
├── build-dasl-module-py/      # -> cmd_dasl build-module
├── build-unified-dasl-py/     # -> cmd_dasl build-unified
├── dasl-index-scanner-py/     # -> cmd_scan_index (done)
├── dasl-term-filter-py/       # -> cmd_dasl filter
├── fix-unified-imports-py/    # -> cmd_dasl fix-imports
├── prime-stratify-py/         # -> cmd_dasl stratify
├── partition-dasl-py/         # -> cmd_dasl partition
├── cmd-split/                 # unify: add --all/--source/--module
├── cmd-split-all/             # fold into cmd_split --all
├── cmd-merge/                 # add --dedup-only
├── cmd-decl-table/            # keep
├── cmd-refresh/               # fold into pipeline
├── cmd-pipeline/              # add --skip-* / --from-archive
├── cmd-replay/                # fold into pipeline --from-archive
├── makefile-split/            # thin wrappers, keep
└── run-notebooklm-all-sh/     # -> notebooklm --all
```

## Key Docs

- Full plan: `~/dotagents/tasks/split-tools-consolidation-plan.md`
- Inventory: `~/dotagents/tasks/split-tools-inventory-merge-plan.md`
- Per-tool tasks: `~/dotagents/tasks/tools/<tool-name>/`
- Public README: `~/2026/06-june/26/aristotle-cli-rs/README.md`
- Private README: `/mnt/data1/time-2026/05-may/07/arist/README.md`

## Shmem Cross-References

> Generated: 2026-06-23 11:10:58 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| CLI | bott_cycling_correct | theorem |
| CLI | meme_clifford_fractran_monster | theorem |
| Execution | ordered_execution_precedence_correct | theorem |
| Repo | meme_emoji_dao_agreements_report | theorem |
| Repo | meme_emoji_agreements_report | theorem |
| Split | splitWitnessIntoShards | def |
| State | ssp_states_do_not_collapse | theorem |