{
  description = "DASL GOAP task — auto-generated pipeline flake template";
  inputs = {
    nixpkgs.url = "git+file:///mnt/data1/git/github.com/NixOS/nixpkgs.git";
    flake-utils.url = "git+file:///mnt/data1/git/github.com/numtide/flake-utils.git";
    n0x-pi.url = "git+file:///mnt/data1/git/github.com/sub0xdai/n0x-pi.git?ref=master";
  };
  outputs = { self, nixpkgs, flake-utils, n0x-pi }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in {
        devShells.default = pkgs.mkShell {
          name = "dasl-goap-task";
          buildInputs = [
            pkgs.python3
            pkgs.lean4
            pkgs.cargo
            pkgs.jq
            pkgs.curl
            n0x-pi.packages.${system}.pi
          ];
          shellHook = ''
            export PATH="$HOME/.cargo/bin:$HOME/bin:$HOME/dasl-planning/plan-mappings/goap/target/release:$PATH"
            # Deepseek auth for pi agent
            if [ -f "$HOME/.deepseek/env.sh" ]; then
              source "$HOME/.deepseek/env.sh"
            fi
            echo "DASL GOAP pipeline ready — 13 steps to all_pipeline_done"
            echo "First: annotate_spec (85 types, 1000+ proofs)"
          '';
        };
      });
}
