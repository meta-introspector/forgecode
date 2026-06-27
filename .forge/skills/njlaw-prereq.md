# njlaw prereq (New Jersey Legislature DAO legal entity prerequites search)

## Purpose
Take a draft bill or DAO legal text and find existing New Jersey statutory prerequisites, related bills, and compliance anchors in the local njlaw archive and the NJ Legislature server.

## Prerequisites
- `njlaw` Rust CLI built in `/mnt/data1/time-2026/06/17/njlaw`
- Local archive at `~/archive/njleg` with:
  - `downloads/Statutes/STATUTES-TEXT.zip`
  - `index/` (tantivy index)
  - `project/*.listing.json` (server listings)
  - `parsed/nj-dao-glossary.txt`
  - `draft/*.txt`

## Run prereq
```bash
cargo run -- prereq /mnt/data1/time-2026/06/17/njlaw/draft/solfunmeme-dao-bill.txt --output ~/archive/njleg/prereq/report.txt
```

## What prereq does
1. Extract keywords from the draft using regex.
2. Search the local statute ZIP (`STATUTES.TXT`) for exact keyword matches.
3. Search the tantivy index for `body:<keyword>` hits.
4. Print a combined report with ZIP status, index hit counts, and hit summaries.

## Known limitations
- ZIP search is full-text exact-match only; it does not yet extract per-title blocks.
- Tantivy doc retrieval is not used; we report hit counts only to avoid Tantivy 0.22 API gaps.
- Bill-specific text files (A2371, S1756, A3886) are not yet downloaded or indexed.

## Next steps
- Download related bill texts via `cmd_sync_project` or targeted fetcher.
- Add bill texts to the download corpus and rebuild the tantivy index.
- Confirm index hits via external sources (Google, GitHub, HuggingFace, Pastebin).
- Generate investor/regulator reports from confirmed matches.

## Related files
- `~/archive/njleg/prereq/solfunmeme-dao-prereq-2026-06-19.txt`
- `~/archive/njleg/parsed/nj-dao-glossary.txt`
- `/mnt/data1/time-2026/06/17/njlaw/src/prereq.rs`
- `/mnt/data1/time-2026/06/17/njlaw/src/indexer.rs`
- `/mnt/data1/time-2026/06/17/njlaw/draft/solfunmeme-dao-bill.txt`
