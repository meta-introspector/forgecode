#!/usr/bin/env python3
"""
forge-recall — Cross-reference conversations, file paths, and commands by git tree.

Usage:
    python3 scripts/forge_recall.py index                          # Build/refresh the index
    python3 scripts/forge_recall.py query <path> [--tree]          # Find conversations for a path
    python3 scripts/forge_recall.py tree <commit-ish>              # Show conversations near a git tree
    python3 scripts/forge_recall.py list-paths [--prefix <dir>]    # List all indexed file paths
    python3 scripts/forge_recall.py stats                          # Show index statistics
"""

import json
import os
import re
import sqlite3
import subprocess
import sys
import time
import glob
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

# ── Config ──────────────────────────────────────────────────────────────────
FORGE_DB = os.path.expanduser("~/.forge/.forge.db")
FIXTURES_DIR = "test/fixtures/forge-trace/conversations"
INDEX_DIR = "test/fixtures/forge-trace/index"
INDEX_FILE = os.path.join(INDEX_DIR, "recall_index.json")
GIT_LOG_CACHE = os.path.join(INDEX_DIR, "git_log_cache.json")
MAX_MESSAGE_PREVIEW = 300
FILE_PATH_RE = re.compile(r'(?:/[\w.\-]+)+(?:/\w[\w.\-]*)*')

# ── Git helpers ──────────────────────────────────────────────────────────────

def get_git_log(repo_path="."):
    """Get git log with timestamps, sorted oldest-first."""
    cache_file = GIT_LOG_CACHE if repo_path == "." else None
    
    if cache_file and os.path.exists(cache_file):
        with open(cache_file) as f:
            return json.load(f)
    
    result = subprocess.run(
        ["git", "log", "--format=%H %ct %T %s", "--date-order", "--all", "--reverse"],
        cwd=repo_path,
        capture_output=True, text=True, timeout=30
    )
    if result.returncode != 0:
        print(f"Warning: git log failed: {result.stderr[:200]}", file=sys.stderr)
        return []
    
    entries = []
    for line in result.stdout.strip().split("\n"):
        if not line.strip():
            continue
        parts = line.split(" ", 3)
        if len(parts) >= 3:
            entries.append({
                "commit": parts[0],
                "timestamp": int(parts[1]),
                "tree": parts[2],
                "message": parts[3] if len(parts) > 3 else ""
            })
    
    if cache_file:
        os.makedirs(os.path.dirname(cache_file), exist_ok=True)
        with open(cache_file, "w") as f:
            json.dump(entries, f)
    
    return entries


def find_closest_commit(timestamp, git_log):
    """Find the git commit closest to a given Unix timestamp."""
    if not git_log:
        return None
    
    # Binary search for the closest timestamp
    lo, hi = 0, len(git_log) - 1
    best = 0
    while lo <= hi:
        mid = (lo + hi) // 2
        if git_log[mid]["timestamp"] == timestamp:
            return git_log[mid]
        if git_log[mid]["timestamp"] < timestamp:
            best = mid
            lo = mid + 1
        else:
            hi = mid - 1
    
    # Check neighbor
    if best + 1 < len(git_log):
        if abs(git_log[best + 1]["timestamp"] - timestamp) < abs(git_log[best]["timestamp"] - timestamp):
            best = best + 1
    
    return git_log[best]


def get_git_diff_for_commit(commit_hash, repo_path="."):
    """Get the list of files changed in a commit."""
    result = subprocess.run(
        ["git", "diff-tree", "--no-commit-id", "--name-only", "-r", commit_hash],
        cwd=repo_path,
        capture_output=True, text=True, timeout=15
    )
    if result.returncode == 0:
        return [f.strip() for f in result.stdout.strip().split("\n") if f.strip()]
    return []


# ── File path extraction from messages ──────────────────────────────────────

