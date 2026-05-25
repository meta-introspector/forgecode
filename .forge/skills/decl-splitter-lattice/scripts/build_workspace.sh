#!/bin/bash
# Build and validate the generated workspace
# Generates workspace Cargo.toml automatically if missing

set -e

WORKSPACE_DIR="$1"

if [ -z "$WORKSPACE_DIR" ]; then
    echo "Usage: $0 <workspace_dir>"
    exit 1
fi

if [ ! -d "$WORKSPACE_DIR" ]; then
    echo "Error: Workspace directory '$WORKSPACE_DIR' not found"
    exit 1
fi

cd "$WORKSPACE_DIR"

# Auto-generate workspace Cargo.toml if it doesn't exist
if [ ! -f "Cargo.toml" ]; then
    echo "Generating workspace Cargo.toml..."
    members=""
    for d in $(ls -d forge_*/ 2>/dev/null | sort); do
        members="${members}  \"${d%/}\",
"
    done
    cat > Cargo.toml << EOF
[workspace]
resolver = "2"
members = [
$members]

[workspace.package]
version = "0.1.0"
edition = "2021"
EOF
    echo "  $(ls -d forge_*/ 2>/dev/null | wc -l) workspace members added"
fi

echo "Building workspace: $WORKSPACE_DIR"
cargo check --workspace 2>&1 | tee build_output.txt

echo ""
echo "Build complete. Check build_output.txt for errors."
