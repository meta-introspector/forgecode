# Tile/MASL Architecture: Unified View Composition System

## Objective

Build a content-addressed, graph-native tile architecture that composes views (Firefox flamegraphs, pastebin tiles, media gallery) into a unified system powered by a Rust tile server with Nix-deployed systemd/nginx infrastructure. Integrate all existing tools (cargo-vendormod, deep_scanner, zos-server, pastebin) into a coherent developer workflow with skill-based automation.

## Context

From the existing conversation and codebase analysis:

- **DeepWiki Q&A** (`pastebin:20260522_145034`): Proposes splitting Forge into content-addressed crates using MASL (schema language), IPLD/dag-cbor (data model), CIDs (content addressing), and tiles (labelled subgraphs)
- **enhanced-pastebin-media** (`/home/mdupont/enhanced-pastebin-media/`): Existing Rust warp-based media gallery server with MIDI, PlantUML tiles, and a web frontend
- **decl-splitter/lattice** (`forgecode/.forge/skills/decl-splitter-lattice/`): Tool that already splits Rust crates into FFI-boundary plugin crates with CAR archival of excluded decls
- **cargo-vendormod** (`~/projects/cargo-clean/tools/cargo-vendormod/`): Tool for managing git submodules across repos with dependency analysis
- **deep_scanner** (`~/projects/dasl/IMPL/deep_scanner/`): Scanner for reading all files in a corpus
- **zos-server** (`~/zos-server/`): Minimal server with FFI plugin support
- **pastebin** (`~/pastebin/`): Daily-used paste tool with IPFS, sheaf, DASL support
- **DLS** (`~/DOCS/`): Collected documentation archive
- **Nix repos** (`~/nix/`): ~49 repos with unfinished Nix work
- **Git mirrors** (`~/git/`): ~100 bare repos

## Implementation Plan

- [ ] 1. **Create WASM flamegraph viewer** — Build a Rust WASM component that renders perf/flamegraph data as an interactive SVG. Use existing perf data parsing libraries or a minimal custom parser. The component should accept JSON-structured stack traces and render zoomable flamegraph tiles. Output as a WebAssembly module with a JavaScript interop layer for embedding in the web frontend.
- [ ] 2. **Extend enhanced-pastebin-media to tile server** — Refactor the existing warp-based media gallery to serve tiles as first-class entities. Each tile gets: a CID-based identifier (using the existing ipld-car infrastructure), typed metadata (flamegraph, MIDI, PlantUML, SVG, etc.), and a REST API for tile composition. Add a `/tile/{cid}` endpoint that resolves tiles and returns rendered content.
- [ ] 3. **Implement tile composition engine** — Design an MVC-like pattern where tiles can reference other tiles. A "dashboard" tile composes sub-tiles (flamegraph + MIDI player + PlantUML diagram) into a single view. Use IPLD dag-cbor for tile references. The existing `tools/decl_splitter/src/lattice.rs` CAR writer can be adapted for tile storage.
- [ ] 4. **Create MASL-to-tile codegen pipeline** — Define a minimal MASL schema format (YAML-based) that describes tile types, their fields, and composition rules. Generate Rust types + IPLD schemas from MASL. This leverages the existing decl-splitter's syn parsing infrastructure for code generation.
- [ ] 5. **Build Nix derivation for tile server** — Create a `flake.nix` that builds the Rust tile server as a Nix package, generates a systemd service unit, and configures nginx as a reverse proxy. Use the existing flake infrastructure in `forgecode/flake.nix` as a reference pattern.
- [ ] 6. **Add CAR file MVC for tile storage** — Implement read/write/delete operations on CAR files via the tile server API. Each session/tile gets its own CAR file with appended blocks. The MVC pattern: Model (CAR block store), View (tile rendering), Controller (API endpoints). Build on the existing CAR writer from `lattice.rs:1086-1190`.
- [ ] 7. **Integrate pastebin as tile source** — Extend the pastebin tool (`~/pastebin/`) to emit tiles instead of plain text pastes. Each paste becomes a tile with CID, and the tile server can render paste content inline (code highlighting, diagram preview). The existing `~/pastebin/src/view.rs` and `~/pastebin/src/sheaf.rs` are entry points.
- [ ] 8. **Wire cargo-vendormod for cross-repo management** — Use `cargo-vendormod` (`~/projects/cargo-clean/tools/cargo-vendormod/`) to manage the submodule relationships between forgecode, pastebin, enhanced-pastebin-media, zos-server, and dasl. Create a `.gitmodules` file at the workspace root that references all repos as submodules with the gemini-annotation markup pattern from `~/projects/dasl/rust/ipld-core/GITMODULES_DOCUMENTATION.md`.
- [ ] 9. **Mirror all repos in ~/git/ as bare repos** — Write a script that iterates over all repos in ~/nix/, ~/projects/, ~/pastebin/, ~/zos-server/, ~/enhanced-pastebin-media/, and creates/updates bare git mirrors in ~/git/. Use the `--mirror` flag with periodic `git remote update` for synchronization.
- [ ] 10. **Integrate deep_scanner for corpus analysis** — Apply `~/projects/dasl/IMPL/deep_scanner/` to scan all mirror repos and create a searchable corpus. The scanner emits machine-readable output (JSON/CBOR) that feeds into the tile server for code search across repos.
- [ ] 11. **Build zos-server FFI plugin for tile server** — Create a minimal FFI plugin using the zos-server pattern (`~/zos-server/`) that bridges the Rust tile server with external computations (flamegraph generation, MIDI processing). The plugin communicates via dag-cbor over the FFI boundary.
- [ ] 12. **Create automation skills for each tool** — For each integrated tool (cargo-vendormod, deep_scanner, pastebin, zos-server, enhanced-pastebin-media), create a Forge skill under `.forge/skills/<tool-name>/` with SKILL.md documentation and scripts. Follow the pattern established by `.forge/skills/decl-splitter-lattice/`.

