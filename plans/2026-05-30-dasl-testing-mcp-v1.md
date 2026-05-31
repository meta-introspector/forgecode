# DASL Testing MCP Controllers

## Objective

Create a suite of MCP tools that expose all capabilities of `~/dasl/dasl-testing` — the cross-implementation CBOR/DAG-CBOR testing infrastructure — as MCP server tools usable by AI agents. This enables agents to run round-robin comparisons, fuzzing campaigns, lattice generation, sheaf coverage analysis, and service orchestration without leaving their workflow.

## Background

`~/dasl/dasl-testing` is the DASL CBOR cross-implementation testing framework. It has 20+ CBOR/DAG-CBOR implementations across Python, Rust, Go, JavaScript, Java, C, and C++, all feeding into a central round-robin oracle that detects divergences. The infrastructure includes HTTP microservices (5 services + gateway), 7 C round-robin binaries, Rust fuzzing harnesses, a lattice-based test generator, and a sheaf-theoretic coverage mapping pipeline. No MCP integration currently exists.

The existing forgecode pattern (see `~/DOCS/development-guide.md`) is to create Rust MCP server crates using `rmcp::Router` with stdio transport, register them in `.mcp.json`, and expose as flake packages. This plan follows that pattern.

## Implementation Plan

### Phase 1: Infrastructure — Git mirror, flake input, gateway MCP

- [ ] 1. Create a git mirror for dasl-testing at `~/git/solana.solfunmeme.com/dasl-testing` and push the current state. Add dasl-testing as a flake input to forgecode's `flake.nix` using the `git+file://` mirror URL pattern, referencing `~/DOCS/flake-integration.md` step 1-3.

- [ ] 2. Create `crates/forge_dasl_gateway_mcp/` — a Rust MCP server (`rmcp::Router`) that talks to the existing `gateway.py` HTTP API at port 8010. Expose tools for listing services (`GET /services`), checking health (`GET /health`), comparing a single hex input across all implementations (`POST /compare`), and batch comparison (`POST /compare-batch`). Register as `"dasl-gateway"` in `.mcp.json`. Add as flake package and devShell dep.

- [ ] 3. Create `crates/forge_dasl_service_mcp/` — a Rust MCP server that manages the life cycle of the 5 microservices (serde_ipld_dagcbor:8007, libipld:8008, n0_dasl:8009, dag-cbrrr:8001, python-libipld:8002). Expose tools for `start_service(impl_name)`, `stop_service(impl_name)`, `restart_service(impl_name)`, `list_services`, `get_service_logs(impl_name, lines)`, and `service_health(impl_name)`. Each service can be spawned via `nix run` or direct binary invocation. Register as `"dasl-service"` in `.mcp.json`.

### Phase 2: Testing orchestration MCP servers

- [ ] 4. Create `crates/forge_dasl_roundrobin_mcp/` — a Rust MCP server wrapping `round_robin.py`. Expose tools for `run_round_robin(input_sets, impls, batch_size, timeout, divergences_only)` that invokes the Python script and returns the parsed JSON results. Also expose `list_implementations` (returns all 20 supported impls grouped by language), `list_input_sets` (fixtures, raw, adversarial, crashes, timing, lattice, all), `get_divergence_summary(input_sets, impls)` that runs and summarizes, and `get_implementation_status(impl_name)` that checks if a given impl binary is available on disk. Register as `"dasl-roundrobin"` in `.mcp.json`.

- [ ] 5. Create `crates/forge_dasl_fuzz_mcp/` — a Rust MCP server that triggers and manages fuzzing campaigns across all Rust harnesses (serde_ipld_dagcbor, libipld, n0_dasl). Expose tools for `start_fuzz_campaign(harness, iterations, corpus_dir, crash_dir)` that spawns a fuzzer in background, `list_fuzz_campaigns` listing active/completed runs, `get_fuzz_results(harness, run_id)` returning crash stats and recent crash files, `get_crash_details(crash_path)` reading `.hex` + `.txt` crash artifacts, and `fuzz_all_harnesses(iterations)` that fans out across all Rust crates. Register as `"dasl-fuzz"` in `.mcp.json`.

