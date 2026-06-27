# SolFunMeme DAO — Skill Reference

## Overview

SolFunMeme DAO is a New Jersey–registered limited‑liability DAO operating on Solana. It combines:
- Statutory legal‑entity recognition under NJ Title 42 (RULLCA supplement).
- Smart‑contract governance (propose → vote → execute).
- Lean4 formal proofs for every invariant.
- ZK‑SNARK compliance proofs that let the DAO prove regulated behavior without revealing secrets.
- A forking/guest‑DAO model so other meme communities can spin up sovereign DAOs off the same software stack.

## Key Documents

| Document | Path (relative to repo root) |
|---|---|
| Legal framework (bylaws + entity act) | `draft/solfunmeme-dao-legal-framework.md` |
| Smart-contract architecture | `draft/solfunmeme-dao-contracts-architecture.md` |
| Implementation plan | `draft/dao-implementation-plan.md` |
| Lean4 proof milestones | `draft/solfunmeme-dao-proofs-lean4.md` |
| NJ Digital Asset Bill Analysis | `draft/nj-digital-asset-bill-analysis.md` |
| DAO bill draft text | `draft/solfunmeme-dao-bill.txt` |

## Smart-Contract Stack

- **Runtime**: Solana
- **Language**: Rust (Anchor)
- **Token**: SPL Token + Token‑2022 (Transfer Hook)
- **Governance**: `solfunmeme-dao-governance`
  - `initialize`, `create_proposal`, `cast_vote`, `execute_proposal`, `cancel_proposal`, `emergency_pause`
- **Treasury**: `solfunmeme-dao-treasury`
  - PDA‑owned multisig; N‑of‑M signatures required for transfers
- **Membership**: Token‑2022 `Transfer Hook` emits membership events; PDA ledger maps `Pubkey → Timestamp + balance`

### Governance Parameters (canonical defaults)
- Quorum: 20 % of outstanding supply
- Ordinary resolution: simple majority
- Special resolution: 67 % supermajority
- Constitutional amendment: 90 % supermajority
- Voting window: ≥ 72 h
- Timelock: 7 days (contract upgrades), 24 h (emergency)
- Spending tiers: <$10k → majority, $10k–$100k → 67 %, >$100k → 90 %

## Lean4 Formal Verification

Source modules (target filenames):

| Module | Purpose |
|---|---|
| `DaoGovernance.lean` | State machine, proposal lifecycle, quorum invariants |
| `DaoProofs.lean` | Tally soundness, treasury conservation, timelock liveness |
| `DaoExtract.lean` | Codegen: compiles verified predicates to Solana BPF / Rust |

### Proof Targets (M1 – M5)

| Milestone | Theorem Sketch |
|---|---|
| M1 State Machine | `transfer : DaoState → DaoState` respects invariants |
| M2 Voting Invariants | `quorum_sufficient`, `no_double_vote`, `tally_sound` |
| M3 Treasury Conservation | `total_outflow ≤ treasury_balance` and multisig threshold satisfied |
| M4 Upgrade Safety | `execute_upgrade → proposal.passed ∧ now ≥ timelock_end` |
| M5 Liability Rules | `good_faith_vote → ¬liable` with personal-guarantee + securities exceptions |

## ZKP Compliance Proofs

The DAO issues `ComplianceProof` (Groth16 or PLONK) objects embedded in on‑chain attestations. Circuits:

| Circuit | Public Inputs | Private Inputs | Use |
|---|---|---|---|
| `quorum.circom` | `proposalId`, `totalSupply`, `turnout` | merkle proof of each voter’s weight | Validates quorum without revealing voters |
| `treasury_guard.circom` | `proposalId`, `spendAmount`, `treasuryBalance` | multisig key set, signatures | Validates spend limit + N‑of‑M |
| `timelock.circom` | `proposalId`, `executionSlot` | `timelockEndSlot` | Validates timelock elapsed |
| `exemption_eligibility.circom` | `investorAccreditationHash` | investor private data | Proves investor is accredited without PII |

All proofs are anchored on‑chain via the governance program’s `verify_proof` instruction. The `ComplianceAttestation` PDA stores the proof hash, circuit ID, and verification key root.

## Guest DAO / Forking Model

