# Pipelight Locate Output Schema

## Default Output Format

```
<prefix> │ <path> │ <name> │ <pipeline> │ <tag> │ <output>
```

## --format Output

```
name=build tag=rust lock=/nix/store/...-source/Cargo.lock
path=/nix/store/...-source/<relative-path>
```

## Example

```
nix │ /mnt/data1/nix/time/2026/05/15/forgecode/scripts/pipelight/compile_and_install │ compile_and_install.sh │ compile │ rust │ /mnt/data1/nix/time/2026/05/15/forgecode/scripts/pipelight/output_pipeline
```

## Usage with pipelight-schema-generator

```bash
# Pipe locate output directly
pipelight locate --recursive | nix run .#pipelight-schema-generator

# Or save paths, then process
pipelight locate --recursive --no-filename > /tmp/paths.txt
cat /tmp/paths.txt | nix run .#pipelight-schema-generator
```