- [ ] 6. Create `crates/forge_dasl_croundrobin_mcp/` — a Rust MCP server wrapping the 7 C round-robin binaries (rr-tinycbor, rr-libcbor, rr-qcbor, rr-zcbor, rr-cncbor, rr-cborphine, rr-libmcu-cbor). Expose tools for `decode_with(binary_name, hex_data)` piping hex to a specific C binary and parsing JSON output, `compare_across_c_impls(hex_data)` running all 7 C impls on the same input and detecting divergences, and `list_c_implementations` listing available built C binaries. Each binary is available from `nix build .#c-cbor-all`. Register as `"dasl-c-roundrobin"` in `.mcp.json`.

### Phase 3: Analysis and reporting MCP servers

- [ ] 7. Create `crates/forge_dasl_lattice_mcp/` — a Rust MCP server wrapping `lattice_generator.py`. Expose tools for `generate_lattice(depth, breadth, diversity, link_fraction, output_dir)` that runs the Python generator and returns the corpus statistics, and `get_lattice_corpus_stats(output_dir)` showing how many test cases were generated across the 4 axes. Register as `"dasl-lattice"` in `.mcp.json`.

- [ ] 8. Create `crates/forge_dasl_sheaf_mcp/` — a Rust MCP server wrapping the sheaf pipeline tools. Expose tools for `run_sheaf_pipeline` running generate_catalog → build_matrix → glue_stalks → sheaf_report, `get_coverage_matrix(impl_pattern)` querying the built matrix per implementation, `get_sheaf_report_path` returning path to the generated HTML report, `list_cbor_features` listing all 40+ tested CBOR feature categories from the catalog, and `feature_coverage(feature_name, impl)` checking which implementations cover a specific CBOR feature. Register as `"dasl-sheaf"` in `.mcp.json`.

- [ ] 9. Create `crates/forge_dasl_dashboard_mcp/` — a Rust MCP server wrapping the FastAPI dashboard at port 8080. Expose tools for `get_dashboard_summary` (from `/summary`), `get_implementation_results(impl_name)` (from `/results/{impl_name}`), `get_all_divergences` (from `/divergences`), and `get_dashboard_url` returning the dashboard URL. Register as `"dasl-dashboard"` in `.mcp.json`.

### Phase 4: Super-orchestrator and integration

- [ ] 10. Create `crates/forge_dasl_orchestrator_mcp/` — a meta-orchestrator that delegates to all 8 dasl MCP servers above. Expose high-level compound tools like `run_full_test_suite(input_sets, impls)` that runs round-robin + fuzzing + lattice in sequence, `investigate_divergence(hex_data)` that pipes the same hex through all MCP servers and builds a unified report, `generate_regression_report` that runs sheaf + round-robin + lattice and produces a summary, and `start_all_services` / `stop_all_services` for bulk service management. This crate coordinates via subprocess calls to the other MCP servers. Register as `"dasl-orchestrator"` in `.mcp.json`.

- [ ] 11. Create `.forge/skills/dasl-testing-mcp/SKILL.md` documenting all 10 MCP servers (gateway, service, round-robin, fuzz, c-roundrobin, lattice, sheaf, dashboard, orchestrator) with tool listings, usage examples, service port table, and inter-server dependency graph. Include `references/` directory with the CBOR implementation table and the fixture format reference.

- [ ] 12. Wire all 10 MCP servers into `modules/system.nix` as systemd services (for HTTP-based servers like dashboard and service-managed) and into `.mcp.json` as stdio subprocess entries (for the rest). Add all flakes packages, apps, and devShell deps. Verify `nix flake check --no-build` passes.

## Verification Criteria

- Each of the 10 MCP servers compiles via `cargo check -p forge-dasl-*-mcp`
- `nix eval` shows all 10 packages under `.#packages.x86_64-linux`
- Each MCP server responds to `tools/list` over stdio with at least one tool defined
- The gateway MCP server can query the live gateway at port 8010 and return service list
- The round-robin MCP server can run `list_implementations` without invoking the full script
- `nix flake check --no-build` passes

