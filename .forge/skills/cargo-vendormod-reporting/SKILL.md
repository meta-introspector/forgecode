---
name: cargo-vendormod-reporting
description: Generate, process, and verify reports and manifests with cargo-vendormod. Use for report generation, goal tracking, coverage summaries, gitmodules metadata, rollup locks, Cargo.toml fixes, and DASL metadata processing.
---

# Cargo-Vendormod Reporting

## Generate Reports

```bash
cargo run --bin processing -- report \
  --workspace-path /path/to/workspace \
  --output-dir ./report
```

## Metadata / Gitmodules

```rust
use cargo_vendormod::gitmodules_metadata;
use cargo_vendormod::rollup_lock::RollupLock;
```

## Goals / Tracking

```rust
use cargo_vendormod::goal_tracker;
```

## Coverage Summary

```rust
use cargo_vendormod::coverage;
```

## DASL Metadata

```rust
use cargo_vendormod::dasl_metadata_processor;
```

## Report Processing

```rust
use cargo_vendormod::report_processing;
use cargo_vendormod::report_generator;
```

## Fix Cargo.toml

```bash
cargo run --bin vendormod -- fix-cargo-toml --manifest-path ./Cargo.toml
```

## Key Modules

`report_generator`, `report_processing`, `goal_tracker`, `coverage`, `gitmodules_metadata`, `rollup_lock`, `fix_cargo_toml`, `dasl_metadata_processor`.

## Shmem Cross-References

> Generated: 2026-06-23 11:12:30 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| DASL | writeDaslFile | def |
| DASL | meme_DASL2_LITERATE | theorem |
| DASL | DaslItem | structure |
| Fix | _root_.Set.conj_mem_fixingSubgroup | theorem |
| Fix | exists_fixed | theorem |
| Fix | writePrefixTree | def |
| Generate | generateLeech | def |
| Generate | generateDatagram | def |
| Generate | generateShardSummary | def |
| Gitmodules | meme__gitmodules | theorem |
| Modules | meme__gitmodules | theorem |
| Report | meme_emoji_dao_agreements_report | theorem |
| Report | meme_emoji_agreements_report | theorem |
| Summary | generateShardSummary | def |