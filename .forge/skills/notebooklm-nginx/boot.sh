#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
echo "=== $(basename $(dirname "$0")) ==="
echo "Launching pi agent in the task dev shell..."
exec nix develop -c pi
