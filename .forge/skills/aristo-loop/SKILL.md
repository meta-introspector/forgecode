---
name: aristo-loop
description: >-
  Execute the 6-step Aristotle Loop: refresh, audit, tag refusals/winging,
  pick best project, resolve and index into shmem, capture traffic.
---

# aristo-loop

## one-liner
```bash
/mnt/data1/time-2026/06-june/23/aristo-loop/aristo-loop.sh
```

## Trigger
When operating the Aristotle → DASL testing cycle: polling, splitting, auditing refusals, capturing traffic.

## Approach
1. Run 01-refresh.sh (fetch + refresh + build)
2. Run 02-audit.sh (decl-table + sorry scan)
3. Run 03-tag.sh (grep for refusals/winging)
4. Run 04-pick.sh (rank and select top project)
5. Run 05-resolve.sh (build, shmem index, verify searchable)
6. Run 06-capture.sh (tcpdump/mitmproxy + cache)

## See Also
- [[tasks/aristo-loop]]
- SOP-ARISTO-001
