---
name: zos-nix-nora-plugin
description: >-
  Build, manifest, and load ZOS Forge plugins as standalone Nix flakes with
  /nix/store shared objects and NORA package identities.
---

# zos-nix-nora-plugin — ZOS Plugin Nix/NORA Workflow

Use this skill when working with the ZOS plugin bridge, `~/zos-server/plugins`, or Forge plugin loading.

## Core model

The default ZOS plugin set is no longer built around loose `.so` files in the plugin directory. Each source-backed plugin should be represented by a git+file flake input, a NORA identity, and a concrete /nix/store shared object.

1. a standalone `flake.nix`;
2. a `Cargo.toml`/`Cargo.lock` when it is a Rust plugin;
3. a `/nix/store` shared object output;
4. a root ZOS flake input so the system package depends on the plugin flakes;
5. a `nora` identity in `~/zos-server/plugins/plugins.toml`;
6. a `plugins.toml` entry that Forge loads by default.

## Important constraints

- Do not commit plugin `vendor/` directories.
- Do not rely on loose `.so` files in `~/zos-server/plugins`.
- Prefer `cargoDeps = pkgs.rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };` in plugin flakes.
- Do not add `extraRegistries` for the default crates.io source unless the lockfile uses a non-default registry source.
- The running NORA service has active cargo registry endpoints, but raw artifact upload is documented as disabled. Treat `nora = "name"` as the package identity until the upload/publish workflow is explicitly enabled.
- Source-less legacy binaries are not part of the default set unless they are rebuilt from source-backed flakes.

## Standard flake pattern

A Rust plugin flake should usually contain:

- a pinned `nixpkgs` input;
- `pkgs.rustPlatform.buildRustPackage`;
- `src = ./.`;
- `cargoDeps = pkgs.rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };`;
- `CARGO_BUILD_TARGET = "x86_64-unknown-linux-gnu";`;
- a `zig-cc` wrapper targeting `x86_64-linux-gnu.2.36` when glibc compatibility is required;
- `postInstall` glibc symbol check using `readelf -V`.

## Root flake input model

`~/zos-server/flake.nix` declares the source-backed plugin flakes as inputs and exposes `zos-plugin-default-set` plus a default `zos-server-with-plugins` package. This makes the system package depend on the plugin flakes before the server package is built.

## Default plugins

The current default manifest covers:

- `generators`
- `git-tools`
- `tokenizer`
- `repo-headers`
- `zombie-driver`
- `monster-hecke`
- `observer-qa`
- `parquet-index`
- `shard-tools`

## Rebuild all default plugins

```bash
for d in generators git-tools tokenizer repo-headers zombie-driver monster-hecke observer-qa parquet-index shard-tools; do
  nix build --no-link --print-out-paths "$HOME/zos-server/plugins/$d"
done
```

## Regenerate `plugins.toml`

```bash
python3 - <<'PY'
import pathlib, subprocess
home = pathlib.Path.home()
names = ['generators','git-tools','tokenizer','repo-headers','zombie-driver','monster-hecke','observer-qa','parquet-index','shard-tools']
lines = [
    '# Default ZOS plugin set.\n',
    '#\n',
    '# Each plugin is represented by:\n',
    '# - flake: git+file flake identity that builds the plugin .so in /nix/store\n',
    '# - nora: NORA package identity used for artifact publication\n',
    '# - store_path: concrete /nix/store output directory loaded by Forge\n',
    '# - shared_object: concrete /nix/store .so loaded by Forge\n',
    '#\n',
    '# The loader uses this manifest as the default input set. Additional plugins can\n',
    '# be discovered later, but only Nix-store outputs are loaded by default.\n',
    '\n'
]
for name in names:
    flake_dir = home/'zos-server/plugins'/name
    out = subprocess.check_output(['nix','build','--no-link','--print-out-paths',str(flake_dir)], text=True).strip()
    so = subprocess.check_output(['bash','-lc',f'find -L {subprocess.list2cmdline([out])} -type f -name \'*.so\' | sort | head -1'], text=True).strip()
    lines += [
        '[[plugins]]\n',
        f'name = "{name}"\n',
        f'flake = "git+file:./plugins/{name}"\n',
        f'nora = "{name}"\n',
        f'store_path = "{out}"\n',
        f'shared_object = "{so}"\n',
        '\n'
    ]
(home/'zos-server/plugins/plugins.toml').write_text(''.join(lines))
PY
```

## Verify manifest loading

```bash
python3 - <<'PY'
import ctypes, pathlib
manifest = pathlib.Path.home()/'zos-server/plugins/plugins.toml'
rows=[]; current=None
for line in manifest.read_text().splitlines():
    line=line.strip()
    if line.startswith('[['):
        if current: rows.append(current)
        current={}
    elif current is not None and '=' in line:
        k,v=line.split('=',1); current[k.strip()]=v.strip().strip('"')
if current: rows.append(current)
for row in rows:
    so = pathlib.Path(row['shared_object'])
    lib = ctypes.CDLL(str(so))
    lib.zos_plugin_name.argtypes = []
    lib.zos_plugin_name.restype = ctypes.c_char_p
    print(row['name'], so.exists(), lib.zos_plugin_name().decode())
PY
```

## Forge loader behavior

Forge reads `~/zos-server/plugins/plugins.toml` first. Each manifest entry must resolve to `/nix/store` paths, and `shared_object` must be inside `store_path`. The loader then verifies the ABI symbol `zos_plugin_name` exists before loading the plugin.

## Troubleshooting

- If Nix says `cargoVendorDir` is missing, use `cargoDeps = pkgs.rustPlatform.importCargoLock { ... };`.
- If Cargo complains about duplicate crates.io sources, remove `extraRegistries` for the default crates.io registry.
- If Forge does not load a plugin, check that `plugins.toml` has both `store_path` and `shared_object`, and that both paths are under `/nix/store`.
- If a plugin exports no `zos_plugin_name`, it is not a Forge/ZOS plugin and should be skipped.

## Related skills

- `nora-monitor-tile` for NORA endpoint status and cargo registry details.
- `nora-car-shmem` for NORA storage backend details.
- `zombie-driver2` for the Rust compiler/plugin-generation workflow.