def extract_paths_from_text(text):
    """Extract file paths from text content."""
    if not text:
        return set()
    paths = set()
    # Match patterns like: path/to/file.ext or /absolute/path
    for match in FILE_PATH_RE.finditer(text):
        path = match.group(0)
        # Filter: must have at least one dot (extension) or be an absolute path
        if ('.' in path or path.startswith('/')) and len(path) > 5:
            # Filter out obvious non-paths
            if not any(skip in path for skip in ['http://', 'https://', '://']):
                paths.add(path)
    return paths


def extract_paths_from_conversation(conv):
    """Extract all file paths mentioned in a conversation."""
    paths = set()
    
    # From file_operations
    file_ops = conv.get("file_operations") or conv.get("files_changed") or {}
    if isinstance(file_ops, dict):
        paths.update(file_ops.keys())
    
    # From files_touched
    files_touched = conv.get("files_touched") or []
    if isinstance(files_touched, list):
        paths.update(files_touched)
    
    # From messages
    for msg in conv.get("messages", []):
        cp = msg.get("content_preview") or msg.get("content") or ""
        if isinstance(cp, str):
            paths.update(extract_paths_from_text(cp))
    
    return paths


# ── Shell command extraction from DB ─────────────────────────────────────────

def extract_commands_with_paths(db_path=FORGE_DB):
    """Extract shell commands from the database that reference file paths."""
    commands = []
    if not os.path.exists(db_path):
        return commands
    
    try:
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        # Try to get context JSON which has tool calls with shell commands
        cursor.execute(
            "SELECT conversation_id, context, created_at, updated_at FROM conversations WHERE context LIKE '%shell%' OR context LIKE '%command%'"
        )
        
        for row in cursor.fetchall():
            conv_id, context_json, created_at, updated_at = row
            if not context_json:
                continue
            
            try:
                ctx = json.loads(context_json)
            except json.JSONDecodeError:
                continue
            
            # Extract messages from context
            messages = []
            if isinstance(ctx, dict):
                messages = ctx.get("messages", ctx.get("context", []))
            elif isinstance(ctx, list):
                messages = ctx
            
            for msg in messages:
                # Handle 'message' wrapper
                inner = msg.get("message", msg) if isinstance(msg, dict) else {}
                
                # Extract tool calls
                if isinstance(inner, dict):
                    tool_calls = inner.get("tool_calls") or []
                    for tc in tool_calls:
                        if isinstance(tc, dict):
                            func = tc.get("function", tc)
                            if isinstance(func, dict):
                                name = func.get("name", "")
                                args_str = func.get("arguments", "")
                                if isinstance(args_str, str) and name:
                                    try:
                                        args = json.loads(args_str)
                                    except json.JSONDecodeError:
                                        continue
                                    cmd = args.get("command", "")
                                    path = args.get("path", "")
                                    
                                    paths_in_cmd = set()
                                    if path:
                                        paths_in_cmd.add(path)
                                    if cmd:
                                        paths_in_cmd.update(extract_paths_from_text(cmd))
                                    
                                    if paths_in_cmd:
                                        commands.append({
                                            "conversation_id": conv_id,
                                            "tool": name,
                                            "command": cmd if name == "shell" else "",
                                            "path": path if name in ("read", "write", "patch", "remove") else "",
                                            "paths": list(paths_in_cmd),
                                            "timestamp": created_at or updated_at or ""
                                        })
            conn.close()
    except Exception as e:
        print(f"Warning: DB extraction error: {e}", file=sys.stderr)
    
    return commands


# ── Index builder ────────────────────────────────────────────────────────────

