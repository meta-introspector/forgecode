---
name: decl-splitter-lattice
description: Split Rust code into individual declarations, analyze dependencies, generate FFI plugin crate lattice with external-deps-aware layering, and build compilable workspace. Use when you need to decompose monolithic Rust crates into topologically-sorted plugin crates with minimal dependencies.
---

# Decl Splitter & Lattice Generator

This skill provides a complete pipeline for splitting Rust code into individual declaration files and generating a dependency-aware lattice of FFI plugin crates.

## Quick Start

```bash
# 1. Split a crate's source files into individual decls
.forge/skills/decl-splitter-lattice/scripts/split_crate.sh crates/forge_domain

# 2. Generate plugin crates (optionally with excludes)
.forge/skills/decl-splitter-lattice/scripts/generate_lattice.sh \
  crates/forge_domain /tmp/forge_plugins

# 3. Build the resulting workspace (auto-generates workspace Cargo.toml)
.forge/skills/decl-splitter-lattice/scripts/build_workspace.sh /tmp/forge_plugins
done
```

## CAR Archival of Excluded Decls

Excluded decls are not lost — they are archived as **DAG-CBOR vernacular documents** in a CAR (Content-Addressable Archive) file.

### Overview

Each excluded decl produces a `SynVernacular` document:
- Semantic metadata (kind, defines, references, external_uses)
- Impl-specific metadata (self_type, trait)
- Full raw source code
- Human-readable error hint (why excluded)
- CID (content-addressed, computed from CBOR bytes)
- Timestamp

### Format

```
excluded_decls.car
├── Header: {"version": 1, "roots": []}
├── Block 1: CID(sha2-256) → CBOR(SynVernacular for forge_domain_Cause)
├── Block 2: CID(sha2-256) → CBOR(SynVernacular for forge_domain_ChatResponse)
└── Block 3: CID(sha2-256) → CBOR(SynVernacular for forge_domain_impl_for_)
```

### CID Computation

```
CID v1:
  version:  0x01
  codec:    0x71 (dag-cbor)
  hash:     0x12 0x20 sha2-256(32 bytes)
  data:     SHA-256(CBOR document)
```

### Usage

```bash
# Generate with CAR archive
# For the excluded decls we can see the CID pointers and re-derive them when needed.

cargo run --package decl-splitter --bin decl-lattice -- \
  generate --crate-dir crates/forge_domain \
  --output /tmp/forge_plugins \
  --exclude-file excludes.txt \
  --car-output /tmp/forge_plugins/excluded_decls.car
```

### CAR File Structure (Raw)

```
<LEB128(header_len)> <CBOR(header)>
<LEB128(section_len)> <CID_bytes(36)> <CBOR(SynVernacular)>
<LEB128(section_len)> <CID_bytes(36)> <CBOR(SynVernacular)>
...
```

### SynVernacular Document Schema

```rust
struct SynVernacular {
    decl_id: String,        // e.g. "forge_domain_Cause"
    kind: String,           // "struct" | "enum" | "impl" | "fn" | "trait" | "type"
    defines: Vec<String>,   // type names defined by this decl
    impl_self_type: Option<String>,  // for impl blocks
    impl_trait: Option<String>,      // trait being implemented
    references: Vec<String>,          // type identifiers referenced
    external_uses: Vec<String>,       // external crate names
    source: String,                   // full raw source code
    source_file: String,              // original file path
    error_hint: String,               // why excluded
    cid: String,                      // self-referential CID
    timestamp: String,                // ISO 8601 timestamp
}
```

### Benefits

- **No data loss**: Excluded decls preserve their full AST context
- **Content-addressed**: Each document has a unique CID derived from its CBOR content
- **Self-describing**: The CAR file format is a standard IPLD transport format
- **Inspectable**: Use any CAR reader to list and extract vernacular documents
- **Replayable**: Documents can be re-imported when compilers catch up

