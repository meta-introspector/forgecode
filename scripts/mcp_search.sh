#!/usr/bin/env bash
# mcp_search — search GitHub repos via the running github-mcp-server HTTP endpoint
set -euo pipefail

MCPSRV="http://127.0.0.1:8082"
TOKEN=$(gh auth token)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FORMATTER="$SCRIPT_DIR/format_mcp_results.py"

search() {
  local query="$1"
  local label="$2"

  echo "═══════════════════════════════════════════════════════════════"
  echo "  $label"
  echo "  Query: $query"
  echo "═══════════════════════════════════════════════════════════════"

  # Initialize MCP session
  curl -s -X POST "$MCPSRV" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"mcp-search","version":"1.0"}}}' \
    > /dev/null 2>&1

  # Send initialized notification
  curl -s -X POST "$MCPSRV" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{"jsonrpc":"2.0","id":2,"method":"notifications/initialized"}' \
    > /dev/null 2>&1

  # Tool call
  ARGS_JSON=$(python3 -c "import json; print(json.dumps({'query': '$query', 'limit': 10}))")
  curl -s -X POST "$MCPSRV" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"search_repositories\",\"arguments\":$ARGS_JSON}}" \
    2>/dev/null | grep '^data: ' | sed 's/^data: //' \
    | python3 -c "
import sys, json
for line in sys.stdin:
    try:
        d = json.loads(line.strip())
        if 'result' in d:
            for c in d['result'].get('content', []):
                if c.get('type') == 'text':
                    print(c['text'])
    except:
        pass
" | python3 "$FORMATTER"

  echo ""
}

search "mcp-server forgejo" "Forgejo MCP Servers"
search "mcp-server postgresql" "PostgreSQL MCP Servers"
search "mcp-server postgres" "PostgreSQL MCP Servers (alt)"
search "mcp-server parquet" "Parquet MCP Servers"
search "mcp-server dag-cbor OR ipld" "DAG-CBOR / IPLD MCP Servers"
