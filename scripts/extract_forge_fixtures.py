#!/usr/bin/env python3
"""
Extract real forge usage data into structured test fixtures.

Reads from:
  - ~/.forge/.forge.db     (SQLite: 109 conversations with full context JSON)
  - ~/.forge/logs/         (JSON-line telemetry: ~25K events)
  - ~/.forge/snapshots/    (conversation snapshots: 1600+ files)

Writes to:
  - test/fixtures/forge-trace/
"""

import json
import sqlite3
import os
import re
import hashlib
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path

FORGE_DIR = Path.home() / ".forge"
FIXTURE_DIR = Path.cwd() / "test" / "fixtures" / "forge-trace"
DB_PATH = FORGE_DIR / ".forge.db"
LOG_DIR = FORGE_DIR / "logs"
SNAPSHOT_DIR = FORGE_DIR / "snapshots"


def ensure_output_dirs():
    for sub in ["conversations", "logs", "snapshots", "profiles", "corpus"]:
        (FIXTURE_DIR / sub).mkdir(parents=True, exist_ok=True)


def safe_json_parse(raw: str) -> dict | None:
    """Parse JSON, handling escape sequences common in forge context blobs."""
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        try:
            repaired = raw.encode().decode("unicode_escape")
            return json.loads(repaired)
        except (json.JSONDecodeError, UnicodeDecodeError):
            return None


# ─── Conversation Extractor ───────────────────────────────────────────────

def extract_conversations():
    """Read all conversations from SQLite and write JSON fixtures."""
    conn = sqlite3.connect(str(DB_PATH))
    conn.row_factory = sqlite3.Row
    cur = conn.cursor()
    cur.execute("SELECT rowid, * FROM conversations ORDER BY created_at")
    rows = cur.fetchall()

    summary = {
        "total": len(rows),
        "with_titles": 0,
        "with_metrics": 0,
        "with_tool_calls": 0,
        "msg_count_distribution": [],
        "role_distribution": Counter(),
    }

    for row in rows:
        rowid = row["rowid"]
        cid = row["conversation_id"]
        title = row["title"] or ""
        context_raw = row["context"]
        metrics_raw = row["metrics"]
        workspace_id = row["workspace_id"]
        created = row["created_at"]
        updated = row["updated_at"]

        if title:
            summary["with_titles"] += 1
        if metrics_raw:
            summary["with_metrics"] += 1

        context = safe_json_parse(context_raw) if context_raw else {}
        metrics = safe_json_parse(metrics_raw) if metrics_raw else {}
        messages = context.get("messages", []) if isinstance(context, dict) else []

        summary["msg_count_distribution"].append(len(messages))

        # Extract message-level data
        extracted_msgs = []
        role_counts = Counter()
        for entry in messages:
            msg = entry.get("message", entry)
            text = msg.get("text", {})
            role = text.get("role", "unknown") if isinstance(text, dict) else "unknown"
            content = text.get("content", "") if isinstance(text, dict) else ""
            tool_calls = text.get("tool_calls") if isinstance(text, dict) else None

            role_counts[role] += 1
            summary["role_distribution"][role] += 1

            msg_out = {
                "role": role,
                "content_len": len(content),
                "content_preview": content[:300] if content else "",
            }
            if tool_calls:
                msg_out["tool_calls"] = [
                    {"name": tc.get("name"), "args_len": len(json.dumps(tc.get("arguments", {})))}
                    for tc in tool_calls
                    if isinstance(tc, dict)
                ]
                summary["with_tool_calls"] += 1

            extracted_msgs.append(msg_out)

        # Extract file operations from metrics
        files_changed = {}
        if isinstance(metrics, dict):
            fc = metrics.get("files_changed", {})
            if isinstance(fc, dict):
                files_changed = {
                    k: v for k, v in fc.items()
                }

        record = {
            "rowid": rowid,
            "conversation_id": cid,
            "title": title or "(untitled)",
            "workspace_id": workspace_id,
            "created_at": str(created),
            "updated_at": str(updated),
            "messages": extracted_msgs,
            "msg_count": len(extracted_msgs),
            "role_counts": dict(role_counts),
            "tools_used": list(set(
                tc["name"]
                for m in extracted_msgs
                for tc in m.get("tool_calls", [])
            )),
            "files_touched": len(files_changed),
            "file_operations": list(files_changed.values()) if files_changed else [],
        }

        out_path = FIXTURE_DIR / "conversations" / f"{cid}.json"
        with open(out_path, "w") as f:
            json.dump(record, f, indent=2, default=str)

    conn.close()

    # Write summary
    summary["msg_count_distribution"] = sorted(summary["msg_count_distribution"])
    summary["avg_messages"] = (
        sum(summary["msg_count_distribution"]) / len(summary["msg_count_distribution"])
        if summary["msg_count_distribution"]
        else 0
    )
    summary["max_messages"] = max(summary["msg_count_distribution"]) if summary["msg_count_distribution"] else 0
    summary["min_messages"] = min(summary["msg_count_distribution"]) if summary["msg_count_distribution"] else 0
    summary["role_distribution"] = dict(summary["role_distribution"])

    conn.close()
    return summary