SolFunMeme DAO software is structured so other communities can:
1. Fork the Anchor program, providing their own mint + governance config.
2. Mint a “Guest DAO” token whose Transfer Hook points at the canonical membership verifier.
3. Publish a `fork_manifest.json` containing: program IDs, mint addresses, governance params, and the canonical `VKRoot` (verification‑key Merkle root).
4. Cross‑register Guest DAOs via on‑chain attestation from the parent DAO’s multisig, enabling:
   - Shared liquidity pools
   - Coordinated treasury reserves
   - Governance interoperability via `DelegateAccount`

### Meme-Hosting Pattern

A “meme” in this context is a self‑executing governance scope:
- Example: `PepeGovernancePlugin`, `DogeTreasurySplit`
- Each plugin is an Anchor instruction handler invoked by the parent DAO’s `execute_proposal`.
- Plugins are sandboxed: they may only transfer from their own sub‑PDA treasury account.
- Plugin bytecode is itself formally verified (Lean4 → Anchor target) before deployment.

## NJ Law Compliance Map (prereq)

The `njlaw` CLI’s `prereq` command cross‑references DAO bylaws against existing NJ statutory corpus (`~/archive/njleg/downloads/Statutes/STATUTES-TEXT.zip` + `BillTracking/`).

Key precedents to satisfy:
- **RULLCA § 18** — limited‑liability association, entity continuity
- **RULLCA § 22** — operating agreement, membership admission/withdrawal
- **T.49:3‑47 et seq.** — securities registration / exemption requirements
- **A2371 / S1756** — Digital Asset and Blockchain Technology Act (licensing, compliance policies)
- **A3886** — Blockchain Promotion and Integration Program (state‑level crypto‑friendly reporting)
- **S4163** — DAO‑specific LLC recognition (smart‑contract governance, registered‑agent provisions)

**Skill ABI format (canonical minimal set)**

A Skill published by the DAO must specify:

```yaml
skill: <name>
version: <semver>
abi_version: 1
inputs:
  - name: <param>
    type: <Lean4 type | Rust type | protobuf wire type>
    format: <hex | base64 | utf-8 | cid>
    validation: <regex | range | schema>
preconditions:
  - <Lean4 Prop | Rust predicate | ZK circuit public input>
postconditions:
  - <Lean4 Prop | Rust predicate | ZK circuit public output>
side_effects:
  - <on-chain CPI | ledger write | external call>
security:
  auth: <multisig | token-gated | pubkey-allowlist>
  replay_protection: <nonce | timelock | proposal-id>
  zk_attestation: <circuit-id | none>
```

**ZK-Skill attestation flow**

1. Invoker assembles inputs satisfying the Skill ABI.
2. Invoker runs the ZK circuit (`circuit-id` from ABI) producing a proof.
3. DAO verifier (on-chain program or off-chain agent service) checks the proof against the ABI’s verification key root.
4. If valid, the Skill executes; the result is wrapped in a **ZK-Skill receipt** proving postconditions.
5. Receipt is anchored on-chain or in an inter-agent ledger (IPLD DAG-CBOR record signed by the DAO’s agent key).

**Agent identity model**

- Preferred: Solana pubkey ( Ed25519 ) or content-addressed CID (DAG-CBOR).
- Agents carry a `SkillCapability` credential listing the Skill IDs they are authorized to invoke.
- Credentials may themselves be ZK-attested (e.g., "agent X is authorized for skill Y" without revealing X or Y to third parties).

## Shmem Cross-References

> Generated: 2026-06-23 10:20:04 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| CID | computeCID | def |
| DAO | meme_emoji_dao_rust | theorem |
| DAO | meme_emoji_dao_nix | theorem |
| DAO | meme_dao_combinator | theorem |
| Map | uniformInducing_toContinuousMap | theorem |
| Map | toContinuousMap | def |
| Map | map_t₀ | theorem |
| Proof | meme_fractran_proof | theorem |
| Proof | meme_direct_proof | theorem |
| Proof | meme_fractran_proof_nix | theorem |
| Reference | self_reference_transport_preserves_mod_71_eq_0 | theorem |
| ZKP | computeZkpCommitment | def |
| ZKP | zkperf_witness_verifies | theorem |