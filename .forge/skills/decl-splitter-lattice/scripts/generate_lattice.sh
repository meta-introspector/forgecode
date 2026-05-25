#!/bin/bash
# Generate plugin crates from split declarations with excludes and CAR archival

set -e

CRATE_DIR="$1"
OUTPUT_DIR="$2"
EXCLUDE_FILE="$3"
CAR_FILE="$4"

if [ -z "$CRATE_DIR" ] || [ -z "$OUTPUT_DIR" ]; then
    echo "Usage: $0 <crate_dir> <output_dir> [exclude_file] [car_output]"
    echo "  car_output: Path for CAR archive of excluded decls (optional)"
    exit 1
fi

if [ ! -d "$CRATE_DIR" ]; then
    echo "Error: Crate directory '$CRATE_DIR' not found"
    exit 1
fi

echo "Generating plugin crates for: $CRATE_DIR"
echo "Output: $OUTPUT_DIR"
if [ -n "$EXCLUDE_FILE" ]; then
    echo "Excludes: $EXCLUDE_FILE"
fi

# Check if split decls exist
if [ ! -d "$CRATE_DIR/src/decls" ]; then
    echo "Error: No split declarations found. Run split_crate.sh first."
    exit 1
fi

# Run decl-lattice generate with optional exclude and car output
EXCLUDE_ARG=""
if [ -n "$EXCLUDE_FILE" ] && [ -f "$EXCLUDE_FILE" ]; then
    EXCLUDE_ARG="--exclude-file $EXCLUDE_FILE"
fi

CAR_ARG=""
if [ -n "$CAR_FILE" ]; then
    CAR_ARG="--car-output $CAR_FILE"
fi

cd /mnt/data1/time-2026/05-may/15/forgecode
cargo run --package decl-splitter --bin decl-lattice -- \
  generate --crate-dir "$CRATE_DIR" --output "$OUTPUT_DIR" \
  $EXCLUDE_ARG $CAR_ARG

echo "Done! Generated plugin crates in: $OUTPUT_DIR"
if [ -n "$CAR_FILE" ]; then
    echo "CAR archive: $CAR_FILE"
fi
