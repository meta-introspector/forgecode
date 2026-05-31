# DASL Testing MCP Controllers

## Objective

Create MCP controllers for all tools in `~/dasl/dasl-testing` — a cross-implementation DAG-CBOR testing framework with 20+ CBOR implementations across 7 languages, 7 C round-robin binaries, 3 Rust microservices, and Python analysis pipelines. Each controller is a thin Rust MCP server (following the pattern documented in `~/DOCS/development-guide.md`) that invokes dasl-testing binaries via subprocess. The dasl-testing project itself is added as a git+file:// flake input (mirrored at `~/git/github.com/hyphacoop/dasl-testing.git/`) to provide pure-nix builds of all tools.

Success means: 8 MCP servers registered in `.mcp.json`, each exposing `tools/list` to the forge agent, with a skill at `.forge/skills/dasl-testing-mcp/` documenting the full tool surface.

## Implementation Plan

- [ ] 0. Verify git mirror and flake input for dasl-testing
  - The mirror already exists at `~/git/github.com/hyphacoop/dasl-testing.git/` with remote `local` configured in the checkout.
  - `~/dasl/dasl-testing/.git` points to `~/dasl/.git/modules/dasl-testing` (it's a git submodule).
  - Task: verify the mirror is up-to-date (push `main` from checkout → mirror).
  - Task: add `dasl-testing` as a flake input in forgecode's `flake.nix` with URL `git+file:///home/mdupont/git/github.com/hyphacoop/dasl-testing.git`.
  - Task: verify `nix flake check --no-build` passes with the new input.
  - Rationale: gitplan.org principle1 — all things from git; principle2 — all git mirrored in ~/git/.
  - Reference: `~/DOCS/flake-integration.md` for the full 7-step input addition process.

- [ ] 1. Create forge_dasl_gateway_mcp crate in `crates/forge_dasl_gateway_mcp/`
  - Wraps `services/gateway.py` which listens on port8010 and orchestrates microservice queries.
  - Tools: `gateway_list_services` (health check all microservices), `gateway_compare_hex` (compare hex output across 3+ microservices), `gateway_batch` (run batch encode/decode/comparison pipeline).
  - Subprocess invokes `python3 services/gateway.py --port 8010` with query arguments.
  - Uses `tokio::process::Command` with timeout for each tool call.
  - Runtime state: `Arc<RwLock<HashMap<String, GatewayStatus>>>` to cache service health.
  - Crate files go to `crates/forge_dasl_gateway_mcp/Cargo.toml` and `src/main.rs`.
  - Rationale: gateway is the entry point to all microservice interactions — needed before any cross-implementation testing.
  - Reference: `~/DOCS/development-guide.md` for the crate template pattern.

- [ ] 2. Create forge_dasl_service_mcp crate in `crates/forge_dasl_service_mcp/`
  - Wraps the 8 deploy scripts (`deploy_*.sh` in dasl-testing) for microservice lifecycle management.
  - Services to manage: serde_ipld_dagcbor (port8001), libipld (port8002), n0_dasl (port8003), python (port8004), js (port8005), go (port8006), pastebin (port8007), dashboard (port8008).
  - Tools: `service_start` (nix build + systemd user service install + start), `service_stop` (systemctl stop), `service_restart` (systemctl restart), `service_status` (health check + port check with `ss -tlnp`), `service_logs` (journalctl --user -u <service>).
  - Runtime state: `Arc<RwLock<HashMap<String, ServiceInfo>>>` tracking pid, port, uptime per service.
  - Port discovery via `find` in systemd unit files or `ss` output parsing.
  - Rationale: managing 8 microservices manually is error-prone — this MCP provides consistent lifecycle control.
  - Reference: `~/DOCS/system-manager.md` for systemd service patterns.

- [ ] 3. Create forge_dasl_roundrobin_mcp crate in `crates/forge_dasl_roundrobin_mcp/`
  - Wraps `round_robin.py` (63k lines, 20 implementations) and 7 C round-robin binaries from dasl-testing's flake.
  - C binaries (nix-built as packages: rr-tinycbor, rr-libcbor, rr-qcbor, rr-zcbor, rr-cncbor, rr-cborphine, rr-libmcu-cbor) are discovered via their nix store paths.
  - Tools: `roundrobin_run_python` (run round_robin.py with specific implementations, divergence types, test count), `roundrobin_run_c` (run a specific C binary with hex input), `roundrobin_compare_all` (run python + all C binaries on same input and report divergences), `roundrobin_list_implementations` (list all available implementations from round_robin.py).
  - Divergence types: decode-mismatch, encode-mismatch, crash, timeout, hex-diff, roundtrip-fail.
  - Uses background spawn with UUID run-tracking for long-running operations.
  - Rationale: core testing workflow — cross-implementation comparison is the main value of dasl-testing.
  - Reference: `~/DOCS/mcp-servers.md` for tool definition patterns.

- [ ] 4. Create forge_dasl_fuzz_mcp crate in `crates/forge_dasl_fuzz_mcp/`
  - Wraps the fuzzing harnesses in `harnesses/` directory (16 harness directories across C, C++, Go, Java, JS, Python, Rust).
  - Tools: `fuzz_list_harnesses` (list all available harnesses with language, status, crash count), `fuzz_run_harness` (start fuzzing a specific harness with configurable time/workers), `fuzz_get_crashes` (aggregate crash data from crash_inventory.json), `fuzz_build_harness` (build a specific harness via nix), `fuzz_corpus_stats` (stats about seed corpora for each harness).
  - Crash inventory files: `harnesses/crash_inventory.json`, `harnesses/fuzzing_inventory.json`.
  - Uses `harnesses/Makefile` for build commands where nix cross-ref is needed.
  - Rationale: fuzzing is disjoint from round-robin testing — harnesses exist for each language and need independent management.
  - Reference: `~/DOCS/architecture.md` for the RunningService lifetime pattern.

- [ ] 5. Create forge_dasl_lattice_mcp crate in `crates/forge_dasl_lattice_mcp/`
  - Wraps `lattice_generator.py` which generates test cases with 4-axis complexity scaling.
  - Tools: `lattice_generate` (generate test cases with parameters: depth, width, nesting, tags), `lattice_apply_sheaf` (run generated cases through a specific sheaf pipeline), `lattice_list_axes` (list available complexity axes and their ranges), `lattice_export_cases` (export generated cases as CAR file or JSON).
  - Output goes to `INPUT/` directory or stdout depending on arguments.
  - Rationale: lattice generation creates systematic coverage that random fuzzing misses.
  - Reference: `~/DOCS/development-guide.md` for serde_json output formatting.

- [ ] 6. Create forge_dasl_sheaf_mcp crate in `crates/forge_dasl_sheaf_mcp/`
  - Wraps the `sheaf/` pipeline directory which produces coverage matrices, sheaf restriction diagrams, and HTML reports.
  - Tools: `sheaf_run_pipeline` (execute the full sheaf analysis pipeline with configurable runners), `sheaf_generate_report` (produce HTML report from coverage data), `sheaf_compare_runs` (diff two sheaf runs to show coverage regressions/improvements), `sheaf_list_runners` (list available sheaf analysis runners).
  - The `sheaf/` directory has its own structure — the MCP reads coverage matrices from sheaf output.
  - Rationale: sheaf analysis is the post-processing/visualization step after round-robin and fuzzing.
  - Reference: `~/DOCS/mcp-servers.md` for tool categorization patterns.

- [ ] 7. Create forge_dasl_dashboard_mcp crate in `crates/forge_dasl_dashboard_mcp/`
  - Wraps `services/dashboard_service.py` (FastAPI dashboard on port8080) and `report/` HTML output.
  - Tools: `dashboard_status` (health check the dashboard web UI), `dashboard_get_report` (fetch a specific HTML report by name), `dashboard_list_reports` (list available reports in `report/` and `reports/` directories), `dashboard_export_data` (export raw test data as JSON for external consumption).
  - The dashboard service itself is managed by `forge_dasl_service_mcp` — this crate only provides query/read tools.
  - Rationale: separate read-only dashboard access from service lifecycle management.
  - Reference: `~/DOCS/system-manager.md` for service registration patterns.

- [ ] 8. Create forge_dasl_orchestrator_mcp crate in `crates/forge_dasl_orchestrator_mcp/`
  - Meta-server that coordinates the 7 MCPs above into a single coherent interface.
  - Tools: `orchestrator_run_test_plan` (stages: deploy services, generate lattice, run round-robin, run fuzz, run sheaf, generate report — all with one call), `orchestrator_get_pipeline_status` (status of a running test plan by run_id), `orchestrator_cancel_pipeline` (stop a running test plan), `orchestrator_get_recent_results` (last N test plan results with pass/fail summary).
  - Pipeline state machine: idle → deploying → generating → roundrobin → fuzzing → sheaf → reporting → done.
  - Uses `Arc<RwLock<HashMap<Uuid, PipelineState>>>` for run tracking.
  - Each pipeline stage spawns the corresponding MCP server as subprocess via `tokio::process::Command`.
  - Rationale: single-entry interface for the common "run everything" workflow.
  - Reference: `~/DOCS/architecture.md` for shared state patterns.

- [ ] 9. Register all 8 MCP servers in forgecode flake, `.mcp.json`, and system-manager
  - In `flake.nix`: add each crate as a package (nix build), app (`nix run .#forge-dasl-*`), and devShell dependency, following the pattern at `~/DOCS/flake-integration.md`.
  - In `.mcp.json`: add each server with `"command": "forge-dasl-<name>-mcp"` (stdio transport) following `~/DOCS/architecture.md` config format.
  - In `modules/system.nix`: add systemd service definitions for the gateway service (port8010) and dashboard (port8080) that run in the background following `~/DOCS/system-manager.md`.
  - Rationale: tools must be discoverable by forge to be useful.
  - Dependencies: tasks0-8 must be complete.

- [ ] 10. Create `.forge/skills/dasl-testing-mcp/` skill with full documentation
  - Skill covers: all 8 MCP server purposes, tool signatures, workflow patterns (microservice lifecycle → round-robin → sheaf → report), file format references, cross-implementation comparison results.
  - Reference files copied from dasl-testing: `services.txt` (port assignments), `IMPLEMENTATION_STATUS_REPORT.md` (implementation coverage), `MULTI_LANGUAGE_FUZZING_FINAL_STATUS.md` (fuzzing inventory).
  - Rationale: agents need documentation to effectively use these MCP servers.
  - Dependencies: tasks1-9 must be complete.

## Verification Criteria

- `nix flake check --no-build` passes with all 8 new MCP server packages and dasl-testing flake input
- `cargo check -p forge-dasl-*` passes for all 8 crates
- Each crate's `tools/list` MCP response shows the expected tools when run over stdio
- `.mcp.json` contains all 8 entries with correct command names
- `modules/system.nix` has gateway (port8010) and dashboard (port8080) services that can be deployed
- `.forge/skills/dasl-testing-mcp/SKILL.md` documents all tools and workflow patterns
- The orchestrator meta-server can successfully pipe a test plan through all stages

## Potential Risks and Mitigations

1. **Python environment for gateway/lattice scripts**
   - Impact: MCP crashes when subprocess invokes `python3` and dependencies aren't available
   - Likelihood: Medium
   - Mitigation: Include `pkgs.python3Packages.{fastapi,requests,uvicorn}` in dasl-testing's flake and invoke via nix-built derivation paths
   - Contingency: Fall back to shell-based discovery of `which python3` and check `import` before tool execution

2. **C round-robin binaries not on PATH**
   - Impact: `roundrobin_run_c` tool fails with "command not found"
   - Likelihood: Medium
   - Mitigation: Each MCP discovers binaries at startup by reading dasl-testing flake's `packages` output metadata
   - Contingency: Accept user-configured binary paths in tool arguments

3. **Port conflicts on microservices**
   - Impact: Service MCP reports service as unhealthy when port is occupied by unrelated process
   - Likelihood: Low
   - Mitigation: Use `ss -tlnp` to check port availability before starting; report conflicting PID
   - Contingency: Accept custom port assignment in tool arguments

4. **Long-running operations (round-robin, fuzzing) timing out**
   - Impact: MCP connection drops while tool is still running
   - Likelihood: High
   - Mitigation: Spawn long-running ops in background with UUID tracking; provide `get_results(run_id)` polling tools
   - Contingency: Implement progress streaming via MCP notifications if protocol supports it

5. **Orphaned microservices after MCP crash**
   - Impact: Services keep running on ports, interfering with next test run
   - Likelihood: Medium
   - Mitigation: Scan for orphaned service PIDs on startup and offer cleanup; track PIDs in shared state
   - Contingency: Provide `service_cleanup_all` tool that kills any process on known service ports

6. **Shellcheck failures on deploy scripts if reused**
   - Impact: Script-rewriting tools produce non-Shellcheck-compliant code
   - Likelihood: Low
   - Mitigation: All MCP servers invoke existing scripts as-is; no script modification
   - Contingency: N/A — scripts are read-only from MCP perspective

## Alternative Approaches

1. **Single monolithic MCP server instead of 8 small ones**
   - Pros: Single binary, simpler deployment, shared state is straightforward
   - Cons: Violates gitplan.org principle of "one decl per flake step"; hard to maintain 50k lines of Rust; any crash takes down all functionality
   - Recommendation: **Not chosen** — 8 small servers follow the existing pattern in forgecode (nora, pipelight, parquet are all separate) and allow independent debugging

2. **Python-based MCP servers using `fastmcp` instead of Rust**
   - Pros: Direct access to dasl-testing's Python libraries without subprocess overhead
   - Cons: Introduces Python runtime dependency on forgecode; no existing Python MCP pattern in the codebase; RMCP vendored in Rust already
   - Recommendation: **Not chosen** — Rust MCP servers with subprocess invocation follows the established forgecode pattern (`~/DOCS/architecture.md`)

3. **Wrap each dasl-testing CLI binary as an individual MCP server**
   - Pros: Maximum granularity — one MCP per binary
   - Cons: 20+ MCP servers to register and manage; most CLIs are too simple to warrant a server
   - Recommendation: **Not chosen** — grouping by domain (gateway, service, fuzz, etc.) provides the right level of abstraction

## Assumptions

- dasl-testing mirror at `~/git/github.com/hyphacoop/dasl-testing.git/` is kept synchronized with the checkout at `~/dasl/dasl-testing/`
- All C round-robin binaries build successfully from dasl-testing's flake.nix via `mkCRoundRobin`
- Microservices use ports8001-8008 as documented in the deploy_*.sh scripts
- The `sheaf/` directory produces machine-readable coverage matrices (JSON), not just HTML
- Forge agent running in the same nix shell as `nix develop` has all forge-dasl-* binaries on PATH

## Dependencies

- Forgecode flake must already have `rmcp` vendored (confirmed: `vendor/rmcp/` exists)
- Workspace Cargo.toml must have `serde_json`, `tokio`, `anyhow`, `tracing`, `rmcp` as workspace dependencies (confirmed from `~/DOCS/development-guide.md` and `crates/forge_nora_mcp/Cargo.toml`)
- `~/git/solana.solfunmeme.com/` must be accessible for any new mirrors (confirmed)
- systemd user services must be available (confirmed from `~/DOCS/system-manager.md`)

## Notes

- All Rust crate files go in `crates/forge_dasl_*/` following the existing naming convention (`forge_nora_mcp`, `forge_parquet_mcp`, `forge_pipelight_mcp`)
- Scripts and config files that assist during development go to `~/05/30/` following gitplan.org principle of dated directories
- No files go to `/tmp` — use `~/05/30/` for any temporary working files
- The `~/DOCS/` directory already contains all reference documentation needed: `architecture.md`, `mcp-servers.md`, `flake-integration.md`, `development-guide.md`, `system-manager.md`
- Each crate should be kept under 600 lines of Rust to remain maintainable — complex logic stays in dasl-testing's Python/C scripts
