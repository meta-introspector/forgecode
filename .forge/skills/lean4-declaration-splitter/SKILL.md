---
name: lean4-declaration-splitter
description: >-
  Rust+Lean pipeline for splitting Lean4 modules into per-declaration flakes.
  Replaces Python static_split.py with aristotle-manager Rust CLI driving
  SplitDecls.lean (kernel API) and StaticSplit.lean (regex). Use when
  splitting Lean codebases, running Aristotle batch splits, or extending
  the splitter engine.
---

# lean4-declaration-splitter

## one-liner
```bash
cd ~/projects/arist && cargo run --release -- split
```

## Trigger
When working on Lean declaration splitting, per-declaration flakes,
Aristotle result management, or the splitter-engine.

## Approach
1. Rust driver (`aristotle-manager`) discovers projects and invokes Lean splitter
2. Lean splitter (`SplitDecls.lean`) uses Environment API for exact dependency tracking
3. Static regex mode (`StaticSplit.lean`) available for no-build scenarios
4. All Makefile targets route through `cargo run --release`

## See Also
- [[skills/aristotle-manager]]
- [[skills/aristotle-splitter]]
- [[tasks/lean4-2-category-formalization]]

## Shmem Cross-References

> Generated: 2026-06-23 10:19:59 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| — | No matches in shmem for 2 keywords | — |
| — | Searchable terms: SplitDecls.lean, StaticSplit.lean | — |