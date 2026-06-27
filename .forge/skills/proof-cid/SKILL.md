---
name: proof-cid
description: >-
  Save every Lean4 proof step as a content-addressed CID in shmem.
  Decompose proofs into atoms (theorem, goal, tactic, rewrite, subgoal, qed),
  store the proof DAG, generate SPARQL queries for introspection, and
  translate to natural language. Use when CID-addressing Lean4 proofs.
---

# proof-cid — CID-addressed proof DAGs

## one-liner
```bash
python3 ~/bin/proof-cid store --theorem load_bearing_myth
```

## Trigger
When saving proofs as content-addressed data, building proof DAGs,
generating SPARQL views of proof structure, or translating proofs.

## Commands

| Command | What |
|---------|------|
| `extract` | Extract proof terms from Lean4 modules |
| `store` | Decompose proof into CID-addressed steps |
| `introspect` | Show all known proofs and relationships |
| `query` | Show SPARQL queries for proof inspection |
| `translate` | Translate proof to natural language |

## Proof Atom Types

| Kind | CID Content | Example |
|------|-------------|---------|
| `theorem` | Full theorem statement | "load_bearing_myth : 196883 + 1 = 196884" |
| `goal` | Goal to prove | "196883 + 1 = 196884" |
| `tactic` | Tactic application | "decide" |
| `rewrite` | Term transformation | "196883+1=196884 → true" |
| `subgoal` | Sub-goal generated | "Nat.Prime 2 ∧ Nat.Prime 3 ∧ ..." |
| `qed` | Proof complete | "QED — done" |

## SPARQL Queries

```sparql
# All proof steps for a theorem
SELECT ?step ?kind ?tactic WHERE {
  ?step :kind ?kind .
  OPTIONAL { ?step :usesTactic ?tactic }
}

# Step chain (proof flow)
SELECT ?a ?b WHERE { ?a :then ?b }

# Tactic usage statistics
SELECT ?tactic (COUNT(?step) as ?count) WHERE {
  ?step :usesTactic ?tactic
} GROUP BY ?tactic

# Is the proof complete?
ASK { ?step :kind "qed" }
```

## See Also
- [[skills/shmem-sparql]]
- [[skills/dasl-atomize]]
- [[skills/shmem2disk]]

## Shmem Cross-References

> Generated: 2026-06-23 10:20:01 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| All | norm_expSeries_div_summable_of_mem_ball | theorem |
| All | le_iff_forall_rat_lt_imp_le | theorem |
| All | exists_forall_hasDerivAt_Ioo_eq_of_contDiff | theorem |
| CID | computeCID | def |
| Proof | meme_fractran_proof | theorem |
| Proof | meme_direct_proof | theorem |
| Proof | meme_fractran_proof_nix | theorem |
| Step | fractran_step | def |