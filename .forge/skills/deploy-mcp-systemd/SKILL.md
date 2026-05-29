---
name: deploy-mcp-systemd
description: Deploy Model Context Protocol (MCP) servers as declarative systemd services using numtide/system-manager. Use when users need to (1) run any MCP server as a systemd service, (2) add MCP servers to a Nix flake with system-manager, (3) configure token/auth injection for MCP servers in systemd context, (4) deploy HTTP/SSE-transport MCP servers as daemons, or (5) manage multiple MCP servers (GitHub, Forgejo, PostgreSQL, etc.) from a single flake configuration.
---

# Deploy MCP Server via System-Manager

## Architecture

```
flake.nix
  └─ inputs.system-manager (github:numtide/system-manager)
  └─ outputs.systemConfigs.default
       └─ modules/system.nix
            └─ systemd.services.<mcp-server-name>
                 └─ ExecStart via pkgs.writeShellScript wrapper
```

MCP servers use **HTTP/SSE transport** (not stdio) when run as systemd services, because systemd runs in the background and stdio is not available.

## Quick Start

### 1. Add system-manager to flake inputs

```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    system-manager = {
      url = "github:numtide/system-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ nixpkgs, system-manager, ... }: let
    forAllSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ];
  in {
    systemConfigs.default = system-manager.lib.makeSystemConfig {
      modules = [ ./modules/system.nix ];
    };
  };
}
```

### 2. Create module with service definition

```nix
# modules/system.nix
{ config, lib, pkgs, ... }:

let
  # Wrapper script for token injection (keeps secrets out of nix store)
  mcp-wrapper = pkgs.writeShellScript "mcp-server-wrapper" ''
    set -euo pipefail
    # Inject token from CLI auth if not already set
    if [ -z "''${MY_TOKEN_VAR:-}" ]; then
      if command -v ${pkgs.gh}/bin/gh &>/dev/null; then
        TOKEN="$(${pkgs.gh}/bin/gh some token command 2>/dev/null)" || {
          echo "ERROR: token retrieval failed" >&2
          exit 1
        }
        export MY_TOKEN_VAR="$TOKEN"
      else
        echo "ERROR: token not set and helper CLI not found" >&2
        exit 1
      fi
    fi
    exec ${pkgs.some-mcp-server}/bin/some-mcp-server http "$@"
  '';
in {
  nixpkgs.hostPlatform = "x86_64-linux";

  systemd.services.my-mcp-server = {
    enable = true;
    description = "My MCP Server (HTTP/SSE)";
    documentation = [ "https://github.com/owner/repo" ];
    after = [ "network.target" ];
    wantedBy = [ "multi-user.target" ];

    serviceConfig = {
      Type = "simple";
      ExecStart = "${mcp-wrapper} http --port 8082 --toolsets default";
      Restart = "always";
      RestartSec = "5s";
      TimeoutStopSec = "10s";
      MemoryMax = "512M";
      CPUQuota = "50%";
      User = "mdupont";  # needed for gh auth token access
    };
  };
}
```

### 3. Deploy

```bash
sudo nix run 'github:numtide/system-manager' -- switch --flake '.#'
```

## Token/Auth Injection Pattern

Secrets must not be in the nix store. Use `pkgs.writeShellScript` wrappers:

1. Check if env var is already set
2. Fall back to CLI auth (e.g., `gh auth token`)
3. Export the token before exec'ing the server

The wrapper is built by Nix but contains no secrets — only the retrieval logic.

## Scaffolding Future MCP Services

When you want to add a new MCP server but the binary isn't ready yet, use **disabled scaffold** services in the same module:

```nix
# systemd.services.future-mcp-server = {
#   enable = false;
#   description = "Future MCP Server";
#   after = [ "network.target" ];
#   wantedBy = [ "multi-user.target" ];
#   serviceConfig = {
#     Type = "simple";
#     ExecStart = "...";  # TODO: add binary path once available
#     Restart = "always";
#     RestartSec = "5s";
#     MemoryMax = "256M";
#   };
# };
```

Uncomment + fill `ExecStart` when the binary is ready, then redeploy.

## Verifying

```bash
# Check service status
sudo systemctl status mcp-server-name.service

# Check logs
sudo journalctl -u mcp-server-name.service --no-pager -n 30

# Test MCP tools/list over HTTP/SSE
TOKEN=$(gh auth token)
curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"verify","version":"1.0"}}}' \
  http://127.0.0.1:PORT/

curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  http://127.0.0.1:PORT/ | grep -oP '"name":"[^"]+"'
```

## Testing from stdio (before deploying as systemd)

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}\n{"jsonrpc":"2.0","id":2,"method":"notifications/initialized"}\n{"jsonrpc":"2.0","id":3,"method":"tools/list"}\n' | ENV_VAR=$(auth-command) timeout 10 /path/to/mcp-server stdio 2>/dev/null
```

## Common Gotchas

- **system-manager user config**: Passwords must use `users.users.<name>.hashedPassword` or `users.users.<name>.passwordFile`, not plaintext `users.users.<name>.password` (which triggers a failed assertion).
- **`nixpkgs.hostPlatform`**: Must be set in the system module or you'll get "option `nixpkgs.hostPlatform' was accessed but has no value defined".
- **Token scope**: For github-mcp-server, the `repo` scope is required for private repos.
- **Service user**: Run as the actual user (not `nobody`) when the wrapper depends on user-specific CLI auth (e.g., `gh`).