def build_index():
    """Build the recall index from all available sources."""
    print("Building forge-recall index...")
    
    # 1. Load git log
    print("  Scanning git history...")
    git_log = get_git_log()
    print(f"  Found {len(git_log)} commits")
    
    # 2. Load conversations from fixtures
    print("  Scanning conversation fixtures...")
    conversations = []
    fixture_files = sorted(glob.glob(os.path.join(FIXTURES_DIR, "*.json")))
    
    for fpath in fixture_files:
        try:
            with open(fpath) as f:
                conv = json.load(f)
            conversations.append(conv)
        except Exception as e:
            print(f"  Warning: skipping {fpath}: {e}", file=sys.stderr)
    
    print(f"  Loaded {len(conversations)} conversations")
    
    # 3. Extract commands from DB
    print("  Extracting commands from DB...")
    commands = extract_commands_with_paths()
    print(f"  Found {len(commands)} commands referencing file paths")
    
    # 4. Build inverted index: file_path → entries
    index = defaultdict(list)
    
    for conv in conversations:
        conv_id = conv.get("conversation_id") or conv.get("rowid", "unknown")
        title = conv.get("title", "")
        created = conv.get("created_at", "")
        
        # Parse timestamp
        timestamp = 0
        if isinstance(created, str):
            try:
                dt = datetime.fromisoformat(created.replace("Z", "+00:00"))
                timestamp = int(dt.timestamp())
            except ValueError:
                pass
        elif isinstance(created, (int, float)):
            timestamp = int(created)
        
        # Use git_tree_hash from metrics if available, otherwise approximate from git log
        metrics = conv.get("metrics", {}) or {}
        git_tree_hash = metrics.get("git_tree_hash", None) if isinstance(metrics, dict) else None
        
        git_entry = None
        if git_tree_hash:
            # Find the exact commit by tree hash in git log
            for ge in git_log:
                if ge["tree"] == git_tree_hash:
                    git_entry = ge
                    break
        
        if git_entry is None:
            # Fallback: approximate from timestamp
            git_entry = find_closest_commit(timestamp, git_log)
        
        # Extract paths from this conversation
        paths = extract_paths_from_conversation(conv)
        
        # Get file operations summary
        file_ops = conv.get("file_operations") or conv.get("files_changed") or {}
        op_summary = {}
        if isinstance(file_ops, dict):
            for p, info in file_ops.items():
                if isinstance(info, dict):
                    op_summary[p] = {
                        "tool": info.get("tool", ""),
                        "lines_added": info.get("lines_added", 0),
                        "lines_removed": info.get("lines_removed", 0),
                    }
        
        entry = {
            "type": "conversation",
            "conversation_id": conv_id,
            "title": title,
            "timestamp": timestamp,
            "datetime": created,
            "git_commit": git_entry["commit"] if git_entry else None,
            "git_tree": git_tree_hash or (git_entry["tree"] if git_entry else None),
            "git_tree_exact": git_tree_hash is not None,
            "git_message": git_entry["message"] if git_entry else None,
            "msg_count": conv.get("msg_count", len(conv.get("messages", []))),
            "tools_used": conv.get("tools_used", []),
            "file_operations": op_summary,
        }
        
        for path in paths:
            # Normalize path
            norm_path = os.path.normpath(path)
            index[norm_path].append(entry)
    
    # 5. Add commands to index
    for cmd in commands:
        cmd_entry = {
            "type": "command",
            "conversation_id": cmd["conversation_id"],
            "tool": cmd["tool"],
            "command": cmd["command"],
            "path": cmd["path"],
            "timestamp": cmd["timestamp"],
        }
        for path in cmd["paths"]:
            norm_path = os.path.normpath(path)
            index[norm_path].append(cmd_entry)
    
    # 6. Build directory index
    dir_index = defaultdict(list)
    for path, entries in index.items():
        parts = Path(path).parts
        for i in range(1, len(parts) + 1):
            dir_path = os.path.join(*parts[:i])
            if i == len(parts):
                dir_index.setdefault(dir_path, []).extend(entries)
    
    # 7. Save index
    os.makedirs(INDEX_DIR, exist_ok=True)
    
    output = {
        "built_at": datetime.now(timezone.utc).isoformat(),
        "stats": {
            "total_commits": len(git_log),
            "total_conversations": len(conversations),
            "total_commands": len(commands),
            "total_indexed_paths": len(index),
            "total_indexed_dirs": len(dir_index),
        },
        "by_file": {k: v for k, v in sorted(index.items())},
        "by_directory": {k: v for k, v in sorted(dir_index.items())},
        "git_log": git_log[-100:],  # only keep last 100 for quick reference
    }
    
    with open(INDEX_FILE, "w") as f:
        json.dump(output, f, indent=2, default=str)
    
    print(f"\nIndex saved to {INDEX_FILE}")
    print(f"  {len(index)} unique file paths indexed")
    print(f"  {len(dir_index)} unique directories indexed")
    
    return output


