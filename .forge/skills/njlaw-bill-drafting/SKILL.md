# Skill: njlaw-bill-drafting

End-to-end workflow for drafting, formalizing, testing, and submitting legislation as a verified Lean4 + ZK artifact.

## What it does

Turns a markdown/txt bill draft into:

1. A statutory bill text (`draft/<name>-bill.txt`).
2. A legal framework (`draft/<name>-legal-framework.md`).
3. A Lean4 formalization skeleton (`formal/`) with sorry placeholders.
4. A Tantivy index backed by `~/archive/njleg/` + federal law corpus.
5. An `aristo-ready.flag` when all prerequisites are satisfied.
6. An Aristotle submission package (`aristotle-submissions/<name>/`).
7. An actual HTTP upload to Aristotle via the Rust `aristotle-manager` CLI.

## Required tools

| Tool | Location |
|------|----------|
| `njlaw` | `/home/mdupont/2026/06/17/njlaw/` (Rust CLI, `cargo run --bin njlaw --`) |
| `aristotle-manager` | `/home/mdupont/projects/arist/` (Rust CLI, `target/debug/aristotle-manager`) |
| Tantivy index | `~/archive/njleg/index/` |
| Bill corpus | `~/archive/njleg/downloads/BillTracking/` |
| Statutes ZIP | `~/archive/njleg/downloads/Statutes/STATUTES-TEXT.zip` |
| Aristo results | `~/projects/arist/aristotles_results/` |

## Workflow

### 1. Draft the bill

Write:

- `draft/<name>-bill.txt` — statutory text with sections, definitions, and operative clauses.
- `draft/<name>-legal-framework.md` — bylaws, governance params, tokenomics.
- `draft/<name>-proofs-lean4.md` — proof milestones (M1..M6).
- `draft/<name>-contracts-architecture.md` — smart contract layout.
- `draft/INTENT.txt` — open-invitation manifesto.

The `session-ses_*.md` file tracks the live todo list; keep it updated.

### 2. Add ZKP / compliance language to the bill

Every report/filing to a state authority must be accompanied by `ComplianceAttestation` PDAs
(quorum, treasury, timelock, investor-exemption circuits).

### 3. Build / rebuild the Tantivy index

```bash
cd /home/mdupont/2026/06/17/njlaw
cargo run --bin njlaw -- build-index --recreate
```

`--recreate` removes the old index first. Without it, the tool opens the existing index and appends.

Indexed sources (in order):

1. `~/archive/njleg/downloads/` — statutes ZIP, bill tracking HTM/TXT.
2. `~/2026/06/17/njlaw/draft/` — all DAO drafts.
3. `~/projects/arist/aristotles_results/15094d2c-.../RequestProject/Law` — federal law formalization (16 modules).
4. `~/projects/arist/aristotle-submissions/solfunmeme-dao/formal/` — NJ Lean4 formalization in progress.

### 4. Run prereq check

```bash
cargo run --bin njlaw -- prereq draft/<name>-bill.txt
```

Extracts bill keywords, searches ZIP + index. When every keyword hits and the statutes ZIP is present,
writes `draft/aristo-ready.flag`. Gates on Aristotle submission readiness.

### 5. Lean4 formalization (parallel work packages)

Under `aristotle-submissions/<name>/formal/`:

```
formal/
├── DaoGovernance.lean   -- state machine, proposal lifecycle
├── DaoProofs.lean       -- M2–M5 invariant proofs
├── Law/NJ/              -- statutory model (WP-NJ0..NJ6)
└── Zk/                  -- Circom circuits (future)
```

Each WP = dispositive-facts record → predicate → necessity lemmas → non-vacuous worked example.
Must be `sorry`-free, standard axioms only, content-addressed via `NJStatute`.

### 6. Package for Aristotle

```
aristotle-submissions/<name>/
├── SUBMISSION.md            -- project overview, submission flow
├── ARISTO-TASKS.md          -- per-WP checklist with quality gates
└── formal/                  -- Lean source tree
```

### 7. Submit to Aristotle

From `~/projects/arist/`:

```bash
cargo run --bin aristotle-manager -- formalize \
  ~/projects/arist/aristotle-submissions/<name> \
  --prompt "Fill in the sorries ..."
```

Output: `Project created: <uuid>`.

### 8. Track Aristotle results

```bash
# Poll for completed results
cargo run --bin aristotle-manager -- download --limit 10

# Check a specific project
cargo run --bin aristotle-manager -- check <project-id>

# Download completed results
cargo run --bin aristotle-manager -- download-result <project-id> --output-dir ./output
```

## Bills produced so far

| Bill | Draft file | Aristo project | Status |
|------|-----------|----------------|--------|
| SolFunMeme DAO Recognition and Compliance Act | `draft/solfunmeme-dao-bill.txt` | `5f43262f-85da-439a-a404-5433260d20ae` | Submitted — awaiting sorries |

## Key invariants

- `CorpusCompleteness`: ∀ relevant bill, bill.text ∈ BillTracking.
- `IndexConsistency`: tantivy `version = STATUTES-TEXT.zip.hash ∧ docCount ≥ statuteCount`.
- `PrereqSoundness`: `prereq(draft) = ComplianceMap ∧ unsatisfied = ∅`.
- `AristoReady`: all three above → `ARISTOTLE_READY`.
- `FormalCompleteness`: ∀ milestone ∈ {M1..M6}, proved = true.

## Skill-ABI (machine-readable invocation contract)

```yaml
skill: njlaw-bill-drafting
version: 0.1.0
inputs:
  - name: draft_name
    type: string
    format: utf-8
    validation: "^[a-z0-9-]+$"
preconditions:
  - "~/archive/njleg/ exists with downloads/"
  - "~/projects/arist/ exists"
  - "njlaw binary builds"
postconditions:
  - "draft/aristo-ready.flag exists"
  - "aristotle-manager formalize returns Project created: <uuid>"
side_effects:
  - writes Tantivy index at ~/archive/njleg/index/
  - writes aristo-ready.flag
  - HTTP POST to https://aristotle.harmonic.fun/api/v3/project
security:
  auth: API key in ARISTOTLE_API_KEY env
  replay_protection: Aristotle deduplicates by project dir hash
```

## Errors and recovery

| Error | Cause | Fix |
|-------|-------|-----|
| `422 File must be a valid gzipped tar file` | `formalize` sent plain tar | Patch `cmd_formalize` to gzip bytes (already done in `~/projects/arist/src/main.rs`) |
| `Index locked / Schema mismatch` | Stale index after schema change | Re-run with `--recreate` |
| `aristo-ready.flag missing` | Index missing a keyword | Check `search_index` output in prereq report, add corpus |
| `Project already exists` | Same dir re-uploaded | Tar directory name in `aristotle_processed.txt` blocks re-submit; remove line to retry |

## Shmem Cross-References

> Generated: 2026-06-23 10:20:01 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Add | zero_add | lemma |
| Add | tan_div_sqrt_one_add_tan_sq | theorem |
| Add | coeAddHom | def |
| Build | buildPrefixTree | def |
| ZKP | computeZkpCommitment | def |
| ZKP | zkperf_witness_verifies | theorem |