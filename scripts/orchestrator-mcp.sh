#!/usr/bin/env bash
# orchestrator-mcp.sh — Send MCP tool calls to forge-tmux-agent-orchestrator-mcp
#
# Usage:
#   ./scripts/orchestrator-mcp.sh list_agents
#   ./scripts/orchestrator-mcp.sh spawn_agent '{"label":"scan-1","command":"cargo check"}'
#   ./scripts/orchestrator-mcp.sh capture_agent_output '{"session":"scan-1","lines":20}'
#   ./scripts/orchestrator-mcp.sh kill_agent '{"session":"scan-1"}'

set -euo pipefail

BIN="${BIN:-./target/debug/forge-tmux-agent-orchestrator-mcp}"
TOOL="${1:?usage: $0 <tool-name> [arguments-json]}"
ARGS="${2:-{}}"

# MCP conversation: init → initialized → tools/call
mcp_tool_call() {
    local tool="$1"
    local args="$2"

    {
        # id=1: initialize
        printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"orchestrator-cli","version":"1.0"}}}\n'
        # id=2: initialized notification
        printf '{"jsonrpc":"2.0","id":2,"method":"notifications/initialized"}\n'
        # id=3: tool call
        printf '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"%s","arguments":%s}}\n' "$tool" "$args"
    } | "$BIN" 2>/dev/null
}

# Parse MCP response and extract tool result text
parse_result() {
    python3 -c "
import sys, json
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        msg = json.loads(line)
    except json.JSONDecodeError:
        continue
    if 'result' in msg:
        content = msg['result'].get('content', [])
        for c in content:
            if c.get('type') == 'text':
                text = c['text']
                # Try to pretty-print JSON
                try:
                    parsed = json.loads(text)
                    print(json.dumps(parsed, indent=2))
                except (json.JSONDecodeError, ValueError):
                    print(text)
    elif 'error' in msg:
        print(f'ERROR (code {msg[\"error\"].get(\"code\",\"?\")}): {msg[\"error\"].get(\"message\",\"\")}')
        data = msg['error'].get('data', '')
        if data:
            print(f'  Details: {data}')
    else:
        print(json.dumps(msg, indent=2))
"
}

mcp_tool_call "$TOOL" "$ARGS" | parse_result