## Potential Risks and Mitigations

1. **Python script environment dependencies**
   Mitigation: Each MCP server that wraps a Python script should verify the script exists and the Python environment is available before invoking. Use `which python3` and path checks. The `round_robin.py` and `lattice_generator.py` scripts have Python dependencies that may not be in the system Python.

2. **Service ports may conflict**
   Mitigation: The service MCP server should check port availability before starting a service (`ss -tlnp | grep :PORT`). Services run on fixed ports (8001-8002, 8007-8009, 8010, 8080) — document port assignments and detect conflicts at startup.

3. **Long-running operations blocking the MCP response**
   Mitigation: Round-robin and fuzzing campaigns can take minutes. Use background spawning (`tokio::spawn`) with a UUID-based run-tracking system. Each `start_*` call returns a `run_id` immediately; a separate `get_results(run_id)` tool polls for completion.

4. **C round-robin binaries may not be on PATH**
   Mitigation: Store the path to `c-cbor-all` build result (`nix build .#c-cbor-all`) and fall back to locating binaries via `find` or `locate`. The MCP server should accept a `c_binary_dir` config option or auto-discover from the flake.

5. **Service processes may orphan on MCP server crash**
   Mitigation: The service MCP server should track spawned PIDs in an `Arc<RwLock<HashMap>>` (following the agent orchestrator pattern in `crates/forge_tmux_agent_orchestrator_mcp/src/main.rs`). On startup, it should scan for orphaned service processes and optionally clean them up.

## Alternative Approaches

1. **Single monolith MCP server with many tools instead of 10 separate servers.**
   Trade-off: Less config overhead (one `.mcp.json` entry, one flake package) but harder to maintain, test, and deploy independently. The 10-server approach follows the Unix philosophy (each does one thing well) and matches the existing forgecode pattern. Recommended: 10 servers.

2. **Wrap everything using Python MCP SDK instead of Rust.**
   Trade-off: Faster to prototype (could reuse Python scripts as-is) but introduces a Python runtime dependency for the MCP layer and doesn't follow forgecode's existing `rmcp`-based pattern. The Rust approach gives cleaner integration with the flake build system and consistent tool registration. Recommended: Rust.

3. **Direct HTTP-to-MCP bridge for the gateway instead of a custom crate.**
   Trade-off: Could use a generic HTTP-to-MCP proxy that forwards POST requests. But the existing gateway doesn't expose all the endpoints we need (no fuzzing, no sheaf, no lattice) — these would require custom tool logic anyway. Recommended: full Rust crate per server.

## Files to Create

- `crates/forge_dasl_gateway_mcp/Cargo.toml` + `src/main.rs`
- `crates/forge_dasl_service_mcp/Cargo.toml` + `src/main.rs`
- `crates/forge_dasl_roundrobin_mcp/Cargo.toml` + `src/main.rs`
- `crates/forge_dasl_fuzz_mcp/Cargo.toml` + `src/main.rs`
- `crates/forge_dasl_croundrobin_mcp/Cargo.toml` + `src/main.rs`
- `crates/forge_dasl_lattice_mcp/Cargo.toml` + `src/main.rs`
- `crates/forge_dasl_sheaf_mcp/Cargo.toml` + `src/main.rs`
- `crates/forge_dasl_dashboard_mcp/Cargo.toml` + `src/main.rs`
- `crates/forge_dasl_orchestrator_mcp/Cargo.toml` + `src/main.rs`
- `.forge/skills/dasl-testing-mcp/SKILL.md` + `references/`

## Files to Modify

- `Cargo.toml` (workspace members for all 10+ new crates)
- `flake.nix` (new input for dasl-testing, 10 new packages + apps + devShell deps)
- `.mcp.json` (10 new MCP server entries)
- `modules/system.nix` (HTTP-based dasl servers as systemd services)
