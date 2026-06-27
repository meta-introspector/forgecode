---
name: crane
description: >-
  Crane — Nix library for building Rust crates. Covers buildPackage, vendorCargoDeps,
  cargoVendorDir patterns, omaster refs for local mirrors, and the no-circular-bootstrap
  rule (nora must not use nora to build nora). Use when building Rust projects with nix
  or debugging crane build failures.
---

# Crane — Rust in Nix

Crane is the standard Nix library for building Rust crates via
`vendorCargoDeps` + `buildPackage`. Provides vendoring, dependency
caching, and workspace support.

---

## Quick Patterns

### Two-phase build (deps cached)

```nix
let cargoArtifacts = craneLib.buildDepsOnly { inherit src; };
in craneLib.buildPackage { inherit src cargoArtifacts; }
```

### Vendor deps via crane (default, no git vendor dir)

```nix
craneLib.buildPackage {
  inherit src;
  # cargoVendorDir is automatically set to vendorCargoDeps result
}
```

Crane's `vendorCargoDeps` creates a nix derivation that downloads all crates
from your `Cargo.lock` — no `cargo vendor vendor/` needed, nothing to commit.

---

## DASL Pattern: omaster Refs

All flake inputs use `omaster`/`omain` refs pointing to original upstream:

```nix
inputs = {
  nixpkgs.url = "git+file:///mnt/data1/git/github.com/NixOS/nixpkgs.git?ref=omaster";
  crane.url = "git+file:///mnt/data1/git/github.com/ipetkov/crane.git?ref=omaster";
  rust-overlay.url = "git+file:///mnt/data1/git/github.com/oxalica/rust-overlay.git?ref=omaster";
};

outputs = { self, nixpkgs, crane, rust-overlay, ... }:
  let
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ rust-overlay.overlays.default ];
    };
    craneLib = (crane.mkLib pkgs).overrideToolchain pkgs.rust-bin.stable.latest.default;
  in {
    packages.${system}.default = craneLib.buildPackage {
      src = craneLib.cleanCargoSource ./.;
      doCheck = false;
    };
  };
```

---

## No Circular Bootstrap Rule

**nora must not use nora to build nora.** If the package you're building
IS the registry, remove any `.cargo/config.toml` that points to nora
before the crane build. Crane's `vendorCargoDeps` will download directly
from crates.io.

Wrong (circular):
```nix
# Don't inject nora config into nora's own build!
src = pkgs.runCommand "src-with-nora" {} ''
  cp -r ${nora-src} $out
  echo '[source.crates-io]' >> $out/.cargo/config.toml
  echo 'replace-with = "nora"' >> $out/.cargo/config.toml
'';
```

Right:
```nix
# Let crane vendor from crates.io directly
craneLib.buildPackage {
  src = nora-src;  # No .cargo/config.toml injection
}
```

---

## Working Flake Example (~/projects/nora/)

See `~/projects/nora/flake.nix` for a complete working example with:
- `omaster` refs on all inputs
- crane + rust-overlay
- System-manager deployment config
- 4-step `deploy.sh` (lock update → build → systemConfig → activate)

---

## Deploy Pattern

```bash
cd ~/projects/nora
bash deploy.sh deploy
# 1. nix flake lock --update-input nora-src
# 2. nix build .#
# 3. nix build .#systemConfigs.nora
# 4. sudo <result>/bin/activate
```

---

## Migration: cargoVendorDir → Crane

Checklist for converting existing `buildRustPackage` flakes to crane:

1. **Add crane + rust-overlay inputs** (omaster refs)
2. **Remove `cargoVendorDir = "vendor"`** — crane handles deps via Cargo.lock
3. **Remove `.cargo/config.toml`** — crane handles registry
4. **Remove `vendor/` from git** — `git rm -r --cached vendor/ && rm -rf vendor/`
5. **Replace `buildRustPackage` with `craneLib.buildPackage`**
6. **Use `craneLib.cleanCargoSource ./.;`** instead of manual source filters
7. **Use `cargoExtraArgs`** instead of `cargoBuildFlags`
8. **No `auditable = false` needed** — crane doesn't use cargo-auditable
9. **Drop `flake-utils`** — hardcode `system = "x86_64-linux"`

### Before (cargoVendorDir)

```nix
inputs = {
  nixpkgs.url = "git+file:///.../nixpkgs.git?ref=master";
  flake-utils.url = "git+file:///.../flake-utils.git";
};
outputs = { self, nixpkgs, flake-utils }:
  flake-utils.lib.eachDefaultSystem (system:
    let pkgs = nixpkgs.legacyPackages.${system};
    in {
      packages.default = pkgs.rustPlatform.buildRustPackage {
        src = ./.;
        cargoVendorDir = "vendor";
        cargoBuildFlags = [ "--bin service" ];
        doCheck = false;
        auditable = false;
      };
    });
```

### After (crane)

```nix
inputs = {
  nixpkgs.url = "git+file:///.../nixpkgs.git?ref=omaster";
  crane.url = "git+file:///.../crane.git?ref=omaster";
  rust-overlay.url = "git+file:///.../rust-overlay.git?ref=omaster";
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
    packages.${system}.default = craneLib.buildPackage {
      inherit src;
      pname = "my-crate";
      version = "0.1.0";
      cargoExtraArgs = "--bin service";
      doCheck = false;
    };
  };
```

---

## Edition 2024 Caveats

When building edition 2024 crates with crane:

- **`Write::flush()` returns `()`** not `io::Result<()>` — use `.map_err(|e| Box::new(e) as Box<dyn Error>)?`
- **`catch_unwind` type inference breaks** with nested `Result` patterns — assign to intermediate variable before match
- **Duplicate imports are hard errors** — `BufRead`, `BufWriter`, `Instant` etc. can only appear once
- **`license` field required** if using the `built` crate build script (`CARGO_PKG_LICENSE`)
- **`overrideToolchain` gives fresh rustc** — no `auditable` hacks needed, edition 2024 works out of the box

---

## Verification Pattern

After building, verify the binary serves correctly:

```bash
# Build
nix build --no-link --print-out-paths
BIN=$(nix build --print-out-paths)/bin/service

# Test health endpoint
$BIN 19999 &
sleep 1
curl -s http://127.0.0.1:19999/health
kill %1
```

For system-manager deployment, verify with `systemctl`:

```bash
sudo systemctl restart my-service
systemctl is-active my-service
curl http://127.0.0.1:<port>/health
```

---

## Related

- `nix-flakes` — flake structure, crane migration patterns
- `system-manager` — service deployment patterns
- `nix-build` — flake verification and diagnostics
- `nora-monitor-tile` — nora health monitoring

## Shmem Cross-References

> Generated: 2026-06-23 11:12:36 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Build | buildPrefixTree | def |
| DASL | writeDaslFile | def |
| DASL | meme_DASL2_LITERATE | theorem |
| DASL | DaslItem | structure |
| Let | orthogonal_span_singleton_eq_to_lin_ker | theorem |
| Let | meme_MONSTER_WALK_INDEX_COMPLETE | theorem |
| Let | [CompleteSpace | instance |
| Nix | meme_bach_bwv_nix | theorem |
| Nix | meme_emoji_dao_nix | theorem |
| Nix | meme_fractran_proof_nix | theorem |
| Rust | meme_emoji_dao_rust | theorem |
| Test | test_lemma | lemma |
| Test | test_ingest | theorem |
| Test | test_pow | theorem |