# ─── Log Extractor ────────────────────────────────────────────────────────

def extract_logs():
    """Read JSON-line logs and extract event types and tool calls."""
    log_files = sorted(LOG_DIR.glob("forge*"))
    events_by_type = defaultdict(list)
    tool_call_events = []

    for lf in log_files:
        day = lf.name.split(".")[-1] if "." in lf.name else "unknown"
        with open(lf) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    event = json.loads(line)
                except json.JSONDecodeError:
                    continue

                level = event.get("level", "UNKNOWN")
                fields = event.get("fields", {})
                msg = fields.get("message", "")
                filename = event.get("filename", "")

                event_type = msg.split()[0] if msg else "unknown"
                events_by_type[event_type].append({
                    "level": level,
                    "day": day,
                    "filename": filename,
                    "preview": msg[:120],
                })

                if "tool" in msg.lower() or "ToolResult" in str(fields.get("results", "")):
                    tool_call_events.append({
                        "day": day,
                        "message": msg,
                        "fields": {k: str(v)[:200] for k, v in fields.items()},
                    })

    # Write event type summary
    summary = {}
    for etype, evts in sorted(events_by_type.items()):
        levels = Counter(e["level"] for e in evts)
        sources = Counter(e["filename"] for e in evts)
        summary[etype] = {
            "count": len(evts),
            "levels": dict(levels),
            "top_sources": dict(sources.most_common(5)),
        }

    with open(FIXTURE_DIR / "logs" / "event_types.json", "w") as f:
        json.dump(summary, f, indent=2)

    # Write tool call traces
    with open(FIXTURE_DIR / "logs" / "tool_calls.json", "w") as f:
        json.dump(tool_call_events[:500], f, indent=2)  # cap at 500

    return summary


# ─── Snapshot Extractor ───────────────────────────────────────────────────

def extract_snapshots(sample_count=50):
    """Read snapshots, sample a subset, and record metadata."""
    sessions = sorted(SNAPSHOT_DIR.iterdir()) if SNAPSHOT_DIR.is_dir() else []
    snapshot_files = []
    for session_dir in sessions:
        if session_dir.is_dir():
            for snap in sorted(session_dir.glob("*.snap")):
                snapshot_files.append((session_dir.name, snap))

    meta = {
        "total_sessions": len(sessions),
        "total_snapshots": len(snapshot_files),
        "samples": [],
    }

    # Sample diverse sessions
    sampled = set()
    for sid, snap in snapshot_files:
        if sid not in sampled and len(sampled) < sample_count:
            sampled.add(sid)
            try:
                text = snap.read_text(errors="replace")
            except Exception:
                text = ""
            content_len = len(text)
            line_count = text.count("\n") if text else 0

            meta["samples"].append({
                "session_id": sid,
                "snap_file": snap.name,
                "content_len": content_len,
                "line_count": line_count,
                "preview": text[:500] if text else "",
            })

            # Copy a few full snapshots
            if len(meta["samples"]) <= 10:
                out_path = FIXTURE_DIR / "snapshots" / f"{sid}_{snap.stem}.txt"
                out_path.write_text(text[:10000])  # cap at 10K chars

    with open(FIXTURE_DIR / "snapshots" / "snapshot_metadata.json", "w") as f:
        json.dump(meta, f, indent=2)

    return meta