# ── Query ────────────────────────────────────────────────────────────────────

def query_path(pattern, show_tree=False):
    """Query the index for conversations related to a file path."""
    if not os.path.exists(INDEX_FILE):
        print("Index not found. Run 'forge_recall.py index' first.", file=sys.stderr)
        return
    
    with open(INDEX_FILE) as f:
        index = json.load(f)
    
    by_file = index.get("by_file", {})
    by_dir = index.get("by_directory", {})
    
    norm_pattern = os.path.normpath(pattern)
    
    # Find all matching paths
    matching_files = {}
    for path, entries in by_file.items():
        if norm_pattern in path or path.endswith(norm_pattern):
            matching_files[path] = entries
    
    matching_dirs = {}
    for path, entries in by_dir.items():
        if norm_pattern in path or path == norm_pattern:
            # Only show directory matches that aren't also file matches
            if path not in matching_files:
                matching_dirs[path] = entries
    
    if not matching_files and not matching_dirs:
        print(f"No results for '{pattern}'")
        return
    
    results = []
    
    # Process file matches
    for path, entries in matching_files.items():
        for entry in entries:
            if entry["type"] == "conversation":
                results.append((path, entry, "conversation"))
            elif entry["type"] == "command":
                results.append((path, entry, "command"))
    
    # Process directory matches
    for path, entries in matching_dirs.items():
        for entry in entries:
            if entry["type"] == "conversation":
                results.append((path, entry, "conversation"))
    
    # Sort by git tree (commit timestamp) or timestamp
    def sort_key(item):
        path, entry, etype = item
        ts = entry.get("timestamp", 0)
        if isinstance(ts, str):
            try:
                ts = int(datetime.fromisoformat(ts.replace("Z", "+00:00")).timestamp())
            except:
                ts = 0
        return ts
    
    if show_tree:
        results.sort(key=lambda x: x[1].get("git_tree", "") or "")
    else:
        results.sort(key=sort_key)
    
    # Deduplicate by conversation_id + path
    seen = set()
    unique_results = []
    for item in results:
        path, entry, etype = item
        dedup_key = (entry.get("conversation_id", ""), path)
        if dedup_key not in seen:
            seen.add(dedup_key)
            unique_results.append(item)
    
    print(f"\n{'='*72}")
    print(f"  Results for: {pattern}")
    print(f"{'='*72}")
    print(f"  {len(unique_results)} unique results ({len(matching_files)} file paths, {len(matching_dirs)} directories)")
    print()
    
    # Group by conversation
    by_conv = defaultdict(list)
    for item in unique_results:
        path, entry, etype = item
        cid = entry.get("conversation_id", "unknown")
        by_conv[cid].append(item)
    
    for cid, items in sorted(by_conv.items(), 
                              key=lambda x: min((i[1].get("timestamp", 0) or 0) for i in x[1]),
                              reverse=True):
        # Use first entry for metadata
        first = items[0][1]
        
        # Format time
        ts = first.get("timestamp", 0)
        if ts:
            try:
                dt = datetime.fromtimestamp(int(ts) if isinstance(ts, (int, float)) else 0)
                time_str = dt.strftime("%Y-%m-%d %H:%M")
            except:
                time_str = str(ts)
        else:
            time_str = "unknown"
        
        title = first.get("title", "untitled")
        git_commit = first.get("git_commit", "")[:12] if first.get("git_commit") else ""
        git_tree = first.get("git_tree", "")[:12] if first.get("git_tree") else ""
        git_tree_exact = first.get("git_tree_exact", False)
        git_msg = first.get("git_message", "")
        
        print(f"  [{time_str}] {title}")
        print(f"  ├─ Conversation: {cid[:12]}...")
        if git_commit:
            exact_label = " (exact)" if git_tree_exact else " (approx)"
            print(f"  ├─ Git commit: {git_commit}  tree: {git_tree}{exact_label}")
            if git_msg:
                print(f"  ├─ Git message: {git_msg[:80]}")
        
        # Show paths referenced
        paths_in_conv = sorted(set(p for p, _, _ in items))
        if paths_in_conv:
            print(f"  └─ Files ({len(paths_in_conv)}):")
            for p in paths_in_conv[:8]:
                tool = ""
                # Check if there's a tool operation
                for _, entry, etype in items:
                    if entry.get("type") == "conversation" and p in entry.get("file_operations", {}):
                        op = entry["file_operations"][p]
                        tool = f" ({op.get('tool','')})"
                        break
                else:
                    for _, entry, etype in items:
                        if entry.get("type") == "command" and p in entry.get("paths", [p]):
                            tool = f" ({entry.get('tool','')})"
                            break
                print(f"     ├─ {p}{tool}")
            if len(paths_in_conv) > 8:
                print(f"     └─ ... and {len(paths_in_conv) - 8} more")
        print()


