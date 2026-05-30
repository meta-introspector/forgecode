#!/usr/bin/env bash
# Test forge-tmux-agent-orchestrator-mcp via MCP stdio protocol.
#
# IMPORTANT: The FIFO write end MUST be held open continuously. Each
# `echo > "$FIFO"` creates a fresh open/close cycle, and when the
# last writer closes, the reader sees EOF — after which FramedRead
# treats the stream as permanently closed.
#
# Usage:
#   ./scripts/test-agent-orchestrator.sh                   # list_agents
#   ./scripts/test-agent-orchestrator.sh spawn_agent '{"label":"my-agent"}'
#   ./scripts/test-agent-orchestrator.sh list_agents
#   ./scripts/test-agent-orchestrator.sh agent_status '{"session":"agent-...")}'
#   ./scripts/test-agent-orchestrator.sh kill_agent '{"session":"agent-..."}'
#   ./scripts/test-agent-orchestrator.sh send_to_agent '{"session":"agent-...","input":"echo hi"}'
#   ./scripts/test-agent-orchestrator.sh read_agent '{"session":"agent-...","lines":20}'

set -euo pipefail

TOOL_NAME="${1:-list_agents}"
TOOL_ARGS="${2:-{}}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ORCH_BIN="${3:-$PROJECT_DIR/target/debug/forge-tmux-agent-orchestrator-mcp}"

if [ ! -f "$ORCH_BIN" ] && [ ! -x "$ORCH_BIN" ]; then
    echo "Building forge-tmux-agent-orchestrator-mcp..." >&2
    cd "$PROJECT_DIR"
    cargo build -p forge-tmux-agent-orchestrator-mcp 2>&1
    ORCH_BIN="target/debug/forge-tmux-agent-orchestrator-mcp"
fi

echo "=== Orchestrator MCP: $TOOL_NAME ===" >&2

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT
FIFO="$TMPDIR/mcp_in"
mkfifo "$FIFO"

# Start server with FIFO as stdin
"$ORCH_BIN" < "$FIFO" > "$TMPDIR/server_out" 2>&1 &
SERVER_PID=$!

# CRITICAL: Hold the FIFO write end open on fd 3. Without this, each
# echo opens/closes the pipe, and the reader sees EOF permanently.
exec 3>"$FIFO"

sleep 0.3

# 1. Initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' >&3
sleep 0.3

# 2. Initialized notification
echo '{"jsonrpc":"2.0","id":2,"method":"notifications/initialized"}' >&3
sleep 0.3

# 3. Tool call
echo "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"$TOOL_NAME\",\"arguments\":$TOOL_ARGS}}" >&3
sleep 1

# Close the FIFO write end (sends EOF to server)
exec 3>&-

wait "$SERVER_PID" 2>/dev/null || true

# Show output
cat "$TMPDIR/server_out"