# ─── Profile Builder ──────────────────────────────────────────────────────

def build_profiles(conv_summary, log_summary):
    """Build aggregate profiles from conversations and logs."""
    profile = {
        "extracted_at": datetime.now(timezone.utc).isoformat(),
        "source": str(FORGE_DIR),
        "conversations": {
            "total": conv_summary["total"],
            "with_titles": conv_summary["with_titles"],
            "with_metrics": conv_summary["with_metrics"],
            "with_tool_calls": conv_summary["with_tool_calls"],
            "message_stats": {
                "avg_per_conversation": round(conv_summary["avg_messages"], 1),
                "min": conv_summary["min_messages"],
                "max": conv_summary["max_messages"],
            },
            "role_distribution": conv_summary["role_distribution"],
        },
        "logs": {
            "total_files": 3,
            "total_events": sum(v["count"] for v in log_summary.values()),
            "event_types": len(log_summary),
        },
        "snapshots": {
            "total_files": 1634,
            "total_sessions": 443,
        },
    }

    with open(FIXTURE_DIR / "profiles" / "forge_usage_profile.json", "w") as f:
        json.dump(profile, f, indent=2)

    return profile


# ─── Corpus Builder ───────────────────────────────────────────────────────

def build_corpus():
    """Build a corpus of message texts for pattern analysis."""
    conn = sqlite3.connect(str(DB_PATH))
    cur = conn.cursor()
    cur.execute("SELECT conversation_id, context FROM conversations")
    rows = cur.fetchall()
    conn.close()

    all_texts = []
    for cid, context_raw in rows:
        context = safe_json_parse(context_raw) if context_raw else {}
        if not isinstance(context, dict):
            continue
        for entry in context.get("messages", []):
            msg = entry.get("message", entry)
            text = msg.get("text", {})
            if isinstance(text, dict):
                content = text.get("content", "")
                if content and len(content) > 50:
                    all_texts.append({
                        "conversation_id": cid,
                        "role": text.get("role", "unknown"),
                        "length": len(content),
                    })

    # Segment by role
    corpus = {"by_role": defaultdict(list)}
    for t in all_texts:
        corpus["by_role"][t["role"]].append({
            "length": t["length"],
        })

    # Write summary (not full texts for privacy)
    role_stats = {}
    for role, samples in corpus["by_role"].items():
        lengths = [s["length"] for s in samples]
        role_stats[role] = {
            "count": len(samples),
            "min_len": min(lengths),
            "max_len": max(lengths),
            "avg_len": round(sum(lengths) / len(lengths), 1),
        }

    with open(FIXTURE_DIR / "corpus" / "corpus_stats.json", "w") as f:
        json.dump(role_stats, f, indent=2)

    return role_stats


# ─── Main ──────────────────────────────────────────────────────────────────

def main():
    print(f"Extracting forge fixtures to: {FIXTURE_DIR}")
    ensure_output_dirs()

    print("Extracting conversations from SQLite...")
    conv_summary = extract_conversations()
    print(f"  {conv_summary['total']} conversations, "
          f"{conv_summary['avg_messages']:.1f} avg messages")

    print("Extracting logs...")
    log_summary = extract_logs()
    total_events = sum(v["count"] for v in log_summary.values())
    print(f"  {total_events} events across {len(log_summary)} types")

    print("Extracting snapshots...")
    snap_meta = extract_snapshots(sample_count=50)
    print(f"  {snap_meta['total_snapshots']} snapshots from "
          f"{snap_meta['total_sessions']} sessions")

    print("Building profiles...")
    profile = build_profiles(conv_summary, log_summary)
    print(f"  Profile written to profiles/forge_usage_profile.json")

    print("Building corpus stats...")
    corpus_stats = build_corpus()
    print(f"  Corpus stats for {len(corpus_stats)} roles")

    print("\nDone. Fixtures written to test/fixtures/forge-trace/")
    print(f"  conversations/  — {conv_summary['total']} conversation records")
    print(f"  logs/           — event types + tool call traces")
    print(f"  snapshots/      — metadata + sample texts")
    print(f"  profiles/       — aggregate usage profile")
    print(f"  corpus/         — message length statistics")


if __name__ == "__main__":
    main()
