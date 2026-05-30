{
  description = "forge: AI enabled pair programmer for Claude, GPT, O Series, Grok, Deepseek, Gemini and 300+ models";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    pipelight = {
      url = "git+file:///home/mdupont/git/github.com/crocuda/pipelight?ref=master";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    system-manager = {
      url = "github:numtide/system-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
    cargo-vendormod = {
      url = "git+file:///mnt/data1/git/solana.solfunmeme.com/cargo-vendormod?ref=organize-submodules";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    deep-scanner = {
      url = "git+file:///mnt/data1/git/solana.solfunmeme.com/deep_scanner?ref=main";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pipelight-schema-generator = {
      url = "git+file:///mnt/data1/git/solana.solfunmeme.com/moltis.git?ref=feat/nix-build-fix&dir=pipelight-schema-generator";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    tmux-mcp = {
      url = "git+file:///home/mdupont/git/github.com/bnomei/tmux-mcp?ref=forge-integration";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = inputs@{ self, nixpkgs, pipelight, cargo-vendormod, deep-scanner, pipelight-schema-generator, system-manager, flake-utils, tmux-mcp, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      formatter = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        pkgs.nixfmt-rfc-style
      );

      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          lib = pkgs.lib;
          sourceFilter = path: type:
            let
              base = baseNameOf path;
            in !(
              base == ".git" || base == ".."
              || (type == "unknown" && base == "")
              || (type == "symlink" && base == "result")
              || (path == toString ./target || lib.hasPrefix (toString ./target + "/") path)
              || (path == toString ./result || lib.hasPrefix (toString ./result + "/") path)
            );
          src = lib.cleanSourceWith {
            src = ./.;
            filter = sourceFilter;
          };

          vendorDir = builtins.path {
            path = toString ./vendor;
            name = "forge-vendor";
          };
          forge = pkgs.stdenv.mkDerivation {
            pname = "forge";
            version = "0.1.0-dev";
            inherit src;

            nativeBuildInputs = [
              pkgs.cargo
              pkgs.rustc
              pkgs.cmake
              pkgs.nasm
              pkgs.perl
              pkgs.pkg-config
              pkgs.protobuf
            ];

            buildInputs =
              [ pkgs.sqlite ]
              ++ lib.optionals pkgs.stdenv.isLinux [
                pkgs.libxkbcommon
                pkgs.libx11
                pkgs.libxext
                pkgs.libxfixes
                pkgs.libxcb
                pkgs.wayland
              ]
              ++ lib.optionals pkgs.stdenv.isDarwin [
                pkgs.libiconv
                pkgs.apple-sdk
              ];

            PROTOC = "${pkgs.protobuf}/bin/protoc";
            PROTOC_INCLUDE = "${pkgs.protobuf}/include";
            APP_VERSION = "0.1.0-dev";

            dontConfigure = true;
            dontUseCmakeConfigure = true;
            dontUpdateAutotoolsGnuConfigScripts = true;

            buildPhase = ''
              cp -r ${vendorDir} vendor
              chmod -R +w vendor
              cargo build --release --frozen -p forge_main --bin forge
            '';

            installPhase = ''
              mkdir -p $out/bin
              cp target/release/forge $out/bin/
            '';

            doCheck = false;

            meta = {
              description = "forge: AI enabled pair programmer for Claude, GPT, O Series, Grok, Deepseek, Gemini and 300+ models";
              homepage = "https://forgecode.dev";
              license = lib.licenses.mit;
              mainProgram = "forge";
              platforms = lib.platforms.unix;
            };
          };
        in
        {
          default = forge;
          forge = forge;
          github-mcp-server = pkgs.github-mcp-server;
          cargo-vendormod = inputs.cargo-vendormod.packages.${system}.default;
          deep-scanner = inputs.deep-scanner.packages.${system}.default;
          pipelight-schema-generator = inputs.pipelight-schema-generator.packages.${system}.default;
          tmux-mcp-rs = inputs.tmux-mcp.packages.${system}.default;
          forge-pipelight-mcp = pkgs.stdenv.mkDerivation {
            pname = "forge-pipelight-mcp";
            version = "0.1.0-dev";
            inherit src;
            nativeBuildInputs = [
              pkgs.cargo
              pkgs.rustc
              pkgs.pkg-config
            ];
            dontConfigure = true;
            dontUseCmakeConfigure = true;
            dontUpdateAutotoolsGnuConfigScripts = true;

            buildPhase = ''
              cp -r ${vendorDir} vendor
              chmod -R +w vendor
              cargo build --release --frozen -p forge-pipelight-mcp --bin forge-pipelight-mcp
            '';
            installPhase = ''
              mkdir -p $out/bin
              cp target/release/forge-pipelight-mcp $out/bin/
            '';
            doCheck = false;
            meta = {
              description = "MCP server wrapping pipelight CLI for build management";
              license = lib.licenses.mit;
              mainProgram = "forge-pipelight-mcp";
              platforms = lib.platforms.unix;
            };
          };
          forge-nora-mcp = pkgs.stdenv.mkDerivation {
            pname = "forge-nora-mcp";
            version = "0.1.0-dev";
            inherit src;
            nativeBuildInputs = [
              pkgs.cargo
              pkgs.rustc
              pkgs.pkg-config
            ];
            dontConfigure = true;
            dontUseCmakeConfigure = true;
            dontUpdateAutotoolsGnuConfigScripts = true;

            buildPhase = ''
              cp -r ${vendorDir} vendor
              chmod -R +w vendor
              cargo build --release --frozen -p forge-nora-mcp --bin forge-nora-mcp
            '';
            installPhase = ''
              mkdir -p $out/bin
              cp target/release/forge-nora-mcp $out/bin/
            '';
            doCheck = false;
            meta = {
              description = "MCP server wrapping Nora registry HTTP API";
              license = lib.licenses.mit;
              mainProgram = "forge-nora-mcp";
              platforms = lib.platforms.unix;
            };
          };
          forge-parquet-mcp = pkgs.stdenv.mkDerivation {
            pname = "forge-parquet-mcp";
            version = "0.1.0-dev";
            inherit src;
            nativeBuildInputs = [
              pkgs.cargo
              pkgs.rustc
              pkgs.pkg-config
            ];
            dontConfigure = true;
            dontUseCmakeConfigure = true;
            dontUpdateAutotoolsGnuConfigScripts = true;

            buildPhase = ''
              cp -r ${vendorDir} vendor
              chmod -R +w vendor
              cargo build --release --frozen -p forge-parquet-mcp --bin forge-parquet-mcp
            '';
            installPhase = ''
              mkdir -p $out/bin
              cp target/release/forge-parquet-mcp $out/bin/
            '';
            doCheck = false;
            meta = {
              description = "MCP server for git inode scanning and parquet file operations";
              license = lib.licenses.mit;
              mainProgram = "forge-parquet-mcp";
              platforms = lib.platforms.unix;
            };
          };
          forge-tmux-agent-orchestrator-mcp = pkgs.stdenv.mkDerivation {
            pname = "forge-tmux-agent-orchestrator-mcp";
            version = "0.1.0-dev";
            inherit src;
            nativeBuildInputs = [
              pkgs.cargo
              pkgs.rustc
              pkgs.pkg-config
            ];
            dontConfigure = true;
            dontUseCmakeConfigure = true;
            dontUpdateAutotoolsGnuConfigScripts = true;

            buildPhase = ''
              cp -r ${vendorDir} vendor
              chmod -R +w vendor
              cargo build --release --frozen -p forge-tmux-agent-orchestrator-mcp --bin forge-tmux-agent-orchestrator-mcp
            '';
            installPhase = ''
              mkdir -p $out/bin
              cp target/release/forge-tmux-agent-orchestrator-mcp $out/bin/
            '';
            doCheck = false;
            meta = {
              description = "MCP server that orchestrates other agents in tmux sessions — spawn, monitor, communicate with agents";
              license = lib.licenses.mit;
              mainProgram = "forge-tmux-agent-orchestrator-mcp";
              platforms = lib.platforms.unix;
            };
          };
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/forge";
        };
        forge = {
          type = "app";
          program = "${self.packages.${system}.forge}/bin/forge";
        };
        github-mcp-server = {
          type = "app";
          program = "${self.packages.${system}.github-mcp-server}/bin/github-mcp-server";
        };
        cargo-vendormod = {
          type = "app";
          program = "${self.packages.${system}.cargo-vendormod}/bin/cargo-vendormod";
        };
        deep-scanner = {
          type = "app";
          program = "${self.packages.${system}.deep-scanner}/bin/deep_scanner";
        };
        forge-nora-mcp = {
          type = "app";
          program = "${self.packages.${system}.forge-nora-mcp}/bin/forge-nora-mcp";
        };
        forge-parquet-mcp = {
          type = "app";
          program = "${self.packages.${system}.forge-parquet-mcp}/bin/forge-parquet-mcp";
        };
        forge-tmux-agent-orchestrator-mcp = {
          type = "app";
          program = "${self.packages.${system}.forge-tmux-agent-orchestrator-mcp}/bin/forge-tmux-agent-orchestrator-mcp";
        };
        tmux-mcp-rs = {
          type = "app";
          program = "${self.packages.${system}.tmux-mcp-rs}/bin/tmux-mcp-rs";
        };
        pipelight-schema-generator = {
          type = "app";
          program = "${self.packages.${system}.pipelight-schema-generator}/bin/pipelight-schema-generator";
        };
      });

      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          lib = pkgs.lib;
        in
        {
          default = pkgs.mkShell {
            packages =
              [
                pkgs.cargo
                pkgs.cargo-insta
                pkgs.cargo-llvm-cov
                pkgs.clippy
                pkgs.cmake
                pkgs.nasm
                pkgs.perl
                pkgs.pkg-config
                pkgs.protobuf
                pkgs.rust-analyzer
                pkgs.rustc
                pkgs.rustfmt
                pkgs.sqlite
                pipelight.packages.${system}.default
                pkgs.github-mcp-server
                inputs.cargo-vendormod.packages.${system}.default
                inputs.deep-scanner.packages.${system}.default
                self.packages.${system}.forge-nora-mcp
                self.packages.${system}.pipelight-schema-generator
                self.packages.${system}.forge-parquet-mcp
                self.packages.${system}.forge-tmux-agent-orchestrator-mcp
                inputs.tmux-mcp.packages.${system}.default
              ]
              ++ lib.optionals pkgs.stdenv.isLinux [
                pkgs.libxkbcommon
                pkgs.libx11
                pkgs.libxext
                pkgs.libxfixes
                pkgs.libxcb
                pkgs.wayland
              ]
              ++ lib.optionals pkgs.stdenv.isDarwin [
                pkgs.libiconv
                pkgs.darwin.apple_sdk.frameworks.AppKit
                pkgs.darwin.apple_sdk.frameworks.CoreGraphics
                pkgs.darwin.apple_sdk.frameworks.Foundation
              ];

            PROTOC = "${pkgs.protobuf}/bin/protoc";
            PROTOC_INCLUDE = "${pkgs.protobuf}/include";
            APP_VERSION = "0.1.0-dev";
          };
        });

      # ── system-manager configuration ──────────────────────────────────────
      systemConfigs.default = inputs."system-manager".lib.makeSystemConfig {
        modules = [
          ./modules/system.nix
        ];
      };
    };
}
