---
name: nix-background-build
description: Automatically triggers or monitors background Nix builds using Pipelight, Hydra, or a nohup fallback. Use when evaluating flakes, running heavy compilations, triggering CI pipelines, or any task where `nix build` would block the terminal session.
---

# Background Nix Build

Instead of running synchronous blocking commands like `nix build .#package`, use this skill for detached execution. Three strategies are available in descending preference: Pipelight, Hydra, nohup fallback.

## 1. Pipelight Integration (Preferred)

[Pipelight](https://pipelight.dev/) is a ultra-lightweight Rust-based orchestration tool for backgrounding tasks.

### Setup Check

Look for a `pipelight.ts` or `pipelight.js` configuration in the project root. If missing, create one with this baseline structure:

```typescript
// pipelight.ts
import { pipeline, step } from "https://deno.land";

const build = pipeline("nix-bg-build", {
  triggers: [{ branches: ["main", "dev"], actions: ["pre-push", "manual"] }]
});

build.step(
  step("Evaluate and Build Flake", [
    "nix build .#default --print-build-logs --no-link"
  ])
);

export default [build];
```

### Execution

1. **Trigger**: Run `pipelight run nix-bg-build` to launch the build in a detached daemon state.
2. **Monitor**: Check status with `pipelight logs` or stream full output with `pipelight logs -vvvv` without blocking the primary workflow.

## 2. Hydra Integration (Server-Side)

If a remote or local Hydra instance is available for continuous integration and binary caching (Attic, Cachix):

1. **Force Evaluation**: Trigger a declarative project evaluation:
   ```bash
   curl -X POST -H "Content-Type: application/json" \
     -d '{}' http://<hydra-url>/jobset/<project>/<jobset>/eval
   ```
2. **Check Status**: Read build status via:
   ```bash
   curl http://<hydra-url>/api/latesteval?project=<project>&jobset=<jobset>
   ```
   Notify the user when evaluation finishes or fails.

## 3. Fallback: Pure Nix (Async Disown)

If neither Pipelight nor Hydra are configured:

```bash
nohup nix build .#default \
  --profile ./outputs/latest-bg-build \
  --print-build-logs > .nix-bg-build.log 2>&1 &
echo $! > .nix-bg-build.pid
```

- Inform the user of the PID (`.nix-bg-build.pid`) and log path (`.nix-bg-build.log`).
- When prompted for status, check `ps -p $(cat .nix-bg-build.pid)`.
