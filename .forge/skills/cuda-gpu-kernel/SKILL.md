---
name: cuda-gpu-kernel
description: >-
  CUDA GPU kernel for Monster lattice queries on the RTX 3080 Ti (12GB).
  Compiles monster_kernel.cu via Nix cudaPackages (nvcc + cudart), runs
  VOA distribution + Morse compression on 196,883 cells from /dev/shm/monster_lattice.
  Pure Nix build: NIXPKGS_ALLOW_UNFREE=1 nix build --impure .#monster-kernel.
  Use when: running GPU lattice queries, compiling CUDA kernels with Nix,
  GPU-accelerating Monster group computations, or benchmarking CUDA on NixOS.
---

# cuda-gpu-kernel — Nix-Built CUDA Monster Lattice

**Source:** `loop-optimization/monster_kernel.cu`  
**Nix:** `zos-gpu-loader/flake.nix#monster-kernel`  
**GPU:** RTX 3080 Ti (12GB), CUDA 13.2 drivers

## Quick Start

```bash
# Build with Nix (downloads CUDA packages once, then cached)
NIXPKGS_ALLOW_UNFREE=1 nix build --impure /path/to/zos-gpu-loader#monster-kernel

# Run
./result/bin/monster_kernel

# Or: GPU dev shell (nvcc in PATH, OpenCL available)
NIXPKGS_ALLOW_UNFREE=1 nix develop /path/to/zos-gpu-loader#gpu
nvcc monster_kernel.cu -o monster_kernel
./monster_kernel
```

## What It Does

```
1. fopen("/dev/shm/monster_lattice")     — read 3.15 MB lattice
2. cudaMalloc + cudaMemcpy               — upload 196,883 cells to GPU
3. monster_query<<<blocks,threads>>>     — VOA distribution (atomicAdd)
4. monster_compress<<<blocks,threads>>>  — Morse encoding (∇∂∫∮⊕⊗⊙⊚)
5. cudaMemcpy back                       — download results
6. printf results                        — VOA counts
```

## CUDA Kernels

### VOA Distribution
```cuda
__global__ void monster_query(MonsterCell* lattice, int* results, int query_type) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= LATTICE_SIZE) return;
    MonsterCell cell = lattice[idx];
    if (query_type == 0) atomicAdd(&results[cell.voa_type], 1);
    else if (query_type == 1) atomicAdd(&results[cell.voa_type], cell.size);
}
```

### Morse Compression
```cuda
__global__ void monster_compress(MonsterCell* lattice, unsigned char* morse_codes) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= LATTICE_SIZE) return;
    MonsterCell cell = lattice[idx];
    morse_codes[idx] = (cell.x + cell.y + cell.z + cell.voa_type) % 8;
}
```

## Struct Layout (must match Rust)

```c
struct __attribute__((packed)) MonsterCell {
    unsigned char x, y, z, voa_type;  // 4 bytes
    unsigned long long inode;          // 8 bytes (u64)
    unsigned int size;                 // 4 bytes
};                                     // = 16 bytes total
```

Matches Rust: `#[repr(C, packed)] pub struct MonsterCell { x: u8, y: u8, z: u8, voa_type: u8, inode: u64, size: u32 }`

## Verification

Compare GPU output against CPU reference:
```bash
# GPU result
nix build --impure .#monster-kernel && ./result/bin/monster_kernel

# CPU reference (should match)
gpu-shmem-query -q voa-dist
```

Expected: VOA distribution matches between GPU and CPU paths.

## Nix Build Details

- Uses `nixpkgs.config.cudaSupport = true` + `allowUnfree = true`
- Downloads `cuda_nvcc` (12.9.86), `cuda_cudart` (12.9.79), `cuda_cccl` (12.9.27)
- Uses `cudaPackages.backendStdenv` for GCC compatibility
- Hermetic: no system CUDA required in PATH
- Output: 1MB statically-linked binary at `result/bin/monster_kernel`

## Related

- **gpu-shmem-query** — Monster lattice CPU queries + OpenCL GPU
- **fractran-gpu-query** — FRACTRAN batch GPU evaluation
- **zos-gpu-loader** — 8 FFI bridges including CUDA plugin