## Verification Criteria

- Flamegraph viewer renders perf data as interactive zoomable SVG in the browser
- Tile server serves CIDs via REST API and resolves composition references
- Nix `nix build` produces a working systemd service with nginx
- CAR file MVC: round-trip write → CID → read → verify content
- pastebin emits tiles; tile server renders paste content
- cargo-vendormod manages all repos as a unified multi-repo workspace
- All ~100+ repos mirrored in ~/git/ as bare repos
- deep_scanner produces corpus index covering all mirror repos
- zos-server FFI plugin communicates with tile server via dag-cbor
- Each tool has a documented Forge skill with working scripts

## Potential Risks and Mitigations

1. **WASM flamegraph performance with large perf profiles**
   Mitigation: Implement level-of-detail rendering — aggregate deep stacks at low zoom, expand on zoom-in. Use Web Workers for parsing.

2. **CAR file storage scaling with many tiles**
   Mitigation: Split into sharded CAR files by tile type/session. Implement LRU cache for hot tiles. Use the existing circular queue resampling from the telemetry design.

3. **cargo-vendormod submodule conflicts across repos**
   Mitigation: Use the gemini-annotation pattern from GITMODULES_DOCUMENTATION.md to version and track submodule relationships. Implement a merge strategy for conflicting submodule paths.

4. **Nix build complexity with multi-repo workspace**
   Mitigation: Use Nix flakes with `follows` pattern for dependency injection. Build each component crate independently before assembling the final tile server binary.

5. **deep_scanner corpus size with 100+ repos**
   Mitigation: Implement incremental scanning — only re-scan repos that have changed since last scan. Store corpus index in CAR files for deduplication.

## Alternative Approaches

1. **Monolithic tile server vs. micro-tile-server pattern**: Instead of one tile server, each tile type could be its own micro-server (flamegraph-tile-server, midi-tile-server, plantuml-tile-server). Trade-off: more deployment complexity but independent scaling and fault isolation.

2. **CAR files vs. SQLite for tile storage**: CAR files give us native IPLD/CID compatibility and content addressing, but SQLite provides better query performance and indexing. Consider a hybrid: CAR for archival/addressing, SQLite for metadata indexing.

3. **WASM flamegraph vs. server-side SVG rendering**: WASM enables client-side interactivity (zoom, search) without server round-trips, but adds ~1MB download. Server-side SVG rendering is lighter but requires more backend logic for interactions. Start with server-side rendered SVG tiles and add WASM interactivity for the flamegraph component.

4. **Nix flakes vs. docker-compose for deployment**: Nix provides reproducible builds and systemd integration natively, but has a steeper learning curve. Docker-compose is more widely understood. Use Nix for the build system and optionally export Docker images from Nix for deployment.

5. **Standalone tile skill vs. extension of decl-splitter-lattice**: The tile generation could be a new skill or an extension of the existing decl-splitter-lattice skill. Since the CAR writer already exists in `lattice.rs`, extending that skill is faster. Create a new skill if the tile system becomes independent from the decl-splitting pipeline.