## Troubleshooting
### 1. Declaration Splitting (`scripts/split_crate.sh`)

Converts monolithic Rust files into individual declaration files (one per struct/enum/impl/fn/trait/type):

```bash
.forge/skills/decl-splitter-lattice/scripts/split_crate.sh crates/forge_domain
```

**What it does:**
- Finds all `.rs` files (excluding `mod.rs`, `lib.rs`, and existing `decls/` dirs)
- Runs `decl-splitter --input <file> --output <target>` for each file
- Output: `src/decls/<stem>/` directories with individual `.rs` files
- Skips already-split files on re-run

**Underlying tool:**
```bash
cargo run --package decl-splitter --bin decl-splitter -- --input <file.rs> --output <target_dir>
```

### 2. Dependency Analysis

Analyzes split declarations to build dependency graph:

```bash
cargo run --package decl-splitter --bin decl-lattice -- \
  analyze --crate-dir crates/forge_domain --output lattice.json
```

Output: JSON with `decl_info` (nodes), `layers` (topological order), `sccs` (cycles).

### 3. Plugin Crate Generation (`scripts/generate_lattice.sh`)

Generates FFI-boundary plugin crates with correct dependencies:

```bash
.forge/skills/decl-splitter-lattice/scripts/generate_lattice.sh \
  crates/forge_domain /tmp/forge_plugins

# With exclude file:
.forge/skills/decl-splitter-lattice/scripts/generate_lattice.sh \
  crates/forge_domain /tmp/forge_plugins /path/to/excludes.txt
```

**Underlying tool:**
```bash
cargo run --package decl-splitter --bin decl-lattice -- \
  generate --crate-dir crates/forge_domain --output /tmp/forge_plugins \
  [--exclude-file excludes.txt]
```

Features:
- **External-deps-aware layering**: Each layer introduces external crate dependencies incrementally
- **Use rewriting**: `crate::TypeName` → `dep_crate::TypeName`
- **Chunking**: Layers with >20 decls split into multiple crates (`_c0`, `_c1`, etc.)
- **Exclude support**: Skip problematic decls to achieve clean compilation
- **Cargo.toml generation**: Path deps for internal crates, version deps for external crates

### 4. Workspace Building (`scripts/build_workspace.sh`)

Builds and validates the generated workspace:

```bash
.forge/skills/decl-splitter-lattice/scripts/build_workspace.sh /tmp/forge_plugins
```

**What it does:**
- Auto-generates workspace `Cargo.toml` with all `forge_*` crate members
- Runs `cargo check --workspace` with output logging to `build_output.txt`

## Key Concepts

### Layer Design

Each plugin crate follows naming: `forge_<crate>_l<layer>[_c<chunk>]`

- **Layer number**: Topological depth + external deps layer
- **Chunk suffix**: Appears when layer has >20 decls
- **Dependencies**: Path deps to lower layers, version deps to external crates

### External Dependency Management

External crates are sorted by frequency (most-used first) to minimize cross-layer pollution:

```
Layer 0: No external deps
Layer 1: serde
Layer 2: serde, pretty_assertions
Layer 3: serde, pretty_assertions, chrono
...
```

### Use Rewriting

Automatically transforms:
- `use crate::TypeName` → `use forge_l1_c0::TypeName`
- `use crate::{A, B}` → `use forge_l1_c0::A; use forge_l1_c0::B;`
- `crate::TypeName` in code bodies → `dep_crate::TypeName`
- Falls back to `self::TypeName` for unresolvable `crate::` refs

## Scripts

### scripts/split_crate.sh

Splits all `.rs` files in a crate into individual declaration files.

**Usage:** `split_crate.sh <crate_dir>`

**Behavior:**
- Creates `src/decls/<file_stem>/` directories
- Each directory contains one `.rs` per declaration
- Skips already-split files on re-run
- After completion, reports total decl count

