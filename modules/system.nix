{ config, lib, pkgs, ... }:

let
  # Wrapper script that injects GITHUB_PERSONAL_ACCESS_TOKEN from gh CLI
  # before launching the server.  Keeps the token out of the nix store.
  mcp-wrapper = pkgs.writeShellScript "github-mcp-server-wrapper" ''
    set -euo pipefail

    # Obtain token from gh CLI if not already set
    if [ -z "''${GITHUB_PERSONAL_ACCESS_TOKEN:-}" ]; then
      if command -v ${pkgs.gh}/bin/gh &>/dev/null; then
        TOKEN="$(${pkgs.gh}/bin/gh auth token 2>/dev/null)" || {
          echo "ERROR: gh auth token failed. Is 'gh auth login' done?" >&2
          exit 1
        }
        export GITHUB_PERSONAL_ACCESS_TOKEN="$TOKEN"
      else
        echo "ERROR: GITHUB_PERSONAL_ACCESS_TOKEN not set and gh CLI not found" >&2
        exit 1
      fi
    fi

    exec ${pkgs.github-mcp-server}/bin/github-mcp-server http "$@"
  '';
in
{
  environment.systemPackages = with pkgs; [
    github-mcp-server
  ];

  # ── github-mcp-server (HTTP/SSE on port 8082) ─────────────────────────────
  systemd.services.github-mcp-server = {
    enable = true;
    description = "GitHub MCP Server (HTTP/SSE)";
    documentation = [ "https://github.com/github/github-mcp-server" ];
    after = [ "network.target" ];
    wantedBy = [ "multi-user.target" ];

    serviceConfig = {
      Type = "simple";
      ExecStart = "${mcp-wrapper} http --port 8082 --toolsets default,users,issues,pull_requests,repos";
      Restart = "always";
      RestartSec = "5s";
      TimeoutStopSec = "10s";
      MemoryMax = "512M";
      CPUQuota = "50%";
      # Run as the regular user so gh auth token is available
      User = "mdupont";
    };
  };
}
