---
name: erdfa-publish-tile

description: >-
  eRDFa Publish: Semantic UI as CBOR shards with Nora registry publishing
  
  Complete workflow for publishing the erdfa-publish Rust crate to a local
  or remote Nora artifact registry instance. Includes:
  
  - Package configuration for Nora publishing
  - Publishing workflow from erdfa-publish-rs
  - Integration with nora-monitor-tile for registry health checks
  
  Use when publishing or consuming erdfa-publish packages through Nora.
---

# eRDFa Publish Tile

## Overview

This skill provides patterns for publishing the **eRDFa Publish** Rust crate to the [Nora](https://github.com/getnora-io/nora) artifact registry, including:

- **eRDFa Publish** (`github.com/meta-introspector/erdfa-publish`): Generate DA51 CBOR shards from typed Rust components
- **Nora Registry**: Publish and proxy Cargo packages through `sparse+http://127.0.0.1:4000/cargo/index/`

### Key Features

| Component | Description
|-----------|-------------|
| **Semantic Components** | Heading, Paragraph, Table, Tree, Link, Image, Code, KeyValue, MapEntity, Group |
| **Conformal Field Tower (CFT)** | Multi-scale text decomposition with n-grams at each layer |
| **CBOR Format** | DA51 tag (55889 / 0xDA51) for content-addressed shards |
| **Nora Publishing** | Publish via `cargo publish --registry nora` |

### Current Version

```
erdfa-publish@0.1.0 → Published to: http://127.0.0.1:4000/cargo/index/
```

## Publishing Workflow

### Step 1: Verify Nora Instance

```bash
# Health check (should show {"status":"healthy"})
curl http://127.0.0.1:4000/health

# Check Cargo registry availability
curl http://127.0.0.1:4000/cargo/
```

### Step 2: Package for Publishing (No Re-compilation Required)

```bash
cd ~/projects/erdfa-publish

# Create package tarball without compiling dependencies
# (Uses nora index via global .cargo/config.toml)
cargo package

# Package file created in target/package/
ls -la target/package/erdfa-publish-0.1.0.crate
```

### Step 3: Publish to Nora Registry

```bash
# Publish using the already-packaged crate
cargo publish --registry nora

# Expected output:
#  Updating `nora` index  
#  Note: Packaged erdfa-publish v0.1.0 ✅
```

### Step 4: Verify Publication

```bash
# Check package appears in nora index
curl http://127.0.0.1:4000/cargo/index/config.json \
  | jq '.packages[] | select(.name == "erdfa-publish")'

# Search local registry
cargo search erdfa-publish --registry nora
```

## Configuration Files

### Global Cargo Config (~/.cargo/config.toml)

```toml
[source.crates-io]
replace-with = "nora"

[source.nora]
registry = "http://127.0.0.1:4000/cargo/index"

[registries.nora]
index = "sparse+http://127.0.0.1:4000/cargo/index/"
```

### Project Cargo.toml Configuration

```toml
[package]
name = "erdfa-publish"
version = "0.1.0"
edition = "2021"
description = "Semantic UI components as CBOR shards with Conformal Field Tower text decomposition"
license = "MIT OR Apache-2.0"

# Dependencies configured for Nora publishing
[dependencies.rust-unixfs]
version = "0.5.0"  # Fetched through Nora registry
optional = true

[dependencies.ipld-core]
version = "0.4.1"  # Fetched through Nora registry
optional = true
```

## Component Types

| Type | Fields | Semantic Meaning |
|------|--------|-----------------|
| `Heading` | `level`, `text` | Section header (1–6) |
| `Paragraph` | `text` | Block of prose |
| `Code` | `language`, `source` | Source code with syntax hint |
| `Table` | `headers`, `rows` | Tabular data |
| `Tree` | `label`, `children` | Recursive hierarchy |
| `List` | `ordered`, `items` | Ordered or unordered list |
| `Link` | `href`, `label` | Navigation reference |
| `Image` | `alt`, `cid` | Image by content address |
| `KeyValue` | `pairs` | Metadata / properties |
| `MapEntity` | `name`, `kind`, `x`, `y`, `meta` | Positioned entity on a map |
| `Group` | `role`, `children` | Container with semantic role |

## CBOR Format

All shards and manifests use **DA51** (0xDA51 = 55889) CBOR tag:

```rust
// Simplified format
{
  "id": "shard-id",
  "cid": "bafk2bz...",  // Content-addressed identifier
  "component": {
    "type": "Heading",
    "level": 1,
    "text": "Results"
  },
  "tags": ["cft", "example"]
}
```

### Scale Layers (Conformal Field Tower)

| Scale | Depth | Splits on | N-grams |
|-------|-------|-----------|---------|
| Post | 0 | — | bigramsets of all tokens |
| Paragraph | 1 | `\n\n` | bigramsets, trigramsets |
| Line | 2 | `\n` | bigramsets, trigramsets |
| Token | 3 | whitespace | — |
| Emoji | 4 | unicode ranges | — |
| Byte | 5 | — | — |

### Arrow Shards (Typed Edges)

Parent→child relationships are themselves shards:

```rust
{
  "id": "my-doc_post→my-doc_p0",
  "component": {
    "type": "KeyValue",
    "pairs": [
      ["from", "my-doc_post"],
      ["to", "my-doc_p0"],
      ["scale_from", "0"],
      ["scale_to", "1"],
      ["morphism", "cft.post→cft.paragraph"]
    ]
  },
  "tags": ["cft", "arrow"]
}
```

## Integration with Nora Skills

### Uses: nora-monitor-tile

The erdfa-publish package integrates with the nora-monitor-tile skill for checking registry health:

```bash
# Health check via script
/skill:nora-monitor-tile check

# Expected: Shows Nora service status at port 4000
```

### Uses: nora-car-shmem

If configured to use CAR shared memory backend:

```toml
# In Nora's config.toml:
[storage]
backend = "car-shmem"
```

### Uses: crane

The erdfa-publish flake.nix uses crane patterns:

```nix
craneLib.buildPackage {
  pname = "erdfa-publish";
  version = "0.1.0";
  cargoLock.lockFile = ./Cargo.lock;
  cargoLock.allowBuiltinFetchGit = true;
  buildFeatures = [ "native" "cli" "ipfs" ];
  buildNoDefaultFeatures = true;
}
```

## Flake Configuration (for building)

```nix
# In ~/projects/erdfa-publish/flake.nix:
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-ipfs.url = "github:meta-introspector/rust-ipfs";
  };
  
  outputs = { self, nixpkgs, flake-utils, rust-ipfs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.erdfa-publish = pkgs.rustPlatform.buildRustPackage {
          pname = "erdfa-publish";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
          buildFeatures = [ "native" "cli" "ipfs" ];
          buildNoDefaultFeatures = true;
        };
      }
    );
}
```

## Quick Commands Reference

```bash
# Check Nora health
curl http://127.0.0.1:4000/health

# Package erdfa-publish
cd ~/projects/erdfa-publish
cargo package

# Publish to Nora
cargo publish --registry nora

# Verify publication
curl http://127.0.0.1:4000/cargo/index/config.json \
  | jq '.packages[] | select(.name == "erdfa-publish")'

# Update nora-src in nora flake
cd ~/projects/nora
nix flake lock --update-input nora-src
bash deploy.sh deploy
```

## Publishing Cycle

```
erdfa-publish-rs                    Nora (@127.0.0.1:4000)
  ├─ release build ────────────── cargo package
  │    (fetches deps from nora)  
  ├─ cargo package ────────────── Creates .crate file
  ├─ cargo publish --registry nora ── Uploads to nora registry
  └─ Package now available ────── Accessible via:
       cargo install erdfa-publish
       # or
       cargo add erdfa-publish --registry nora
```

## Architecture Blueprint

See `~/projects/dotagents/SYSTEM_BLUEPRINT.md` for full integration details.

```
┌─────────────────────────────────────────────────────────────┐
│                     Nora Registry (@:4000)                   │
├─────────────────────────────────────────────────────────────┤
│  • Cargo protocol         /cargo/index/                      │
│  • Health monitoring      /health                          │
│  • Storage backend        (local or car-shmem)              │
│  • Publish endpoint       POST /cargo/                      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                 eRDFa Publish (@0.1.0)                     │
├─────────────────────────────────────────────────────────────┤
│  • Semantic Components     (Heading, Table, Tree, ...)      │
│  • Conformal Field Tower   (CFT decomposition)              │
│  • CBOR/DAG-CBOR           (DA51 tag 55889=0xDA51)           │
│  • Dependencies            (rust-unixfs via nora @0.5.0)     │
│  • Nix Build               (crane + rust-overlay)           │
│  • Nora Publishing         (cargo publish --registry nora)   │
└─────────────────────────────────────────────────────────────┘
```

## TODOs & Future Work

- [ ] Document publishing to remote Nora instances (URL-based config)
- [ ] Add integration tests for package consumption
- [ ] Create example projects that depend on erdfa-publish via nora
- [ ] Add shard-preview tool for visualizing CBOR shards
- [ ] Document CFT decomposition strategies

## Related Skills

- `~/projects/dotagent/skills/crane/` - Nix Rust build patterns
- `~/projects/dotagent/skills/nora-monitor-tile/` - Registry health monitoring  
- `~/projects/dotagent/skills/nora-car-shmem/` - CAR shared memory storage
- `~/projects/dotagent/skills/dag-cbor-restriction-morphisms/` - CBOR validation

## Cross-references

- [**erdfa-publish**](https://github.com/meta-introspector/erdfa-publish) - Main repo
- [**nora**](https://github.com/getnora-io/nora) - Artifact registry
- [**system-blueprint**](https://github.com/meta-introspector/rust-ipfs) - Architecture docs

## Shmem Cross-References

> Generated: 2026-06-23 11:12:45 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Code | meme_encoded_message | theorem |
| Code | encodeShard | def |
| Group | _root_.Set.conj_mem_fixingSubgroup | theorem |
| Group | groupByShard | def |
| Image | Finset.mulSupport_of_fiberwise_prod_subset_image | theorem |
| Image | LinearIndepOn.span_image_extend_eq_span_image | theorem |
| Image | ContDiffOn.dense_compl_image_of_dimH_lt_finrank | theorem |
| List | meme_CMakeLists | theorem |
| Reference | self_reference_transport_preserves_mod_71_eq_0 | theorem |
| Shards | processShards | def |
| Shards | generateShardSummary | def |
| Shards | splitWitnessIntoShards | def |
| Step | fractran_step | def |
| Tree | writePrefixTree | def |
| Tree | buildPrefixTree | def |
| Update | det_smul_mk_coord_eq_det_update | theorem |
| Verify | meme_byte_count_verify | theorem |