# Symlink Resolution and Tool Integration

## Objective

Enhance forgecode's file lookup capabilities to properly handle symlinks, integrate with shmem for caching, leverage vendormod for dependency resolution, and use the lean4 splitter for module analysis - all exposed via the ZOS plugin system.

## Implementation Plan

- [ ] 1. Enhance `forge_fs/src/symlink.rs` with improved symlink chain resolution and caching
- [ ] 2. Add shmem-based caching layer for resolved symlink paths in `forge_fs`
- [ ] 3. Integrate vendormod dependency scanning with symlink resolution
- [ ] 4. Connect lean4 splitter module analysis to symlink traversal
- [ ] 5. Create ZOS plugin endpoint for symlink resolution service
- [ ] 6. Add unit tests for symlink resolution with chains
- [ ] 7. Verify integration with existing file lookup APIs

## Verification Criteria

- [ ] Symlink chains resolve correctly up to 40 levels
- [ ] Cache hits return in <1ms after initial resolution
- [ ] Vendormod integration correctly follows symlinks in dependency trees
- [ ] Lean4 splitter correctly processes symlinked modules
- [ ] ZOS plugin endpoint is accessible via plugin API
- [ ] All existing tests continue to pass

## Potential Risks and Mitigations

1. **Circular symlink chains**
   Mitigation: Implement cycle detection in addition to depth limiting

2. **Cache invalidation**
   Mitigation: Use file modification time or inode-based cache keys

3. **Cross-platform compatibility**
   Mitigation: Test on both Linux and macOS (symlink behavior differs)

## Alternative Approaches

1. **Use existing `std::fs::canonicalize`**: Simpler but doesn't allow custom caching
2. **Use `nix` crate directly**: More control but adds dependency complexity