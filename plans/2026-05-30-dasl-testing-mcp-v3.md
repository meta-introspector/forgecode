# DASL Testing MCP Controllers — v3 (gitplan.org Edition)

## Objective

Create MCP controllers for all tools in `~/dasl/dasl-testing` using the
principles from `~/05/28/gitplan.org`:

1. **All things are built from git using nix using nora** — cargo-vendormod
   manages subdirectories, nix flakes build everything, nora publishes packages
2. **One decl per flake** — each MCP server lives in its own git repo with its
   own flake.nix, consumed as a flake input by forgecode
3. **No inline code** — all scripts go to `~/05/30/`, crates go to their own
   repos, we never inline code into forgecode crates/ directly
4. **Pipelight** orchestrates the build (cargo vendormod) → publish (nora) →
   deploy (system-manager) pipeline
5. **Git mirrors** at `~/git/github.com/` — all repos consumed as
   git+file:// inputs

Success means: 8 standalone MCP server repos, each with their own flake.nix,
built by nix, published to nora, consumed as flake inputs by forgecode,
registered in `.mcp.json`, with a skill at `.forge/skills/dasl-testing-mcp/`.

Note: Tool names throughout this plan (like "gateway_list_services") are MCP
tool identifiers, not code snippets. They follow the snake_case convention
defined in the rsMCP specification.

## Implementation Plan

### Phase 0: Infrastructure prep

- [ ] **0.1** Push the dasl-testing checkout to its git mirror at
  `~/git/github.com/hyphacoop/dasl-testing.git/` so the mirror has the latest
  commit before we use it as a flake input
- [ ] **0.2** Create 8 bare git repos at `~/git/github.com/mdupont/`
  named forge-dasl-gateway-mcp, forge-dasl-service-mcp, forge-dasl-roundrobin-mcp,
  forge-dasl-fuzz-mcp, forge-dasl-croundrobin-mcp, forge-dasl-lattice-mcp,
  forge-dasl-sheaf-mcp, and forge-dasl-dashboard-mcp
- [ ] **0.3** Run cargo-vendormod vendoring init on `~/dasl/dasl-testing` to
  generate the initial vendoring config for its crate dependencies and submodules
- [ ] **0.4** Write `~/05/30/scaffold-dasl-mcp-crate.sh` — a reusable script
  that creates a new MCP server repo from a template, builds it with
  rustPlatform.buildRustPackage flake, and pushes the template to the bare mirror
- [ ] **0.5** Write `~/05/30/register-dasl-mcp.sh` — a reusable script that
  adds a git+file:// flake input to forgecode flake.nix, updates flake.lock,
  and registers the server in .mcp.json
- [ ] **0.6** Verify dasl-testing is already a forgecode flake input from the
  git mirror, and all 8 MCP server repos will be consumed as flake inputs

### Phase 1: Service Controllers (gateway + services)

- [ ] **1.1** Run the scaffold script to create forge-dasl-gateway-mcp repo
  with these MCP tools:
  - gateway_list_services — displays all available dasl-testing services
  - gateway_compare_hex — compares hex output across implementations
  - gateway_batch — runs a batch comparison and returns results
  Backend: wraps the dasl gateway process (cargo run --bin dasl-gateway)
- [ ] **1.2** Run the scaffold script to create forge-dasl-service-mcp repo
  with these MCP tools:
  - service_start — starts a named microservice (ports 8001-8009)
  - service_stop — stops a named microservice
  - service_restart — restarts a named microservice
  - service_status — lists all services and their health
  - service_logs — fetches recent logs from a named service
  Backend: wraps 5 microservices from dasl-testing (gateway at 8010,
  roundrobin at 8001-8002, fuzz at 8003-8004, dashboard at 8080)
- [ ] **1.3** Run the register script for both gateway-mcp and service-mcp
  to add them as forgecode flake inputs and register in .mcp.json
- [ ] **1.4** Verify: nix eval returns a name for both packages, both appear
  in .mcp.json

### Phase 2: Testing Controllers (round-robin + fuzz + C)

- [ ] **2.1** Scaffold forge-dasl-roundrobin-mcp with tools:
  - rr_list_impls — lists all 20 CBOR/DAG-CBOR implementations
  - rr_run_divergence — runs one of 6 divergence types across selected impls
  - rr_run_all — runs the full round-robin across all implementations
  - rr_results — retrieves results for a specific run by run id
  Backend: invokes python3 harnesses/round_robin.py from dasl-testing
- [ ] **2.2** Scaffold forge-dasl-fuzz-mcp with tools:
  - fuzz_run_harness — starts fuzzing on serde_ipld_dagcbor, libipld, or n0_dasl
  - fuzz_get_queue — shows coverage queue size for a running harness
  - fuzz_findings — lists crashes and errors found by a harness
  - fuzz_status — lists all running fuzzing sessions with their PIDs
  Backend: invokes cargo fuzz or raw fuzzRunner binaries from dasl-testing
- [ ] **2.3** Scaffold forge-dasl-croundrobin-mcp with tools:
  - crr_list_bins — lists the 7 C round-robin binary names
  - crr_run — runs a specific C binary with a given iteration count
  - crr_compare — compares output between two C implementations by run id
  Backend: invokes prebuilt C binaries from dasl-testing
