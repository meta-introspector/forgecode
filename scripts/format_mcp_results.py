#!/usr/bin/env python3
"""Parse MCP search_repositories JSON results and print a summary."""
import json, sys

raw = sys.stdin.read()
data = json.loads(raw)
items = data.get("items", [] if "items" in data else [data])
print(f"Total results: {data.get('total_count', '?')}\n")
for i, repo in enumerate(items[:10], 1):
    name = repo.get("full_name", "?")
    desc = (repo.get("description") or "(no description)")[:120]
    stars = repo.get("stargazers_count", 0)
    lang = repo.get("language") or "?"
    url = repo.get("html_url", "?")
    topics = ", ".join(repo.get("topics", [])[:5])
    print(f"{i}. {name}")
    print(f"   ⭐{stars}  {lang}")
    print(f"   {desc}")
    if topics:
        print(f"   🏷️  {topics}")
    print(f"   {url}")
    print()
