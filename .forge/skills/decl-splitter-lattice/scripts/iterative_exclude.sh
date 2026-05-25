#!/bin/bash
# Iteratively exclude failing decls until clean build

set -e

WORKSPACE_DIR="$1"
OUTPUT_DIR="${2:-/tmp/excluded_lattice}"
CRATE_DIR="${3:-crates/forge_domain}"
MAX_ITERATIONS="${4:-20}"

if [ -z "$WORKSPACE_DIR" ]; then
    echo "Usage: $0 <workspace_dir> [output_dir] [crate_dir] [max_iterations]"
    exit 1
fi

echo "Iteratively excluding failing decls..."

EXCLUDE_FILE="excludes.txt"
rm -f "$EXCLUDE_FILE"
echo "# Decl excludes for clean build" > "$EXCLUDE_FILE"

for iteration in $(seq 1 $MAX_ITERATIONS); do
    echo ""
    echo "=== Iteration $iteration ==="
    
    # Generate with current excludes
    cargo run --package decl-splitter --bin decl-lattice -- \
      generate --crate-dir "$CRATE_DIR" --output "$OUTPUT_DIR" \
      --exclude-file "$EXCLUDE_FILE" 2>&1 | tail -3
    
    # Build and check
    cd "$OUTPUT_DIR"
    rm -rf target Cargo.lock
    error_count=$(cargo check 2>&1 | grep "^error\[" | wc -l)
    
    if [ "$error_count" -eq 0 ]; then
        echo "SUCCESS: Clean build after $iteration iterations"
        break
    fi
    
    echo "Found $error_count errors"
    
    # Get failing decls
    failing_decls=$(cargo check 2>&1 | grep "error\[E" -A3 | grep "\.rs:" | sed 's/.*--> //' | sed 's/:.*$//' | sed 's|forge_forge_domain_[a-z0-9_]*/src/||' | sed 's/\.rs$//' | sort -u)
    
    # Add to exclude file
    for decl in $failing_decls; do
        if ! grep -q "^${decl}$" "$EXCLUDE_FILE" 2>/dev/null; then
            echo "$decl" >> "$EXCLUDE_FILE"
        fi
    done
    
done

echo "Final excludes: $(grep -v '^#\|^$' "$EXCLUDE_FILE" | sort -u | wc -l)"
