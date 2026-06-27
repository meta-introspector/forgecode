---
name: n0-dasl-tile
description: >-
  ⚠️ n0_dasl — Alternative DASL Impl tile (port 18009) — diagnose, start, develop, and use.
  harness
---

# ⚠️ n0_dasl — Alternative DASL Impl

**Port:** 18009 | **Health:** http://127.0.0.1:18009/health | **Category:** harness

## Diagnose

```bash
# Is it running?
systemctl is-active n0-dasl-tile 2>/dev/null || ss -tlnp "sport = :18009"
curl -s --max-time 2 http://127.0.0.1:18009/health

# Check logs
journalctl -u n0-dasl-tile --no-pager -n 20
tail -f ~/log/n0-dasl-tile.log 2>/dev/null

# Port check
ss -tlnp "sport = :18009"
```

## Start

```bash
# Via system-manager (recommended)
sudo systemctl restart n0-dasl-tile

# Manual (if no systemd unit)
cd ~/dasl/dasl-testing
nohup ./target/release/service > /tmp/n0-dasl-tile.log 2>&1 &
```

## Develop

```bash
# Build
cd ~/dasl/dasl-testing/harnesses/dasl-tile 2>/dev/null || cd ~/dasl
cargo build --release

# Test locally
curl http://127.0.0.1:18009/health
```

## Use

```bash
# Health check
curl http://127.0.0.1:18009/health

# Decode CBOR (harness tiles)
curl -X POST http://127.0.0.1:18009/decode \
  -H "Content-Type: application/json" \
  -d '{"cbor":"74657374"}'

# View in nagios dashboard
open https://solana.solfunmeme.com/tile/nagios/
```

## Related
- [[skills/dasl-testing]] — Cross-implementation fuzzing harness
- [[skills/dasl-tile-guide]] — Tile schema & deployment
- [[skills/tile-usage]] — How to use tiles daily
- [[skills/tile-testing]] — Test suite

## Shmem Cross-References

> Generated: 2026-06-23 10:20:00 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| Build | buildPrefixTree | def |
| DASL | writeDaslFile | def |
| DASL | meme_DASL2_LITERATE | theorem |
| DASL | DaslItem | structure |
| Port | Finset.mulSupport_of_fiberwise_prod_subset_image | theorem |
| Port | finprod_eq_prod_of_mulSupport_subset_of_finite | theorem |
| Port | integral_pos_iff_support_of_nonneg_ae' | theorem |
| Test | test_lemma | lemma |
| Test | test_ingest | theorem |
| Test | test_pow | theorem |