---
name: nagios-tile-monitor
description: >-
  Nagios-style DASL tile health monitor (port 8800). Diagnose, start, develop, use.
  infra — monitoring
---

# 🟩 Nagios Tile Monitor
**Port:** 8800 | **URL:** https://solana.solfunmeme.com/tile/nagios/

## Diagnose
```bash
systemctl status nagios-tile-monitor
curl http://127.0.0.1:8800/health
```

## Start
```bash
sudo systemctl restart nagios-tile-monitor
```

## Develop
```bash
vim ~/DOCS/nagios-tile-server.py
python3 ~/DOCS/nagios-tile-server.py --port 8899 &  # test
```

## Use
- https://solana.solfunmeme.com/tile/nagios/
- `curl http://127.0.0.1:8800/api/status`

## Shmem Cross-References

> Generated: 2026-06-23 10:20:00 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| — | No matches in shmem for 7 keywords | — |
| — | Searchable terms: Develop, Diagnose, Monitor, Nagios, Start, Tile, Use | — |