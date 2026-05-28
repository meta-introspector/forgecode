# Task List: github-mcp-server systemd Wrapper

## Goal

Run `github-mcp-server` as a systemd user service on Ubuntu + Nix, listening on HTTP
port 8082 so Forge can connect via SSE transport.

## Existing Artifacts

| File | Purpose |
|------|---------|
| `~/.local/bin/github-mcp-server-wrapper` | Shell wrapper that injects token + starts server |
| `~/.config/systemd/user/github-mcp-server.service` | systemd user unit definition |

## Current Issues

1. **PATH not set** — systemd user services don't inherit the user's shell PATH, so
   `/nix/var/nix/profiles/default/bin/nix` needs to be on `PATH` (or used absolute).
   Also nix itself needs its own sub-tools (like `nix build` helpers) on PATH.
2. **`gh` missing from systemd PATH** — `/usr/bin/gh` is already absolute, but
   `command -v` may fail without a proper PATH.
3. **`nix run` builds every time** — `nix run` may trigger a rebuild on each restart.
   Should pin to the store path instead.
4. **No graceful shutdown** — No SIGTERM handler; systemd sends SIGTERM but
   `nix run` as a subprocess relay might not forward it correctly.
5. **No health check** — Service reports "started" immediately but the server
   may not be listening yet.
6. **No log rotation** — journald handles this by default, but worth noting.
7. **Only `x86_64-linux`** — Only tested on this arch; multi-arch not handled.

## Task Checklist

### 1. Fix PATH in Wrapper

- [ ] Add `/nix/var/nix/profiles/default/bin` to `PATH` so `nix` can find its
      own sub-tools.
- [ ] Ensure `/usr/bin` (for `gh`) is on `PATH`.

### 2. Pin Store Path Instead of `nix run`

- [ ] Replace `nix run "$FLAKE#github-mcp-server"` with the actual store path:
      `/nix/store/hn58zig7ssgixn83ilawzijbvgg26m8m-github-mcp-server-1.0.4/bin/github-mcp-server`
- [ ] This avoids Nix evaluation on every restart and removes the PATH
      dependency on `nix`.
- [ ] **Trade-off**: The store path changes when nixpkgs is updated.
      Update strategy needed (see below).

**Update Strategy Options**:
- a) Pin to the flake output and re-pin after `nix flake update`
- b) Use a symlink managed by a nix build hook
- c) Accept manual updates when the package version changes

### 3. Signal Handling / Graceful Shutdown

- [ ] The `github-mcp-server` Go binary handles SIGTERM natively, so no wrapper
      logic needed for signal forwarding if we exec it directly.
- [ ] If using `nix run`, it spawns as a child process and systemd may not
      deliver signals correctly. **Prefer direct exec of the store path.**
- [ ] Add `TimeoutStopSec=10` to the service file (already set).

### 4. Health Check (Post-Start Verification)

- [ ] After starting, verify `http://127.0.0.1:8082/health` or similar endpoint
      responds before marking the service as healthy.
- [ ] Could use `ExecStartPost=` with a `curl --retry` loop.

### 5. Linger / Boot Survival

- [x] Already enabled: `loginctl show-user mdupont | grep Linger` → `Linger=yes`
- [ ] Verify the service starts automatically after reboot:
      `systemctl --user is-enabled github-mcp-server.service`

### 6. Verify End-to-End

- [ ] `systemctl --user restart github-mcp-server.service`
- [ ] `curl http://127.0.0.1:8082/` — expect an MCP response or server info
- [ ] `systemctl --user status github-mcp-server.service` — active (running)
- [ ] Test an MCP tools/list call over HTTP/SSE
