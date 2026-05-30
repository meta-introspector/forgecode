# cargo-vendormod — Vendor Cargo Dependencies from Source Mirrors

A Cargo subcommand (`cargo vendormod`) that vendors Rust crate dependencies from local git mirrors instead of downloading from crates.io. Works with `cargo2nix` for reproducible Nix builds.

## Location

- **Source**: `~/projects/cargo-clean/tools/cargo-vendormod/`
- **Mirror**: `~/git/solana.solfunmeme.com/cargo-vendormod`
- **Branch**: `organize-submodules`
- **Binary**: `cargo-vendormod`

## Flake Integration

The package is available in forgecode's flake:

| Output | Command |
|--------|---------|
| Package | `nix build .#cargo-vendormod` |
| App | `nix run .#cargo-vendormod` |
| DevShell | On `PATH` in `nix develop` |

The flake input uses the git mirror at:
```
url = "git+file:///home/mdupont/git/solana.solfunmeme.com/cargo-vendormod?ref=organize-submodules";
```

## Submodules

- `global_graph` — registered as a proper submodule, pointing to `~/git/solana.solfunmeme.com/global_graph`

## Usage

```bash
# Run directly
nix run .#cargo-vendormod -- --help

# In dev shell
nix develop
cargo vendormod --help

# Vendor deps for a project
cargo vendormod /path/to/project
```

## Dependencies

- Rust (cargo + rustc)
- Local git mirrors at `~/git/` for vendoring
