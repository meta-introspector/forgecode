# External-Dep-Aware Layering Strategy

## Layer Assignment Rules

1. **Layer 0**: No external dependencies
2. **Layer N**: Introduces external_crate_layers[N-1]
3. **Final layer**: max(topological_depth, external_deps_layer)

## External Crate Ordering

External crates are sorted by frequency (most-used first):

```yaml
crates:
  - serde          # Used by most decls
  - pretty_assertions
  - derive_setters
  - derive_more
  - schemars
  - uuid
  - chrono
  - async_trait
  - tokio
  - anyhow
  - thiserror
  - url
  # etc.
```

## Example Layer Structure

```
Forge Domain (355 decls)
├── l1: serde + derive_setters (12 decls)
├── l2: + pretty_assertions (9 decls)
├── l3: + schemars + uuid (14 decls)
├── l7_c0: + fake + merge (20 decls)
├── l8_c0: + serde_json (20 decls)
├── l10_c0: + chrono + uuid (20 decls)
├── l11_c0: + schemars + fake (20 decls)
├── l12_c0: + async_trait (20 decls)
├── l14: + anyhow + url (12 decls)
├── l15: + tracing (19 decls)
├── l16: + tokio + chrono (16 decls)
└── l17: + convert_case + schemars (11 decls)
```

## Cargo.toml Generation

Each plugin crate's Cargo.toml includes:

```toml
[dependencies]
# Path deps for internal layers
forge_forge_domain_l1 = { path = "../forge_forge_domain_l1" }
forge_forge_domain_l2 = { path = "../forge_forge_domain_l2" }

# Version deps for external crates
serde = { version = "1.0", features = ["derive"] }
pretty_assertions = "1"
# etc.
```

## FFI Boundary Types

Each crate exports FFI-safe types via `ffi.rs`:

```rust
#[repr(C)]
pub struct MyStruct {
    field: u32,
}
```

FFI types are exported to provide C-compatible interfaces for cross-crate usage.
