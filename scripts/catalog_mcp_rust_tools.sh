#!/usr/bin/env bash
# catalog_mcp_rust_tools.sh — reusable search/catalog for MCP servers and Rust libs
set -euo pipefail
trap 'fail "Error at line $LINENO"' ERR

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CATALOG_DIR="${REPO_ROOT}/test/fixtures/tool-catalog"
CATALOG_FILE="${CATALOG_DIR}/forge-tool-catalog.json"
mkdir -p "$CATALOG_DIR"

QUICK_MODE=false
for arg in "$@"; do
  case "$arg" in --quick) QUICK_MODE=true ;; esac
done

# ─── helpers (all go to stderr so results capture is clean) ───
info() { printf "  [ℹ] %s\n" "$*" >&2; }
ok()   { printf "  [✓] %s\n" "$*" >&2; }
warn() { printf "  [⚠] %s\n" "$*" >&2; }
fail() { printf "  [✗] %s\n" "$*" >&2; }

# ─── 1. GitHub MCP servers ──────────────────────────────
search_gh_mcp() {
  local label="$1"; shift
  local query="$*"
  info "Searching GitHub: $label (\"$query\")"
  gh search repos "$query" --limit 15 \
    --json name,description,url,owner,stargazersCount,primaryLanguage,forkCount,updatedAt \
    2>/dev/null | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    results = []
    for r in data:
        lang = r.get('primaryLanguage', {}).get('name', 'unknown') if r.get('primaryLanguage') else 'unknown'
        results.append({
            'name': r['name'],
            'full_name': r['owner']['login'] + '/' + r['name'],
            'url': r['url'],
            'description': (r.get('description') or '')[:200],
            'stars': r['stargazersCount'],
            'forks': r['forkCount'],
            'language': lang,
            'updated': r.get('updatedAt', ''),
            'source': 'github',
            'category': '$label'
        })
    json.dump(results, sys.stdout)
except (json.JSONDecodeError, KeyError):
    json.dump([], sys.stdout)
"
}

# ─── 2. Crates.io Rust libs ─────────────────────────────
search_crates() {
  local label="$1"
  local query="$2"
  info "Searching crates.io: $label (\"$query\")"
  # Use grep to extract name + description from cargo search output
  cargo search "$query" --registry crates-io --limit 10 2>/dev/null | \
    sed -n 's/^\([^ ]*\) = "\([^"]*\)" \(.*\)/\1 | \2 | \3/p' | \
    while IFS='|' read -r name _ desc; do
      name="$(echo "$name" | xargs)"
      desc="$(echo "$desc" | xargs)"
      jq -c -n --arg n "$name" --arg d "${desc:0:200}" --arg u "https://crates.io/crates/$name" --arg c "$label" \
        '{name: $n, description: $d, url: $u, source: "crates-io", category: $c}' 2>/dev/null
    done
}

# ─── 3. Merge JSON arrays ──────────────────────────────
merge_json() {
  python3 -c "
import sys, json
combined = []
for line in sys.stdin:
    line = line.strip()
    if not line: continue
    try:
        combined.extend(json.loads(line))
    except json.JSONDecodeError:
        pass
json.dump(combined, sys.stdout)
"
}

# ─── Main Scan ──────────────────────────────────────────
echo "" >&2
echo "╔══════════════════════════════════════════════════════════╗" >&2
echo "║         Forge Tool Catalog — MCP Servers & Rust Libs   ║" >&2
echo "╚══════════════════════════════════════════════════════════╝" >&2
echo "" >&2

ALL_RESULTS='[]'

for search_tuple in \
  "gist:gist mcp server" \
  "gist:github gist api mcp" \
  "ipfs:ipfs mcp server" \
  "ipfs:ipfs mcp tool" \
  "github-api:github api mcp server" \
  "pastebin:pastebin mcp server" \
  "general:mcp-server rust"; do
  label="${search_tuple%%:*}"
  query="${search_tuple#*:}"
  results=$(search_gh_mcp "$label" "$query")
  ALL_RESULTS=$(printf '%s\n%s' "$ALL_RESULTS" "$results" | merge_json)
done

for search_tuple in \
  "github-api:octocrab" \
  "github-api:hubcaps" \
  "github-api:github-rs" \
  "github-api:gist" \
  "ipfs:ipfs-api" \
  "pastebin:pastebin-rs" \
  "pastebin:dpaste" \
  "general:mcp-client" \
  "general:rmcp"; do
  label="${search_tuple%%:*}"
  query="${search_tuple#*:}"
  results=$(search_crates "$label" "$query")
  ALL_RESULTS=$(printf '%s\n%s' "$ALL_RESULTS" "["$(echo "$results" | tr '\n' ',' | sed 's/,$//')"]" | merge_json)
