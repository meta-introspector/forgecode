// pipelight.ts
// Baseline Pipelight configuration for background Nix builds.
// Copy this to the project root and customize pipeline names and build steps.
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
