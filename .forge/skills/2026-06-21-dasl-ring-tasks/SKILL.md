# DASL Ring Pipeline — Community Tasks

Date: 2026-06-21  
Status: Seeking contributors  
Repo: `https://solana.solfunmeme.com/dasl-planning`  
Docs: `~/DOCS/DASL_RING_ANALYSIS.md`

## Overview

The DASL codebase forms concentric **semantic rings** around a core specification:

```
Ring 1 (Lean4 proofs)     → 1,762 declarations formalizing Monster, Sheaf, CID, etc.
Ring 2 (Rust impls)       → 198 types: CarShmemClient, AstNode*, protocol types
Ring 3 (Python services)   → 232 functions: tile servers, plan parsers, MCP tools
Ring 4 (Go/JS/C drivers)   → NOT YET SCANNED — needs volunteers
```

We need to grow the ring map from 2,365 terms to the full 500K+ term corpus,
fill Ring 4, and close the spec→proof→impl alignment gaps.

## Tasks (pick one, run the script, report results)

### 🔴 HIGH PRIORITY — Close the Ring Gaps

#### T1: Full Lean Corpus Scan
**Goal**: Run staticsplitjson on all 495K .lean files, push to shmem
**Input**: `~/2026/06-june/26/index/lean.index2.txt`
**Script**: `~/2026/06-june/26/index/scan_focused.py` (modify TARGET_DIRS)
**Deliverable**: shmem CID for the full Lean term corpus
**Effort**: ~4 hours (rate-limited by staticsplitjson, ~50 files/sec)

```bash
# Batch scan approach:
cd ~/2026/06-june/26/index
cat lean.index2.txt | grep '\.lean$' | grep -v '/\.lake/' | grep -v '/nix-store/' | \
  xargs -P 8 -n 1 staticsplitjson >> all-lean-terms.jsonl
```

#### T2: Full Rust Crate Extraction
**Goal**: Extract all Rust terms from aristo, nora, vendormod, forgecode, pipelight
**Script**: `~/2026/06-june/26/index/ring_focused.py` (add RUST_DIRS entries)
**Deliverable**: PR adding Rust terms to dasl-ring-map.json
**Effort**: ~30 min

```bash
# Find all Cargo.toml crates:
find /mnt/data1/time-2026 -name "Cargo.toml" -not -path "*/target/*" | \
  while read cargo; do dir=$(dirname "$cargo"); echo "$dir"; done > rust-crates.txt
```

#### T3: Ring 4 — Go/JS/C/Java Driver Scan
**Goal**: Extract terms from DASL's cross-implementation test harnesses
**Script**: Extend `ring_focused.py` with Go/JS/C extractors
**Deliverable**: dasl-ring-map.json updated with ring4 data
**Effort**: ~1 hour

#### T4: Missing Formalizations → Aristotle
**Goal**: Submit the 6 concepts with Lean proofs but no Rust impl as Aristotle tasks
**Input**: Gap list from `DASL_RING_ANALYSIS.md`
**Command**: `aristotle-manager submit "<concept>" --project-dir <dir>`
**Deliverable**: 6 Aristotle project IDs, each with a formalization task

### 🟡 MEDIUM PRIORITY — Improve the Bridge

#### T5: Embedding-Based NLP Bridge
**Goal**: Replace trigram Jaccard with sentence-transformers for cross-language matching
**Why**: Current trigram NLP found 0 fuzzy matches (naming gap too large)
**Approach**: `pip install sentence-transformers`, embed all term names,
              cosine similarity between Lean and Rust terms
**Script**: Create `ring_embed.py` in `~/2026/06-june/26/index/`
**Deliverable**: dasl-ring-embed-map.json with semantic bridges
**Effort**: ~2 hours

#### T6: Visual Ring Diagram (SVG/D3)
**Goal**: Interactive ring visualization showing spec→proof→impl bridges
**Input**: `~/2026/06-june/26/index/dasl-ring-map.json`
**Deliverable**: HTML page with D3.js concentric ring viz, deployable as tile
**Effort**: ~2 hours

### 🟢 LOW PRIORITY — Quality & Documentation

#### T7: Type Conflict Resolution
**Goal**: Fix the 2 type conflicts found in dedup scan
- `cliffordArea`: `def` vs `opaque` in two Theories.lean versions
- `Pipeline`: `structure` vs `inductive` in SelfModelFlow vs SemanticDupFinder
**Deliverable**: PR resolving conflicts or documenting why they differ

#### T8: Meme Deduplication
**Goal**: Merge the 4 meme file pairs (hyphen vs underscore naming)
- `emoji-dao-rust.lean` / `emoji_dao_rust.lean`
- `monster-codebook.lean` / `monster_codebook.lean`
- etc.
**Deliverable**: Consolidated single files, removed duplicates

#### T9: Run Ring Pipeline Daily
**Goal**: Add `ring_focused.py` to system-manager as a daily service
**Deliverable**: system-manager config + tile showing live ring stats
**Effort**: ~30 min

## How to Contribute

```bash
# 1. Clone the data
git clone https://solana.solfunmeme.com/dasl-planning
cp ~/2026/06-june/26/index/dasl-ring-map.json .

# 2. Pick a task from above

# 3. Run the relevant script

# 4. Submit results
git add <your-output>
git commit -m "T<N>: description"
git push
```

## Reference

| Resource | Path |
|----------|------|
| Ring analysis doc | `~/DOCS/DASL_RING_ANALYSIS.md` |
| Ring data | `~/2026/06-june/26/index/dasl-ring-map.json` |
| Lean dedup | `~/2026/06-june/26/index/dedup-focused.json` |
| Full lean index | `~/2026/06-june/26/index/lean.index2.txt` (3M lines) |
| Ring pipeline | `~/2026/06-june/26/index/ring_focused.py` |
| Dedup scanner | `~/2026/06-june/26/index/dedup_scan.py` |
| staticsplitjson | `/nix/store/hagadmgn61fhbxdq2md6p1jjb5plb52v-staticsplitjson/bin/staticsplitjson` |
| aristotle-manager | `/mnt/data1/time-2026/05-may/07/arist/target/release/aristotle-manager` |

## Shmem Cross-References

> Generated: 2026-06-23 11:10:55 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| CID | computeCID | def |
| Close | _root_.IsOpen.measure_eq_iSup_isClosed | theorem |
| Close | _root_.MeasurableSet.exists_isClosed_diff_lt | theorem |
| Close | _root_.MeasurableSet.exists_lt_isClosed_of_ne_top | theorem |
| DASL | writeDaslFile | def |
| DASL | meme_DASL2_LITERATE | theorem |
| DASL | DaslItem | structure |
| LOW | Real.tendsto_atTop_csInf_of_antitoneOn_bddBelow_nat_Ici | theorem |
| LOW | Real.isGLB_of_tendsto_antitoneOn_bddBelow_nat_Ici | theorem |
| Meme | meme_austria_mechanical_exhibition | theorem |
| Meme | meme_encoded_message | theorem |
| Meme | meme_bach_bwv_nix | theorem |
| Monster | monsterPrime | def |
| Monster | meme_clifford_fractran_monster | theorem |
| Monster | meme_MONSTER_WALK_INDEX_COMPLETE | theorem |
| Reference | self_reference_transport_preserves_mod_71_eq_0 | theorem |
| Ring | coe_ringHom_injective | theorem |
| Rust | meme_emoji_dao_rust | theorem |
| Type | ClaimType | inductive |