# ── Tree viewer ──────────────────────────────────────────────────────────────

def show_tree(commit_ish):
    """Show conversations organized by git tree state."""
    if not os.path.exists(INDEX_FILE):
        print("Index not found. Run 'forge_recall.py index' first.", file=sys.stderr)
        return
    
    with open(INDEX_FILE) as f:
        index = json.load(f)
    
    # Get git log for this commit
    git_log = index.get("git_log", [])
    
    # Find matching commits
    matching = []
    for entry in git_log:
        if commit_ish in entry["commit"] or commit_ish in entry["tree"] or commit_ish in entry["message"]:
            matching.append(entry)
    
    if not matching:
        # Try to get commit info directly
        result = subprocess.run(
            ["git", "log", "-1", "--format=%H %ct %T %s", commit_ish],
            capture_output=True, text=True, timeout=10
        )
        if result.returncode == 0 and result.stdout.strip():
            parts = result.stdout.strip().split(" ", 3)
            matching = [{
                "commit": parts[0],
                "timestamp": int(parts[1]),
                "tree": parts[2],
                "message": parts[3] if len(parts) > 3 else ""
            }]
    
    if not matching:
        print(f"No matching commits for '{commit_ish}'")
        return
    
    by_file = index.get("by_file", {})
    
    for commit_entry in matching[:5]:
        commit_hash = commit_entry["commit"]
        tree_hash = commit_entry["tree"]
        ts = commit_entry["timestamp"]
        msg = commit_entry.get("message", "")
        
        dt = datetime.fromtimestamp(ts)
        
        print(f"\n{'─'*72}")
        print(f"  Commit: {commit_hash[:12]}  Tree: {tree_hash[:12]}")
        print(f"  Date:   {dt.strftime('%Y-%m-%d %H:%M')}")
        print(f"  Msg:    {msg}")
        print(f"{'─'*72}")
        
        # Find files changed in this commit
        changed_files = get_git_diff_for_commit(commit_hash)
        
        if changed_files:
            print(f"\n  Files changed ({len(changed_files)}):")
            for cf in changed_files[:15]:
                # Check if we have conversations about this file
                conversations_about = by_file.get(cf, [])
                conv_count = len([e for e in conversations_about if e["type"] == "conversation"])
                cmd_count = len([e for e in conversations_about if e["type"] == "command"])
                label = ""
                if conv_count or cmd_count:
                    label = f"  ← {conv_count} conversations, {cmd_count} commands"
                print(f"    {cf}{label}")
            if len(changed_files) > 15:
                print(f"    ... and {len(changed_files) - 15} more")
        else:
            print("  (root commit or no file changes)")


# ── List paths ───────────────────────────────────────────────────────────────

