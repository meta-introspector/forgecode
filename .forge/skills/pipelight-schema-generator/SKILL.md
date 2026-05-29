---
name: pipelight-schema-generator
title: Pipelight Schema Generator
description: Generate JSON-LD, CBOR, DASL, RDFa, and Buildbot Nix configurations from file paths using the pipelight-schema-generator tool
---

# Pipelight Schema Generator

Generate structured schema outputs (JSON-LD, CBOR, DASL, RDFa, Buildbot Nix config) from file paths using `pipelight-schema-generator`.

## Quick Start

```bash
# Run from flake (reads stdin file paths, outputs schemas)
cat paths.txt | nix run .#pipelight-schema-generator

# Or with pipelight locate output --format
pipelight locate --format | nix run .#pipelight-schema-generator
```

## Package Info

- **Flake input**: `pipelight-schema-generator` from `git+file:///mnt/data1/git/solana.solfunmeme.com/moltis.git?dir=pipelight-schema-generator&ref=feat/nix-build-fix`
- **Package name**: `pipelight-schema-generator`
- **App**: `nix run .#pipelight-schema-generator`
- **Nix store**: via flake evaluation (locked in `flake.lock`)

## Usage Examples

### Generate all formats from stdin paths

```bash
echo -e "src/main.rs\nsrc/lib.rs\nCargo.toml" | nix run .#pipelight-schema-generator
# Outputs: JSON-LD, CBOR, DASL, RDFa, Buildbot Nix config
```

### Pipe from pipelight locate

```bash
pipelight locate --recursive | nix run .#pipelight-schema-generator
```

### Save output to files

```bash
pipelight locate --recursive | nix run .#pipelight-schema-generator --output-dir ./schemas/
```

## Output Formats

| Format | Extension | Description |
|--------|-----------|-------------|
| JSON-LD | `.jsonld` | Linked Data JSON |
| CBOR | `.cbor` | Concise Binary Object Representation |
| DASL | `.dasl` | Declarative Application Schema Language |
| RDFa | `.html` | RDFa in HTML |
| Buildbot Nix | `.nix` | Buildbot NixOS configuration |

## Integration with MCP

The schema generator can be exposed as a tool via `forge-pipelight-mcp`:

```
# Schematic flow
pipelight locate --format
  → pipelight-schema-generator
    → JSON-LD / CBOR / DASL / RDFa / Buildbot Nix
      → forge-pipelight-mcp tools (pipelight_* tools)
```

## Related MCP Servers

- **`forge-pipelight-mcp`**: CI/CD build management (register pipelines, check status, view logs)
- Both are available via `.mcp.json` and the devShell in `nix develop`
