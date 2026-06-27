---
name: dasl-testing-crosslang
description: >-
  Cross-language DASL/CBOR decoder conformance testing. 18 harnesses across
  7 languages (C, Rust, Go, Python, JS, Java, C++) plus libipld, n0_dasl,
  boxo. 1961 crash entries cataloged. Round-robin cross-implementation
  divergence testing (round_robin.py). 5-phase test strategy: spec-derived,
  fuzzer-generated, synthetic, native, live apps. Coverage measurement
  pipeline (prove-coverage.sh). Use when: testing DAG-CBOR decoder conformance,
  comparing implementations, running cross-language divergence tests,
  or verifying spec compliance across ecosystems.
---

# dasl-testing-crosslang — Cross-Language Decoder Conformance

**Repo:** `~/dasl/dasl-testing/`  
**Planning:** `~/dasl-planning/`  

## Quick Start

```bash
cd ~/dasl/dasl-testing

# Cross-implementation round-robin
python3 round_robin.py

# Coverage measurement pipeline
bash ~/dasl-planning/prove-coverage.sh

# Run specific harness
cd harnesses/c-cbor && make test      # C: tinycbor, libcbor, libcbrrr
cd harnesses/serde_ipld_dagcbor && cargo test  # Rust
cd harnesses/go-ipld-cbor && go test ./...      # Go
cd harnesses/python && python3 fuzz.py          # Python
cd harnesses/js && node fuzz.js                 # JavaScript

# Aggregate results
python3 aggregate_car_files.py
python3 create_car_from_crashes.py
```

## Harnesses (18 total)

| Harness | Language | Implementations |
|---------|----------|----------------|
| `c-cbor` | C | tinycbor, libcbor, libcbrrr |
| `serde_ipld_dagcbor` | Rust | serde_ipld_dagcbor |
| `rust-cbor` | Rust | Additional Rust CBOR |
| `go-dasl` | Go | DASL Go |
| `go-ipld-cbor` | Go | go-ipld-cbor |
| `python` | Python | python-libipld (Atheris-ready) |
| `python-ipld-core` | Python | ipld-core Python bindings |
| `js` | JavaScript | js-dag-cbor (jsfuzz-ready) |
| `java-dag-cbor` | Java | java-dag-cbor |
| `cpp-glaze` | C++ | Glaze CBOR |
| `libipld` | Rust | libipld core |
| `n0_dasl` | Rust | n0 DASL decoder |
| `boxo` | Go | Boxo IPLD |
| `fuzz-team-tile` | Meta | Fuzzing team tile |
| `qa-team-tile` | Meta | QA team tile |

## Test Strategy (5 Phases)

| Phase | Source | Description |
|-------|--------|-------------|
| 1 | Spec-derived | RFC 8949 boundary values, all major types, tag 42 CID, deterministic encoding |
| 2 | Fuzzer-generated | AFL++, libFuzzer, Honggfuzz, Bolero corpus |
| 3 | Synthetic | Cross-implementation divergence cases |
| 4 | Native | Each implementation's own test suite |
| 5 | Live apps | Running applications as test targets |

## Key Files

| File | Purpose |
|------|---------|
| `round_robin.py` | Cross-implementation divergence testing |
| `coverage_base.py` | Base coverage measurement |
| `aggregate_car_files.py` | Aggregate results into CAR format |
| `create_car_from_crashes.py` | Convert crashes to CAR |
| `lattice_generator.py` | Generate fuzz lattice inputs |
| `harnesses/crash_inventory.json` | 1961 crash entries cataloged |
| `harnesses/fuzzing_inventory.json` | 106 fuzz harness entries |
| `FUZZING_ROLLUP_REPORT.md` | Multi-session rollup (234M+ execs) |
| `AFL_FUZZING_REPORT.md` | AFL setup + results |

## Planning Docs (`~/dasl-planning/`)

| Doc | Content |
|-----|---------|
| `02-fuzzing-plan.org` | Multi-tool fuzz strategy across 7 languages |
| `03-constant-invariant-extraction.org` | Constant + invariant extraction |
| `04-test-coverage.org` | Cross-implementation conformance spec |
| `PROOF_OF_COVERAGE.md` | 9-section coverage proof: ipld.txt → Lean4 |
| `NumericalConstants.lean` | All DASL/CBOR/CID constants with theorems |
| `SPEC.org` | 23-line formal pipeline spec |
| `prove-coverage.sh` | Coverage measurement pipeline |
| `corpus-pipeline.org` | 5-phase corpus pipeline |

## Related

- **ipld-core-fuzz** — 6-engine operational fuzzing
- **lean4-fuzz** — formal fuzz verification
- **gpu-shmem-query** — Monster lattice (same IPLD pipeline)
