#!/bin/bash
# Split all declarations in a crate into individual files
# Usage: split_crate.sh <crate_dir>

set -e

CRATE_DIR="$1"

if [ -z "$CRATE_DIR" ]; then
    echo "Usage: $0 <crate_dir>"
    exit 1
fi

if [ ! -d "$CRATE_DIR" ]; then
    echo "Error: Crate directory '$CRATE_DIR' not found"
    exit 1
fi

DECLS_DIR="$CRATE_DIR/src/decls"
mkdir -p "$DECLS_DIR"

echo "Splitting declarations in: $CRATE_DIR"

find "$CRATE_DIR/src" -name "*.rs" \
    -not -path "*/decls/*" \
    -not -name "mod.rs" \
    -not -name "lib.rs" \
    | sort | while read -r f; do

    FILE_STEM=$(basename "$f" .rs)
    TARGET="$DECLS_DIR/$FILE_STEM"

    # Skip if already split
    if [ -d "$TARGET" ] && [ -n "$(ls -A "$TARGET"/*.rs 2>/dev/null)" ]; then
        echo "  Already split: $f"
        continue
    fi

    echo "  Splitting: $f"
    cargo run --package decl-splitter --bin decl-splitter -- \
        --input "$f" \
        --output "$TARGET" \
        2>&1 | tail -1
done

echo "Done! Created $(find "$DECLS_DIR" -name '*.rs' -not -name '_decl_module_invocation.rs' 2>/dev/null | wc -l) decls in: $DECLS_DIR"
