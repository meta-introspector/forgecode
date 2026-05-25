# Parameter Replacement Patterns for Lattice Generation

## Common Patterns

### String Replacement (before / after)
```
use crate::TypeName → use forge_l1_c0::TypeName
use crate::{A, B} → use forge_l1_c0::A; use forge_l1_c0::B;
```

### Identifier Extraction
```rust
// Extract crate name from use statement
use serde::{Deserialize, Serialize};
// → crate_name = "serde"

// Extract module from use statement
use crate::module::Type;
// → module = "module", type = "Type"
```

### Path Splitting
```
crate::path::to::Type
// → crate, module = "path::to", type = "Type"

serde_json::Value
// → crate = "serde_json", type = "Value"
```

## Technical Implementation

### Use Statement Parsing
Uses syn's AST visitor to find `ItemUse` nodes and extract crate names.

### Crate Name Mapping
```yaml
# Underscore to hyphen for Cargo.toml
use: async_trait → toml: async-trait
```

### Module Path Resolution
Checks if a type is defined locally or in a dependency crate:
1. Check `decl_info` for type definitions
2. Check `type_to_crate` mapping for which crate owns the type
3. Rewrite use statement to point to correct crate

## Error Patterns

### E0432: Unresolved Import
Cause: `crate::` reference to type not found
Fix: Rewrite to dependency crate or exclude decl

### E0433: Cannot Find Module
Cause: External crate not in scope
Fix: Add to `external_deps` in Cargo.toml

### E0116: Cross-crate Impl
Cause: Impl for type in different crate
Fix: Move impl to owning crate or exclude

### E0277: Trait Bound Not Satisfied
Cause: nom parser round-trip issue
Fix: Exclude decl or handle nom specially
