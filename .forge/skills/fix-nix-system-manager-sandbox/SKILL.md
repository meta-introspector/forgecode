---
name: fix-nix-system-manager-sandbox
description: >-
  Fix Nix system-manager sandbox permissions blocking dasl-tile service builds.
  Currently fails with "Permission denied (os error 13)" in preBuild sandbox.
  Use when deploying tile servers via system-manager, fixing Nix sandbox issues,
  or matching the pastebin flake.nix pattern.
---

# Fix Nix System-Manager Sandbox for Tile Server Deployment

**Priority:** HIGH
**Area:** Nix
**Status:** Pending
**Source:** status.org

## Problem

Nix system-manager builds fail with "Permission denied (os error 13)" during
the preBuild sandbox phase. The sandbox restricts filesystem access, preventing
cargo from writing to the build directory.

## Current Workaround

Prebuilt binaries from `harnesses/*/target/release/service` are copied into
the Nix store instead of building from source. This works but is not reproducible.

## Solution Pattern

Match the pastebin flake.nix pattern which successfully builds in the sandbox:

1. Use crane + `vendorCargoDeps` instead of `cargoLock.lockFile` or `cargoVendorDir`
2. Vendor all dependencies: `cargo vendor vendor`
3. Create `.cargo/config.toml` with vendored source mapping
4. Fix source filter to exclude only top-level `/target`

## Tile Servers (8 total)

| Server                  | Port  | Binary Status |
|-------------------------|-------|---------------|
| serde_ipld_dagcbor      | 18007 | Prebuilt OK   |
| n0_dasl                 | 18009 | Prebuilt OK   |
| libipld                 | 18011 | Prebuilt OK   |
| dag-cbrrr               | 18001 | Not built     |
| go-ipld-cbor            | 18010 | Not built     |
| js-dag-cbor             | 18002 | Not built     |
| c-cbor (7 binaries)     | 18020 | Not built     |
| gateway                 | 18100 | Not built     |

## Steps

```bash
# 1. Apply pastebin pattern to dasl-tiles flake.nix
cd ~/dasl/dasl-testing
# Edit flake.nix to use crane + omaster refs

# 2. Vendor dependencies
cargo vendor vendor

# 3. Build with system-manager
nix run github:numtide/system-manager -- switch --flake .#dasl-tiles

# 4. Verify services
systemctl status dasl-tile-*
```

## Depends On

Nothing — this is a leaf task.

## Blocks

Nothing directly, but unblocks all 8 tile server deployments.

## Shmem Cross-References

> Generated: 2026-06-23 11:12:46 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Apply | dist_apply_le_dist | theorem |
| Apply | isBigO_norm_rpow_add_one_of_fderiv_of_apply_eq_zero | theorem |
| Apply | End.algebraMap_isUnit_inv_apply_eq_iff | theorem |
| Build | buildPrefixTree | def |
| Fix | _root_.Set.conj_mem_fixingSubgroup | theorem |
| Fix | exists_fixed | theorem |
| Fix | writePrefixTree | def |
| Nix | meme_bach_bwv_nix | theorem |
| Nix | meme_emoji_dao_nix | theorem |
| Nix | meme_fractran_proof_nix | theorem |
| Solution | exists_solution | theorem |
| Verify | meme_byte_count_verify | theorem |