# deep-scanner — Recursive File Scanner for DASL

A Rust CLI tool that recursively scans directories, collecting file metadata (path, size, depth, hash) and producing DAG-CBOR output. Designed for DASL ("dazzle") — a small set of simple, standard primitives for working with content-addressed, linked data.

DASL builds on content addressing (used in Git and IPFS) to create reliable content identifiers (CIDs) through cryptographic hashing. Key principles:
- **Pave the cowpaths**: support what people actually use
- **Extensibility vs optionality**: extension points now, not options
- **Don't make me think**: works out of the box
- **Unix philosophy**: tiny, composable specs

## Location

- **Source**: `~/projects/dasl/IMPL/deep_scanner/`
- **Mirror**: `~/git/solana.solfunmeme.com/deep_scanner`
- **Branch**: `main`
- **Binary**: `deep_scanner`

## Flake Integration

The package is available in forgecode's flake:

| Output | Command |
|--------|---------|
| Package | `nix build .#deep-scanner` |
| App | `nix run .#deep-scanner` |
| DevShell | On `PATH` in `nix develop` |

The flake input uses the git mirror at:
```
url = "git+file:///mnt/data1/git/solana.solfunmeme.com/deep_scanner?ref=main";
```

## Usage

```bash
# Run directly
nix run .#deep-scanner -- /path/to/scan

# In dev shell
nix develop
deep_scanner /path/to/scan
```

## Build from source

```bash
cd ~/projects/dasl/IMPL/deep_scanner
cargo build --release
./target/release/deep_scanner
```

## Related

- Uses `cargo2nix` for Nix builds (inputs from `/tmp/flake-local/`)
- Depends on `cargo-vendormod` at the Cargo level for dynamic file discovery
- Produces DAG-CBOR output via `serde_ipld_dagcbor` + `ipld-core`
- Cargo.lock is tracked in the repo