**Example output:**
```
Splitting declarations in: crates/forge_domain
  Splitting: crates/forge_domain/src/model.rs
done.
  Splitting: crates/forge_domain/src/file.rs
done.
...
Done! Created 559 decls in: crates/forge_domain/src/decls/
```

### scripts/generate_lattice.sh

Generates FFI-boundary plugin crates from split declarations.

**Usage:** `generate_lattice.sh <crate_dir> <output_dir> [exclude_file]`

- `<crate_dir>`: Path to crate with `src/decls/` (must have been split first)
- `<output_dir>`: Where generated plugin crates will be written
- `<exclude_file>`: Optional file listing decl IDs to skip (one per line)

### scripts/build_workspace.sh

Builds the generated workspace.

**Usage:** `build_workspace.sh <workspace_dir>`

- Auto-generates workspace `Cargo.toml` if missing
- Runs `cargo check --workspace` with output logged to `build_output.txt`

## Excluding Problematic Decls

When certain decls can't compile as standalone crates, exclude them:

### Known Problem Categories

| Error | Cause | Fix |
|-------|-------|-----|
| E0116 | Cross-crate impl | Exclude the impl decl |
| E0277 | nom round-trip | Exclude nom-related decls |
| E0432 | Unresolved `crate::` ref | Exclude or split inner type |
| E0433 | Missing crate scope | Exclude the scoped decl |
| E0616 | Private field access | Exclude the accessor |

### Exclude File Format

```txt
# Comment lines start with #
forge_domain_AttachmentContent  # nom parser issue
forge_domain_impl_for_Cause      # E0116 cross-crate impl
```

## Output Structure

Generated workspace contains:

```
my_plugins/
├── forge_my_crate_l1/           # Layer 1: first external deps
│   ├── Cargo.toml
│   ├── lib.rs
│   ├── ffi.rs
│   ├── forge_my_crate_Struct.rs
│   └── forge_my_crate_impl_Trait.rs
├── forge_my_crate_l2/           # Layer 2: + more external deps
├── forge_my_crate_l3_c0/        # Layer 3, chunk 0
├── forge_my_crate_l3_c1/        # Layer 3, chunk 1
├── Cargo.toml                   # Workspace config
└── plugin_plan.json             # Generated metadata (lattice, type map)
```

Each plugin crate builds independently once its dependencies are available.

## Troubleshooting

### Common Issues

**"available binaries: decl-lattice, decl-splitter"**  
The `--bin` flag is required. Use `cargo run --package decl-splitter --bin decl-splitter` or `cargo run --package decl-splitter --bin decl-lattice`.

**"No such file or directory" for exclude file**  
The `--exclude-file` argument requires a valid path. Use the optional argument pattern in `generate_lattice.sh` (only passes it if the file exists).

**"cannot move to a subdirectory of itself" during split**  
Don't use `mv` on the `decls/` directory — use `--output` flag on each file instead. The `split_crate.sh` script handles this correctly.

### Iterative Exclusion

For achieving clean compilation:

1. Generate without excludes
2. Run `cargo check` to find failing decls
3. Add failing decl IDs to exclude file
4. Regenerate with `--exclude-file excludes.txt`
5. Repeat until clean build

**Automated iterative approach** (when needed):
```bash
while cargo check --workspace 2>&1 | grep -q "error\["; do
  # Add failing decls to exclude file
  cargo check 2>&1 | grep "error\[E" -A3 | grep "\.rs:" \
    | sed 's/.*--> //;s/:.*//;s|.*/src/||;s/\.rs$//' \
    | sort -u >> excludes.txt
  # Regenerate
  .forge/skills/decl-splitter-lattice/scripts/generate_lattice.sh \
    crates/forge_domain /tmp/forge_plugins excludes.txt
  .forge/skills/decl-splitter-lattice/scripts/build_workspace.sh /tmp/forge_plugins
done
```