- [ ] **2.4** Register all 3 in forgecode flake + .mcp.json via the register
  script
- [ ] **2.5** Verify each server returns correct tools via MCP tools/list

### Phase 3: Analysis Controllers (lattice + sheaf + dashboard)

- [ ] **3.1** Scaffold forge-dasl-lattice-mcp with tools:
  - lattice_generate — generates a CBOR lattice with 4-axis complexity
  - lattice_verify — checks whether a lattice is valid DAG-CBOR
  - lattice_metrics — returns complexity metrics for a lattice
  Backend: invokes python3 analysis/lattice_generator.py from dasl-testing
- [ ] **3.2** Scaffold forge-dasl-sheaf-mcp with tools:
  - sheaf_run — runs the full sheaf pipeline producing coverage matrices +
    HTML report
  - sheaf_get_coverage — returns the latest coverage matrix as JSON
  - sheaf_open_report — prints the path to the rendered HTML report
  Backend: invokes the sheaf.py pipeline from dasl-testing
- [ ] **3.3** Scaffold forge-dasl-dashboard-mcp with tools:
  - dash_status — reports whether the FastAPI dashboard is running on 8080
  - dash_health — queries the /health endpoint and returns response
  - dash_open — prints the dashboard URL to open in a browser
  Backend: queries http://127.0.0.1:8080 or starts dashboard as subprocess
- [ ] **3.4** Register all 3 in forgecode flake + .mcp.json via the register
  script
- [ ] **3.5** Verify each server returns correct tools via MCP tools/list

### Phase 4: Orchestration + Skill

- [ ] **4.1** Scaffold forge-dasl-orchestrator-mcp with tools:
  - dasl_status — checks readiness of all 8 sub-servers
  - dasl_full_test_run — runs gateWay→roundrobin→fuzz→lattice→sheaf→dashboard
    in sequence
  - dasl_report — returns the aggregate report file path for a run id
  - dasl_restart_all — restarts all 8 MCP servers via the service controller
  Backend: delegates to all 8 sub-servers, no direct subprocess invocation
- [ ] **4.2** Register orchestrator in forgecode flake + .mcp.json
- [ ] **4.3** Create `.forge/skills/dasl-testing-mcp/SKILL.md` documenting
  all 9 MCP servers (8 controllers + orchestrator), their tool surface, port
  assignments, and orchestration flow diagram
- [ ] **4.4** Verify dasl_status returns all 8 servers as ready

### Phase 5: Pipelight pipeline

- [ ] **5.1** Define a pipelight.yaml in the dasl-testing repo that runs
  cargo-vendormod vendoring update on flake.lock changes, then pushes to the
  mirror
- [ ] **5.2** Define a pipelight pipeline in forgecode that listens to dasl-
  testing mirror updates (or manual trigger) and runs nix flake update
  dasl-testing to pull in new tool versions
- [ ] **5.3** Verify the pipeline can be triggered and completes without error

## Verification Criteria

- All 9 MCP server repos exist at `~/git/github.com/mdupont/forge-dasl-*.git/`
- Each repo has: flake.nix, Cargo.toml, src/main.rs, Cargo.lock
- cargo-vendormod vendoring init succeeds on dasl-testing
- All 9 repos consumed as forgecode flake inputs from git+file:// URLs
- nix flake check --no-build passes on forgecode with all 9 inputs
- nix eval for each package returns a valid derivation name
- Each server registered in .mcp.json
- MCP tools/list returns correct tool names for each server
- Orchestrator dasl_status shows all 8 servers as ready
- .forge/skills/dasl-testing-mcp/ documents all 9 servers

## Potential Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Template drift across 9 repos | scaffold-dasl-mcp-crate.sh enforces a consistent structure on every run |
| Nix eval time increases as new inputs are added | Use --refresh only when inputs change; lock commits are purely additive |
| Git mirror goes stale when dasl-testing changes | Pipelight pipeline auto-updates the mirror on commit |
| Port conflicts when multiple servers run locally | Register all ports in ~/05/30/ports.txt and check ss -tlnp before startup |
| cargo-vendormod has a learning curve | Document its subcommands in ~/DOCS/cargo-vendormod.md referencing the known cargo-vendormod skill |
| Orphaned subprocesses if an MCP server crashes | Track PIDs in shared state and scan for orphans on startup |

## Alternative Approaches

| Approach | Pros | Cons |
|----------|------|------|
| **Single monolith crate** with all 8 controllers | Easier to maintain, one flake input, one binary | Violates one-decl-per-flake principle, harder to version independently |
| **Python MCP servers** using FastMCP | Faster to write, no Rust compile time | Inconsistent with the Rust stack, no nix build, harder to integrate with forgecode |
| **Inline forgecode crates/** | No git mirrors needed, everything in one repo | Violates principle 3 (no inline code), clutters forgecode |
| **system-manager for all 9 servers** | Declarative, auto-restart, resource limits | Overkill for stdio-only servers that run on-demand through .mcp.json |
