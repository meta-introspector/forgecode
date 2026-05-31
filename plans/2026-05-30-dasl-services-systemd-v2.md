# DASL Testing Microservices — system-manager Deployment (v2)

## Objective

Deploy all 11 DASL CBOR testing microservices as declarative systemd units via
numtide/system-manager, following gitplan.org principles: git mirrors, Nix
builds via nora registry, cargo-vendormod for submodule management, and
pipelight orchestration from build → publish → deploy.

## Principles (from ~/05/28/gitplan.org)

1. All things built from git using Nix using nora
2. One declaration per flake
3. `cargo-vendormod` manages vendoring/submodules
4. nora serves as local crate registry, proxying to crates.io
5. Pipelight orchestrates build → publish → deploy
6. `git+file://` mirrors for all repos
7. system-manager for systemd (root-level), user services for user-level

## Implementation Tasks

### Phase 1 — Infrastructure & Mirror Setup

- [ ] **1.1** Push latest dasl-testing commits to git mirror
      `~/git/github.com/hyphacoop/dasl-testing.git/`
      - Destination: `~/git/github.com/hyphacoop/dasl-testing.git`

- [ ] **1.2** Run cargo-vendormod on dasl-testing to inventory all submodule deps
      - `cd ~/dasl/dasl-testing && cargo-vendormod vendoring init`
      - This registers `flakify.nix`, `global_graph/`, and harness subdirectories

- [ ] **1.3** Create git mirrors for each distinct upstream:
      - `~/git/github.com/hyphacoop/dasl-testing.git/` (exists, push latest)
      - `~/git/github.com/hyphacoop/dasl-cbor-roundrobin.git/` (C round-robin)
      - `~/git/github.com/hyphacoop/dasl-gateway.git/` (Python gateway)
      - `~/git/github.com/hyphacoop/dasl-dashboard.git/` (Python dashboard)

- [ ] **1.4** Publish Rust harness crates to nora registry
      - `serde_ipld_dagcbor` → `cargo publish --registry nora -p serde_ipld_dagcbor`
      - `libipld` → `cargo publish --registry nora -p libipld-dagcbor`
      - `n0_dasl` → `cargo publish --registry nora -p n0-dasl`
      - Verify: `curl http://127.0.0.1:4000/cargo/api/v1/crates`

- [ ] **1.5** Write `~/05/30/scaffold-dasl-service.sh` — generates system-manager
      service block for a given binary + port. Must accept: `name`, `binary_path`,
      `port`, `language` (rust/python/go/js). Outputs a Nix attribute set matching
      the system-manager `systemd.services.<name>` format.

- [ ] **1.6** Write `~/05/30/register-dasl-services.py` — reads `services.txt`,
      calls scaffold script for each service, and appends all blocks to
      `modules/system.nix` in the DASL section.

### Phase 2 — Rust Microservices (ports 8001-8003)

Each Rust service exposes an HTTP API testing a DAG-CBOR implementation:

- [ ] **2.1** dasl-serde-ipld-dagcbor (port 8001)
      - Binary: derived from `dasl-testing` flake (harnesses/serde_ipld_dagcbor)
      - system-manager block: `Type=simple`, `ExecStart=<store-path>/bin/server`
      - `After=network.target`, `WantedBy=multi-user.target`

- [ ] **2.2** dasl-libipld (port 8002)
      - Binary: derived from `dasl-testing` flake (harnesses/libipld)
      - Same systemd pattern as 2.1

- [ ] **2.3** dasl-n0-dasl (port 8003)
      - Binary: derived from `dasl-testing` flake (harnesses/n0_dasl)
      - Same systemd pattern as 2.1

### Phase 3 — Python Microservices (ports 8004-8005)

- [ ] **3.1** dasl-dag-cbrrr (port 8004)
      - Python-based DAG-CBOR implementation
      - Use flake's `pythonService` derivation for hermetic runtime
      - systemd: `DynamicUser=true`, `PrivateTmp=true`
      - `ExecStart`: `<store-path>/bin/cbrrr-server`

- [ ] **3.2** dasl-python-libipld (port 8005)
      - Python bindings for libipld
      - Same pattern as 3.1

### Phase 4 — Go & JavaScript Microservices (ports 8006-8007)

- [ ] **4.1** dasl-go-cbor (port 8006)
      - Go DAG-CBOR implementation from `harnesses/go-ipld-cbor/`
      - Option A: Add `buildGoModule` to dasl-testing flake
      - Option B: Wrap pre-built binary with `pkgs.buildFHSEnv`
      - systemd: `ExecStart=<store-path>/bin/go-server`

- [ ] **4.2** dasl-js-cbor (port 8007)
      - JavaScript DAG-CBOR from `harnesses/js-multiformats/`
      - Option A: Nix npm build (`pkgs.buildNpmPackage`)
      - Option B: `NODE_PATH=<store-node_modules>` + `pkgs.nodejs`
      - systemd: `ExecStart=<store-path>/bin/js-server`, `User=nobody`