done

# ─── Deduplicate and finalize ──────────────────────────
python3 <<PYEOF > "$CATALOG_FILE"
import json

data = json.loads('''$(echo "$ALL_RESULTS" | python3 -c "import sys,json; print(json.dumps(json.load(sys.stdin)))")''')

seen = set()
deduped = []
for r in data:
    key = r.get('full_name', r.get('name', ''))
    if key and key not in seen:
        seen.add(key)
        deduped.append(r)

def sort_key(r):
    stars = r.get('stars', 0) if 'stars' in r else 0
    cat_order = ['gist', 'ipfs', 'github-api', 'pastebin', 'general']
    cat_idx = cat_order.index(r.get('category', 'general')) if r.get('category') in cat_order else 99
    return (-stars, cat_idx)

deduped.sort(key=sort_key)

import datetime
output = {
    'meta': {
        'generated': datetime.datetime.utcnow().strftime('%Y-%m-%dT%H:%M:%SZ'),
        'total': len(deduped),
        'categories': list(set(r.get('category', 'other') for r in deduped))
    },
    'results': deduped
}
print(json.dumps(output, indent=2))
PYEOF

ok "Catalog written: $CATALOG_FILE ($(python3 -c "import json; d=json.load(open('$CATALOG_FILE')); print(d['meta']['total'])") entries)"

# ─── Summary report ────────────────────────────────────
echo "" >&2
echo "═══════════════════════════════════════════════════════════" >&2
echo "                      TOOL CATALOG REPORT" >&2
echo "═══════════════════════════════════════════════════════════" >&2
echo "" >&2

python3 <<PYEOF
import json
d = json.load(open('$CATALOG_FILE'))
results = d['results']

by_cat = {}
for r in results:
    cat = r.get('category', 'other')
    by_cat.setdefault(cat, []).append(r)

for cat in ['gist', 'ipfs', 'github-api', 'pastebin', 'general']:
    if cat not in by_cat:
        continue
    items = by_cat[cat]
    print(f'  [{cat.upper()}]  {len(items)} items', file=sys.stderr)
    for r in items[:8]:
        stars = r.get('stars', 0)
        lang = r.get('language', '')
        src = r.get('source', '')
        url = r.get('url', '')
        desc = r.get('description', '')[:100]
        if src == 'crates-io':
            print(f'         📦 {r["name"]}  — {desc}', file=sys.stderr)
        else:
            print(f'         ★{stars:>4}  {r.get("full_name", r["name"])}', file=sys.stderr)
            if desc:
                print(f'                {desc}', file=sys.stderr)
    if len(items) > 8:
        print(f'         ... and {len(items)-8} more', file=sys.stderr)
    print(file=sys.stderr)
PYEOF

# ─── Top picks by stars ────────────────────────────────
echo "  [NIX COMPATIBILITY]" >&2
echo "" >&2

python3 <<PYEOF
import json
d = json.load(open('$CATALOG_FILE'))
results = d['results']
top_gh = [r for r in results if r.get('source') == 'github']
top_gh.sort(key=lambda r: -r.get('stars', 0))

for r in top_gh[:10]:
    lang = r.get('language', '?')
    stars = r.get('stars', 0)
    name = r.get('full_name', r['name'])
    desc = r.get('description', '')[:80]
    rust_flag = '🦀' if lang == 'Rust' else '  '
    print(f'  {rust_flag} {name:45s} ★{stars:>4}  [{lang:>6}]  {desc}', file=sys.stderr)
print(file=sys.stderr)
print(f'  🦀 = Rust (preferred — native Nix build with cargoVendorDir)', file=sys.stderr)
print(f'  Note: Python/Node MCP servers can be wrapped but Rust libs', file=sys.stderr)
print(f'  integrate directly via forge_pipelight_mcp pattern.', file=sys.stderr)
PYEOF

# ─── Recommendations ────────────────────────────────────
echo "" >&2
echo "  RECOMMENDATIONS" >&2
echo "" >&2
echo "  1. Gist       → octocrab (Rust, 1380★) — forge_gist_mcp" >&2
echo "  2. IPFS       → ipfs-api-backend-hyper (Rust, HTTP client)" >&2
echo "  3. Pastebin   → system pastebinit CLI (already integrated)" >&2
echo "  4. GitHub API → octocrab (already vendored)" >&2
echo "" >&2
ok "Run with --query <filter> to filter results"