def list_paths(prefix=""):
    """List all indexed file paths, optionally filtered by prefix."""
    if not os.path.exists(INDEX_FILE):
        print("Index not found. Run 'forge_recall.py index' first.", file=sys.stderr)
        return
    
    with open(INDEX_FILE) as f:
        index = json.load(f)
    
    by_file = index.get("by_file", {})
    
    matching = []
    for path in by_file:
        if prefix and prefix not in path:
            continue
        
        # Count conversations and commands for this path
        entries = by_file[path]
        conv_count = len([e for e in entries if e["type"] == "conversation"])
        cmd_count = len([e for e in entries if e["type"] == "command"])
        
        matching.append((path, conv_count, cmd_count))
    
    matching.sort(key=lambda x: x[0])
    
    print(f"\n{'='*72}")
    print(f"  Indexed file paths{' containing ' + prefix if prefix else ''}")
    print(f"{'='*72}")
    print(f"  {len(matching)} paths found")
    print()
    
    for path, conv_count, cmd_count in matching[:50]:
        print(f"  {path:70s}  C:{conv_count:3d}  S:{cmd_count:3d}")
    if len(matching) > 50:
        print(f"  ... and {len(matching) - 50} more")


# ── Stats ────────────────────────────────────────────────────────────────────

def show_stats():
    """Show index statistics."""
    if not os.path.exists(INDEX_FILE):
        print("Index not found. Run 'forge_recall.py index' first.", file=sys.stderr)
        return
    
    with open(INDEX_FILE) as f:
        index = json.load(f)
    
    stats = index.get("stats", {})
    by_file = index.get("by_file", {})
    
    # Top paths by conversation count
    path_counts = []
    for path, entries in by_file.items():
        conv_count = len([e for e in entries if e["type"] == "conversation"])
        if conv_count > 0:
            path_counts.append((path, conv_count))
    
    path_counts.sort(key=lambda x: -x[1])
    
    print(f"\n{'='*72}")
    print(f"  Forge-Recall Index Statistics")
    print(f"{'='*72}")
    print(f"  Built:           {index.get('built_at', 'unknown')}")
    print(f"  Total commits:   {stats.get('total_commits', 0)}")
    print(f"  Conversations:   {stats.get('total_conversations', 0)}")
    print(f"  Commands:        {stats.get('total_commands', 0)}")
    print(f"  Indexed paths:   {stats.get('total_indexed_paths', 0)}")
    print(f"  Indexed dirs:    {stats.get('total_indexed_dirs', 0)}")
    print()
    
    if path_counts:
        print("  Top paths by conversation:")
        for path, count in path_counts[:15]:
            entries = by_file[path]
            tools = set()
            for e in entries:
                if e["type"] == "conversation":
                    tools.update(e.get("tools_used", []))
                elif e["type"] == "command":
                    tools.add(e.get("tool", ""))
            tools_str = ", ".join(sorted(tools)[:4])
            print(f"    {count:3d}x  {path:60s}  [{tools_str}]")


# ── CLI ───────────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    
    command = sys.argv[1]
    
    if command == "index":
        build_index()
    elif command == "query":
        if len(sys.argv) < 3:
            print("Usage: forge_recall.py query <path> [--tree]")
            sys.exit(1)
        path = sys.argv[2]
        show_tree_flag = "--tree" in sys.argv
        query_path(path, show_tree=show_tree_flag)
    elif command == "tree":
        if len(sys.argv) < 3:
            print("Usage: forge_recall.py tree <commit-ish>")
            sys.exit(1)
        show_tree(sys.argv[2])
    elif command == "list-paths":
        prefix = ""
        if len(sys.argv) > 2 and sys.argv[2] == "--prefix" and len(sys.argv) > 3:
            prefix = sys.argv[3]
        list_paths(prefix)
    elif command == "stats":
        show_stats()
    else:
        print(f"Unknown command: {command}")
        print(__doc__)
        sys.exit(1)


if __name__ == "__main__":
    main()
