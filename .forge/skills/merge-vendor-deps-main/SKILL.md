---
name: merge-vendor-deps-main
description: >-
  Convert ipld-car-ipc-shmem-linux from vendor-deps to crane pattern.
  The main branch is broken (cargoLock.lockFile with file:// paths fails in
  Nix sandbox). Prefer crane + vendorCargoDeps over merging vendor-deps.
  Use when fixing offline Nix builds, converting flakes, or updating flake.nix.
---

# Convert ipld-car-ipc-shmem-linux to Crane

**Priority:** HIGH
**Area:** Nix
**Status:** Pending

## Problem

The `main` branch uses `cargoLock.lockFile` which contains `file://` paths
that fail inside the Nix sandbox. The `vendor-deps` branch fixed this with
`cargo vendor` + `cargoVendorDir = "vendor"`, but that commits 11,584 vendor
files to git.

**Better approach:** Convert to crane instead of merging vendor-deps.
Crane's `vendorCargoDeps` handles deps from Cargo.lock — no vendor commits.

## Options

| Approach | Status | Vendor in git | Edition 2024 |
|----------|--------|:---:|:---:|
| main (broken) | ❌ BROKEN | — | ❌ |
| vendor-deps merge | ⚠️ Works | 11,584 files | ❌ needs auditable=false |
| **Crane conversion** | ✅ **Recommended** | **0 files** | ✅ via overrideToolchain |

## Conversion Steps

Follow the `crane` skill migration checklist:

```bash
cd /mnt/data1/time-2026/02-february/22/dasl/ipld-car-ipc-shmem-linux

# 1. Rewrite flake.nix to crane + omaster refs
#    (see harnesses/serde_ipld_dagcbor/flake.nix for template)

# 2. Remove vendor and .cargo/config.toml
git rm -r vendor/ .cargo/
rm -rf vendor/ .cargo/

# 3. Lock and build
nix flake lock
nix build .#letta-ipld-memory

# 4. Verify
nix build .#letta-ipld-memory --option substitute false
```

## Crane flake template

```nix
inputs = {
  nixpkgs.url = "git+file:///mnt/data1/git/github.com/NixOS/nixpkgs.git?ref=omaster";
  crane.url = "git+file:///mnt/data1/git/github.com/ipetkov/crane.git?ref=omaster";
  rust-overlay.url = "git+file:///mnt/data1/git/github.com/oxalica/rust-overlay.git?ref=omaster";
};
outputs = { self, nixpkgs, crane, rust-overlay, ... }:
  let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ rust-overlay.overlays.default ];
    };
    craneLib = (crane.mkLib pkgs).overrideToolchain
      pkgs.rust-bin.stable.latest.default;
    src = craneLib.cleanCargoSource ./.;
  in {
    packages.${system}.letta-ipld-memory = craneLib.buildPackage {
      inherit src;
      pname = "letta-ipld-memory";
      version = "0.1.0";
      cargoExtraArgs = "-p letta-ipld-memory";
      doCheck = false;
    };
  };
```

## Reference

5 harness tiles already converted and deployed:
- `~/dasl/dasl-testing/harnesses/serde_ipld_dagcbor/flake.nix`
- `~/dasl/dasl-testing/harnesses/n0_dasl/flake.nix`
- `~/dasl/dasl-testing/harnesses/libipld/flake.nix`
- `~/dasl/dasl-testing/harnesses/qa-team-tile/flake.nix`
- `~/dasl/dasl-testing/harnesses/fuzz-team-tile/flake.nix`

See [[crane]] for the full migration guide and [[nix-flakes]] for build patterns.

## Shmem Cross-References

> Generated: 2026-06-23 10:20:00 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Reference | self_reference_transport_preserves_mod_71_eq_0 | theorem |
| Remove | remove_57_over_1_terminates | theorem |
| Remove | remove_41_over_41_terminates_identity | theorem |
| Verify | meme_byte_count_verify | theorem |