### Phase 5 — Gateway, Dashboard & Pastebin (ports 8010, 8080, 8090)

- [ ] **5.1** dasl-gateway (port 8010)
      - Python FastAPI gateway proxying to all 7 backend services (8001-8007)
      - `Requires=dasl-serde-ipld-dagcbor dasl-libipld dasl-n0-dasl dasl-dag-cbrrr dasl-python-libipld dasl-go-cbor dasl-js-cbor`
      - `After=` all 7 backends
      - systemd `PartOf=` chain for coordinated restart

- [ ] **5.2** dasl-dashboard (port 8080)
      - Python FastAPI dashboard
      - Static-page frontend + API calling gateway
      - systemd: `ExecStart=<store-path>/bin/dashboard`

- [ ] **5.3** dasl-pastebin (port 8090)
      - Python pastebin service (exists as systemd unit already)
      - Migrate into system-manager module
      - Add tile-rendering support for CBOR/hex output

### Phase 6 — C Round-Robin & Health Checks

- [ ] **6.1** dasl-cbor-roundrobin (7 C binaries)
      - Oneshot execution of 7 C implementations
      - systemd timer: `OnCalendar=hourly`, `RandomizedDelaySec=300`
      - Writes divergence report to `/var/lib/dasl/reports/`
      - `ExecStart`: bash wrapper iterating over 7 binaries

- [ ] **6.2** dasl-healthcheck
      - systemd timer: `OnCalendar=minutely`
      - Checks all 11 endpoints return 200
      - Logs to journald under `unit=dasl-healthcheck.service`
      - On failure: `OnFailure=dasl-recover.service` (auto-restart chain)

### Phase 7 — Cleanup & Verification

- [ ] **7.1** Deploy via system-manager
      - `sudo -E env PATH="$PATH" nix run 'github:numtide/system-manager' -- switch --flake '/mnt/data1/time-2026/05-may/15/forgecode#'`

- [ ] **7.2** Stop and remove old user-level services
      - `systemctl --user stop dasl-*` for each service
      - `systemctl --user disable dasl-*`
      - `rm ~/.config/systemd/user/dasl-*`

- [ ] **7.3** Verify all services via system-manager
      - `sudo systemctl status dasl-* --no-pager`
      - `curl http://127.0.0.1:8001/` (each port)
      - `curl http://127.0.0.1:8010/health` (gateway aggregates)
      - `ss -tlnp | grep -E '800[1-7]|8010|8080|8090'`

- [ ] **7.4** Register all services as MCP tools
      - Add `"dasl-gateway"`, `"dasl-dashboard"` etc. to `.mcp.json`
      - Each MCP server wraps a subset of dasl-testing services

- [ ] **7.5** Add pipelight pipeline for auto-deploy on flake.lock changes
      - Pipelight watches `flake.lock` in forgecode
      - On change: `vendormod update` → `nix flake lock` → system-manager switch

## Verification Criteria

1. `sudo systemctl status dasl-*` shows all 11 services `active (running)`
2. `curl http://127.0.0.1:8010/health` returns `{"status":"ok","services":["serde-ipld-dagcbor:8001","libipld:8002",...]}`
3. Each port 8001-8007 responds to `curl -X POST -d '{"hex":"a16568656c6c6f65776f726c64"}'` with a DAG-CBOR round-trip result
4. `ss -tlnp | grep -cE '800[1-7]|8010|8080|8090'` returns 10 (11 minus round-robin which is oneshot+timer)
5. No stale user-level `dasl-*` services remain
6. `pipelight run` succeeds for build→publish→deploy pipeline
7. Forge `.mcp.json` has entries for gateway and dashboard

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Python venv bustage in nix store | Medium | Use flake `pythonService` wrapper instead of importing .venv |
| Go binary not in flake yet | High | Build with `pkgs.buildGoModule` — add to dasl-testing flake first |
| JS npm deps unreproducible | Medium | Pin `package-lock.json` in mirror, use `buildNpmPackage` |
| Port conflicts with existing services | Medium | Pre-flight `ss -tlnp`, documented port map, stop old services first |
| system-manager build too slow | Low | Use nix-build-daemon; foreground wait only for first deployment |
| Round-robin C binaries fail to build | Low | Already built; verify store paths exist before creating unit |
| pipelight pipeline race with flake.lock | Low | Pipelight monitors file changes, system-manager switches atomically |

## Alternative Approaches

| Approach | Pros | Cons |
|----------|------|------|
| **A: system-manager (chosen)** | Declarative, GC-rooted, Nix-built | Requires sudo, slower initial deploy |
| **B: User services** | No sudo needed | No root-level integration, no dependency management |
| **C: Docker Compose** | Familiar pattern | Adds container overhead, not aligned with gitplan.org |
| **D: Raw systemd units** | Fastest setup | Manual, no GC, no declarative management |
