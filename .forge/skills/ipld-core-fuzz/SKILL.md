---
name: ipld-core-fuzz
description: >-
  6-engine DAG-CBOR fuzzing for ipld-core. AFL++ (234M execs, 52.9% coverage,
  0 crashes), Honggfuzz, libFuzzer, Bolero (property-based), Ziggy, Fuzzcheck
  (structural) — all sharing a single FuzzTarget trait + macro. 5 fuzz targets,
  2035 unique corpus inputs, 701-line coverage.lcov, Nix-based afl.rs builds.
  Use when: fuzzing ipld-core, running DAG-CBOR decode fuzzers, managing
  multi-engine fuzz corpus, or extending coverage of IPLD decoders.
---

# ipld-core-fuzz — 6-Engine DAG-CBOR Fuzzing

**Crate:** `~/dasl/rust/ipld-core/fuzz/`  
**Package:** `ipld-core-fuzz`  
**Nix:** `flake.nix` (afl.rs, honggfuzz, cargo-fuzz)

## Quick Start

```bash
cd ~/dasl/rust/ipld-core

# AFL++ (52.49% coverage, 0 crashes)
cd fuzz && AFL_SKIP_CPUFREQ=1 AFL_EXIT_WHEN_DONE=1 cargo afl fuzz -i afl-in -o hfuzz_workspace/fuzz_target_1/afl_out -- ./target/debug/fuzz_target_1

# libFuzzer
cd fuzz && cargo fuzz run fuzz_target_1

# Fuzzcheck (default, structural-aware)
cd fuzz && cargo test fuzz_target_fuzzcheck -- --test-threads=1

# All engines via Makefile
make afl-default
make fuzz-honggfuzz2
```

## Engines

| Engine | Feature | Executions | Coverage | Crashes |
|--------|---------|-----------|----------|---------|
| AFL++ 4.40c | `afl` | 234,616,764 | 52.90% | 0 |
| Honggfuzz | `honggfuzz` | — | — | — |
| libFuzzer | `libfuzzer` | — | — | — |
| Bolero | `bolero` | — | — | — |
| Ziggy | `ziggy` | — | — | — |
| Fuzzcheck | `fuzzcheck` (default) | 100K/fuzz | — | — |

## Architecture

```
fuzz/
├── Cargo.toml         — 6 feature flags, 5 [[bin]] + 3 [[test]] targets
├── src/lib.rs          — FuzzTarget trait + fuzz_target! macro
├── fuzz_targets/
│   ├── fuzz_target_1.rs              — main decode fuzzer
│   ├── fuzz_target_honggfuzz.rs      — honggfuzz variant
│   ├── fuzz_target_bolero.rs         — bolero property-based
│   ├── fuzz_target_fuzzcheck.rs      — fuzzcheck structural
│   └── fuzz_target_1_ziggy.rs        — ziggy variant
├── bin/
│   ├── import_corpus.rs              — corpus import tool
│   └── gen_inputs.rs                 — structured input generator
├── afl.rs/            — AFL.rs crate (Nix build)
├── cargo-fuzz/        — cargo-fuzz v0.13.1
├── honggfuzz/         — dedicated honggfuzz workspace
├── corpus/            — 2,035 unique inputs
├── coverage.lcov      — 701 lines LCOV
└── Makefile           — afl-default, afl-clean, afl-force, fuzz-honggfuzz2
```

## Shared FuzzTarget Trait

```rust
pub trait FuzzTarget {
    type Input: for<'a> arbitrary::Arbitrary<'a>;
    fn fuzz(input: Self::Input);
}
```

One `fuzz_target!` macro dispatches to the correct engine at compile time.

## Fuzz Target

DAG-CBOR decode:
```rust
impl FuzzTarget for Decode {
    type Input = Vec<u8>;
    fn fuzz(input: Self::Input) {
        if input.len() > 1024 * 1024 { return; }
        let _ = serde_ipld_dagcbor::from_slice::<Ipld>(&input);
    }
}
```

## Related

- **gpu-shmem-query** — Monster lattice + FRACTRAN GPU queries (same IPLD pipeline)
- **dasl-testing-crosslang** — 18 harnesses across 7 languages
- **lean4-fuzz** — formal fuzz verification in Lean4
