---
name: nix-flakes
description: >-
  Creates reproducible builds, manages flake inputs, defines devShells,
  and builds packages with flake.nix. Includes Rust build patterns
  (crane recommended, cargoVendorDir legacy), Python packaging, and
  system-manager systemConfigs. All inputs use git+file:/// per agent/foundation.md.
---

# Nix Flakes — Project Management

Modern Nix project management with hermeticity through `flake.lock`.
Every dependency locked to a specific revision for reproducibility.
All inputs follow the n0x-pi git store invariant.

---

## When to Use

- "Initialize a new Nix project"
- "Update flake inputs"
- "Build a flake package"
- "Set up a devShell"
- "Define system-manager service configs"

## Project Setup

```bash
nix flake init
nix flake new hello -t templates#hello
```

Manage dependencies:

```bash
nix flake update               # Update all inputs
nix flake update nixpkgs        # Update specific input
nix flake lock                  # Lock missing entries without updating
```

## Building & Running

```bash
nix build .                     # Build default package (committed)
nix build .#packageName         # Build specific output
nix run .                       # Run default app
nix run .#appName               # Run specific app
```

For remote flakes (from git store):

```bash
nix build git+file:///mnt/data1/git/github.com/<owner>/<repo>.git
```

For system-manager configs:

```bash
nix build .#systemConfigs.all-services --no-link
sudo $(nix build .#systemConfigs.all-services --print-out-paths)/bin/activate
```

With network access (e.g. nora registry):

```bash
nix build .#systemConfigs.all-services --no-link --impure
```

---

## Rust Package Patterns

### Pattern A: Crane (recommended — no vendor in git)

Crane's `vendorCargoDeps` downloads crates from `Cargo.lock` automatically.
Zero vendor commits, zero `.cargo/config.toml`:

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
    packages.${system}.default = craneLib.buildPackage {
      inherit src;
      pname = "my-crate";
      version = "0.1.0";
      cargoExtraArgs = "--bin service";
      doCheck = false;
    };
    devShells.${system}.default = pkgs.mkShell {
      inputsFrom = [ self.packages.${system}.default ];
    };
  };
```

**Why crane:** No vendor commits, no .gitignore anchor issues, `cleanCargoSource`
handles filtering, `overrideToolchain` gives edition 2024 support, two-phase build
caches deps. See the `crane` skill for migration checklist from cargoVendorDir.

### Pattern B: Vendored deps (legacy — vendor/ in git)

Only use when crane is unavailable:

```nix
pkgs.rustPlatform.buildRustPackage {
  pname = "my-crate";
  version = "0.1.0";
  src = ./.;
  cargoVendorDir = "vendor";
  cargoBuildFlags = [ "--bin service" ];
  doCheck = false;
  auditable = false;
}
```

### Pattern C: CargoLock with nora registry (needs --impure)

```nix
# See crane skill — crane handles this via vendorCargoDeps
```

---

## Python Package Pattern

Simple script packaging (no build system needed):

```nix
pkgs.stdenvNoCC.mkDerivation {
  pname = "my-script";
  version = "0.1.0";
  src = ./script.py;
  dontUnpack = true;           # File is a script, not archive
  buildInputs = [ pkgs.python3 ];
  installPhase = ''
    mkdir -p $out/bin
    cp $src $out/bin/${pname}
    chmod +x $out/bin/${pname}
    # Patch shebang if missing
    if ! head -1 $out/bin/${pname} | grep -q '^#!'; then
      sed -i '1i#!/usr/bin/env python3' $out/bin/${pname}
    fi
  '';
}
```

---

## System-Manager Config Pattern

Define deployable systemd services in a flake:

```nix
{
  inputs = {
    nixpkgs.url = "git+file:///mnt/data1/git/github.com/NixOS/nixpkgs.git?ref=master";
    system-manager = {
      url = "git+file:///mnt/data1/git/github.com/numtide/system-manager.git";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, system-manager }:
    let system = "x86_64-linux";
    in {
      # Packages
      packages.${system}.default = pkgs.hello;

      # System-manager configs
      systemConfigs.all-services = system-manager.lib.makeSystemConfig {
        modules = [
          ./system-manager-config.nix
          { nixpkgs.hostPlatform = system; }
        ];
        specialArgs = {
          inherit self;
          # Pass nix-built packages as extra args
          myService = self.packages.${system}.myService;
        };
      };
    };
}
```

---

## Input Patterns

### Bare mirror (git+file://)

```nix
inputs = {
  nixpkgs.url = "git+file:///mnt/data1/git/github.com/NixOS/nixpkgs.git?ref=master";
  my-repo = {
    url = "git+file:///home/mdupont/git/github.com/meta-introspector/my-repo.git?ref=main";
    flake = false;  # non-flake repo
  };
};
```

### Following nixpkgs (share a single nixpkgs revision)

```nix
my-input = {
  url = "git+file:///...";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

---

## Inspecting Flakes

```bash
nix flake show .                 # List all outputs
nix flake metadata .             # See inputs and revisions
nix eval .#packages.x86_64-linux.default.name
```

---

## Development Environments

```bash
nix develop . --command make build
nix develop . --command env       # Check the environment
```

The `--command` flag is required in headless environments.

---

## Best Practices

- Always commit `flake.lock` for reproducibility
- **Use crane** for Rust builds — no vendor in git, `cleanCargoSource`, edition 2024 support
- Use `omaster` refs for upstream ecosystem (crane, rust-overlay, nixpkgs)
- All inputs use `git+file:///mnt/data1/git/` — never `github:`
- Hardcode `system = "x86_64-linux"` — no `eachDefaultSystem`
- Mirror repos to bare git before referencing in flakes
- Use `--impure` only when services need local network (nora, etc.)
- Avoid `environment.systemPackages` in system-manager configs
- DevShells inherit from packages: `inputsFrom = [ self.packages... ]`

## Guardrails

- Never use `github:` URLs — always `git+file:///`
- Never use `flake-utils.lib.eachDefaultSystem` — system lock
- `nix build` must succeed before committing
- No `vendor/` in git — crane handles dependency fetching
- Edition 2024: `Write::flush()` returns `()`, `catch_unwind` needs intermediate vars
- Add `license` to `Cargo.toml` when using `built` build script

## Related

- `nix` — full Nix ecosystem guide
- `nix-build` — flake verification, diagnostics, vendoring
- `system-manager` — service patterns, mkService helpers
- `dasl-testing` — harness build patterns
- `agent/foundation.md` — the full vendoring philosophy

## Shmem Cross-References

> Generated: 2026-06-23 10:20:01 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Nix | meme_bach_bwv_nix | theorem |
| Nix | meme_emoji_dao_nix | theorem |
| Nix | meme_fractran_proof_nix | theorem |
| Rust | meme_emoji_dao_rust | theorem |