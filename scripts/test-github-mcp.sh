#!/usr/bin/env bash
# test-github-mcp.sh — smoke-test the github-mcp-server via stdio
#
# Usage:
#   ./test-github-mcp.sh                    # show users toolset
#   ./test-github-mcp.sh --toolset=issues   # show issues toolset
#   ./test-github-mcp.sh --list-toolsets    # list available toolsets
#   ./test-github-mcp.sh --help             # this help
#
# Requires: gh (authenticated), nix (flake available)

set -euo pipefail

SCRIPT_NAME="$(basename "$0")"
SERVER_BIN=""
TOOLSETS="users"
SHOW_TOOLSETS=false

info()  { printf "  [ℹ] %s\n" "$*" >&2; }
ok()    { printf "  [✓] %s\n" "$*" >&2; }
warn()  { printf "  [⚠] %s\n" "$*" >&2; }
fail()  { printf "  [✗] %s\n" "$*" >&2; exit 1; }

# ── Parse arguments ───────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h)
      sed -n '3,11p' "$0"
      exit 0
      ;;
    --toolset=*)
      TOOLSETS="${1#*=}"
      shift
      ;;
    --list-toolsets)
      SHOW_TOOLSETS=true
      shift
      ;;
    *)
      fail "Unknown option: $1 (try --help)"
      ;;
  esac
done

# ── Prerequisites ─────────────────────────────────────────────────────────

command -v gh >/dev/null 2>&1 || fail "gh CLI not found — install it first"
command -v nix >/dev/null 2>&1 || fail "nix not found — install it first"

if ! gh auth status 2>/dev/null; then
  fail "gh CLI is not authenticated — run 'gh auth login' first"
fi

# ── Locate the server binary ──────────────────────────────────────────────

info "Locating github-mcp-server via nix..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NIX_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Try flake first, then nixpkgs
if SERVER_BIN=$(nix build "$NIX_DIR#github-mcp-server" --no-link --print-out-paths 2>/dev/null); then
  SERVER_BIN="$SERVER_BIN/bin/github-mcp-server"
elif SERVER_BIN=$(nix build nixpkgs#github-mcp-server --no-link --print-out-paths 2>/dev/null); then
  SERVER_BIN="$SERVER_BIN/bin/github-mcp-server"
else
  fail "github-mcp-server not found in flake or nixpkgs"
fi

ok "Found: $SERVER_BIN"

# ── List toolsets mode ────────────────────────────────────────────────────

if $SHOW_TOOLSETS; then
  info "Available toolsets (pass via --toolset=...):"
  "$SERVER_BIN" stdio --help 2>&1 | \
    sed -n '/Toolsets:/,/^$/p' | \
    sed 's/^/  /'
  exit 0
fi

# ── Get GitHub token ──────────────────────────────────────────────────────

TOKEN=$(gh auth token 2>/dev/null)
if [[ -z "$TOKEN" ]]; then
  fail "Failed to get GitHub token from gh CLI"
fi
info "Token acquired (${#TOKEN} chars)"

# ── Build MCP request payload ─────────────────────────────────────────────

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

INPUT_FILE="$WORKDIR/input.jsonl"

cat > "$INPUT_FILE" <<-REQUEST_EOF
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-${SCRIPT_NAME}","version":"1.0"}}}
{"jsonrpc":"2.0","id":2,"method":"notifications/initialized"}
{"jsonrpc":"2.0","id":3,"method":"tools/list"}
REQUEST_EOF

OUTPUT_FILE="$WORKDIR/output.jsonl"

# ── Run the server (via named pipe) ──────────────────────────────────────

info "Starting github-mcp-server (toolset=$TOOLSETS)..."

FIFO="$WORKDIR/input.fifo"
mkfifo "$FIFO"

# The server reads from the FIFO, we write to it from a background shell.
# Both sides must be open before either side can proceed (blocking I/O).
GITHUB_PERSONAL_ACCESS_TOKEN="$TOKEN" \
  timeout 12 \
  "$SERVER_BIN" stdio --toolsets="$TOOLSETS" \
  < "$FIFO" \
  2>"$WORKDIR/stderr.log" \
  > "$OUTPUT_FILE" &

SERVER_PID=$!

# Write messages with small delays so the server can process
# each one and flush its response before stdin closes.
{
  while IFS= read -r line; do
    printf '%s\n' "$line" > "$FIFO"
    sleep 0.3
  done < "$INPUT_FILE"
  sleep 0.5
} &
WRITER_PID=$!

# Wait for the server to finish (timeout or clean exit)
wait "$SERVER_PID" 2>/dev/null || true
wait "$WRITER_PID" 2>/dev/null || true

# ── Parse and display results ─────────────────────────────────────────────

if [[ ! -s "$OUTPUT_FILE" ]]; then
  warn "No output received. Stderr log:"
  sed 's/^/  | /' "$WORKDIR/stderr.log"
  fail "Server produced no stdout"
fi

# The server may produce initialization responses + tool results
# Extract the tools/list response (id=3)
TOOLS_JSON=$(grep '"id":3' "$OUTPUT_FILE" | head -1 || echo "")

if [[ -z "$TOOLS_JSON" ]]; then
  warn "tools/list response not found. Full output:"
  cat "$OUTPUT_FILE"
  fail "No tools/list result"
fi

PYTHON_OUTPUT=$(echo "$TOOLS_JSON" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    tools = data.get('result', {}).get('tools', [])
    print(len(tools))
    for t in tools:
        name = t.get('name', '?')
        desc = (t.get('description') or '')[:120]
        print(f'{name}  — {desc}')
except Exception as e:
    print(f'0 (parse error: {e})')
" 2>/dev/null || echo "0")

TOOL_COUNT=$(echo "$PYTHON_OUTPUT" | head -1)
TOOL_LIST=$(echo "$PYTHON_OUTPUT" | tail -n +2 || true)

echo ""
ok "github-mcp-server is working!"
echo ""
info "$TOOL_COUNT tools available (toolset=$TOOLSETS):"
while IFS= read -r line; do
  printf '     %s\n' "$line"
done <<< "$TOOL_LIST"
echo ""
info "Stderr log summary (last 5 lines):"
grep -v '^\[' "$WORKDIR/stderr.log" | tail -5 | sed 's/^/  | /'
