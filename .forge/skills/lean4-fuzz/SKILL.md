---
name: lean4-fuzz
description: >-
  Formal fuzz verification in Lean4. FuzzHarness (CBOR-guided, 19-way term gen,
  xorshift64 PRNG, 5 target functions, topological lattice), FuzzLattice
  (coverage inclusion, monotone fuzz_eval, fixpoint), FuzzWitness (3 theorems:
  orbifold_valid, moduli_are_ssp, hecke_index_is_ssp), Counterexample,
  GroupFuzz (8 density theorems), DASL/FuzzCorpus (352-line multi-lang spec),
  Plausible (Sampleable/Testable/Functions). Use when: formalizing fuzz
  properties in Lean4, proving coverage monotonicity, generating fuzz
  witnesses with ZK properties, or connecting operational fuzz to formal proofs.
---

# lean4-fuzz — Formal Fuzz Verification in Lean4

**Sources:**
- `~/kiro.el-research/fractran_vm/FuzzLattice.lean`
- `~/aristotle-results/.../FuzzHarness.lean`
- `~/aristotle-results/.../FuzzWitness.lean`
- `~/dasl-planning/dasl-architecture/DASL/FuzzCorpus.lean`
- `lean-split-tool/merged-aristotle-output/GroupFuzz*.lean`
- `lean-split-tool/merged-aristotle-output/Counterexample.lean`

## Modules

### FuzzHarness.lean (350 lines)
CBOR-guided fuzz harness:
- xorshift64 PRNG
- 19-way random MetaCoq term generator (all term constructors)
- Term mutation for corpus expansion
- Coverage analysis: `collectTags` tree walk, `isNovelCoverage` filter
- 5 target functions: `Term.size`, `.depth`, `.freeVars`, `.tag`, `CBOR.roundTrip`
- `topologicalLattice` controller → `fullFuzzRun` across all targets

### FuzzLattice.lean (80 lines)
Fuzz state lattice:
- `FuzzState` with corpus, coverage, log
- Lattice order: `s ≤ t` iff coverage(s) ⊆ coverage(t)
- `fuzz_eval` — monotone operator (one step)
- `topological_lattice` — fixpoint iteration
- `fuzz_all` — run for all functions

### FuzzWitness.lean (120 lines, 3 theorems)
Property-based testing witness:
- `FuzzVerdict`: PropertyHolds | ShadowDetected | CounterexampleFound
- `ShardMetadata` with eRDFa/DASL annotations (shard, encoding, addr, eigenspace, bott, hecke, orbifold)
- **Theorem 1:** `defaultShardMetadata_orbifold_valid` — (58<71, 28<59, 10<47)
- **Theorem 2:** `defaultShardMetadata_moduli_are_ssp` — {71,59,47} ∈ supersingularPrimes
- **Theorem 3:** `hecke_index_is_ssp` — 29 is supersingular prime
- `FuzzWitness.isValid` — orbifold coords respect their moduli

### GroupFuzz.lean (8 theorems)
Group fuzzing coverage density:
- `coverage_le_sessions`, `density_le_one`, `density_nonneg`
- `empty_sessions_empty_coverage`, `full_coverage_density`
- `hasEpsilonCoverage`, `hasFullCoverage`

### Counterexample.lean
Counterexample generation from property failures.

### DASL/FuzzCorpus.lean (352 lines)
DASL fuzz corpus formalization:
- `Lang`: Rust, Go, Python, JavaScript, Java, C, Cpp
- `Fuzzer`: Honggfuzz
- Full 5-phase pipeline spec

### Testing/Plausible/
Property-based testing framework:
- `Sampleable` — generate random values
- `Testable` — check properties
- `Functions` — function generation

## Proven Theorems

```lean4
-- FuzzWitness.lean
theorem defaultShardMetadata_orbifold_valid :
    ∀ i : Fin 3, defaultShardMetadata.orbifoldCoords i < defaultShardMetadata.orbifoldModuli i := by
  intro i; fin_cases i <;> native_decide

theorem defaultShardMetadata_moduli_are_ssp :
    ∀ i : Fin 3, defaultShardMetadata.orbifoldModuli i ∈ supersingularPrimes := by
  intro i; fin_cases i <;> native_decide

theorem hecke_index_is_ssp : defaultShardMetadata.heckeIndex ∈ supersingularPrimes := by native_decide

-- NumericalConstants.lean (dasl-planning)
theorem tag_prefix_formula : ((MAJOR_TAG : ℕ) <<< TAG_MAJOR_SHIFT) ||| TAG_1BYTE_FOLLOWS = TAG_PREFIX_BYTE := by native_decide
theorem cid_tag_is_42 : TAG_CID_NUMBER = 42 := rfl
```

## Related

- **gpu-shmem-query** — GPU FRACTRAN queries (same pipeline, trivector proofs)
- **ipld-core-fuzz** — 6-engine operational fuzzing
- **dasl-testing-crosslang** — 18 harnesses, 7 languages
