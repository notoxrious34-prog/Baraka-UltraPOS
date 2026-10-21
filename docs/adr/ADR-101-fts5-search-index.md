# ADR-101: FTS5 Search Index with Pre-Normalized Columns and Alias Table

## Status
Accepted

## Context
AUD-036 baselined the BRK-073 hybrid search: three-stage logic over
`baraka_search_norm(...) LIKE %...%` across five text sources. TASK-BRK-075
then measured the real cost at 80,000 domain-valid rows on the reference
machine (Win10/i5-3450/16GB/SATA SSD, debug profile, WAL, synchronous=FULL):
p50/p95 per query range from ~5.2/5.3s (multi-token) to ~18.5/18.8s
(single FR token); even the barcode-exact path costs ~1.6/1.7s. EXPLAIN
confirms a full index scan (`SCAN p USING INDEX idx_products_sku`) because
no index can serve a leading-`%` LIKE, plus a correlated unit-barcode search
per row. The INIT-11 target is p95 < 50ms at 80k rows: the gap is two to
three orders of magnitude, decisive even allowing for debug-vs-release
ratios. The 74-entry alias dictionary lives in code with no table, and no
name index exists. FTS5 (ADR-011's original prescription) was never built.

## Decision
1. One new append-only migration (0016) creates an FTS5 external-content
   table `products_fts(rowid, norm_fr, norm_ar, norm_sku, norm_barcode,
   norm_aliases)` with `tokenize='unicode61 "remove_diacritics 1"'`,
   `content=''`, plus insert/delete/update triggers that mirror the
   pre-normalized source columns.
2. Source of truth stays relational: 0016 adds `products.search_norm_fr`,
   `products.search_norm_ar` and a `product_aliases(product_id, alias_norm)`
   table (UNIQUE(product_id, alias_norm), FK to products, seeded from the
   existing 74-entry code dictionary). The repository is the sole writer of
   norm columns, using the domain `search_normalization` module (no SQL-side
   normalization duplication); the migration backfills norms for existing
   rows inside the same transaction.
3. Query path becomes: (a) barcode-exact indexed equality first (covering
   index on unit/product barcodes, added in 0016 if absent); (b) otherwise a
   single FTS `MATCH` of per-token prefix queries (`"tok"*`) ANDed across
   columns, ordered by BM25 rank with the existing CASE-relevance tiers kept
   as tie-break documentation, LIMIT clamp 1-500 unchanged.
4. Consistency is structural, not hoped-for: triggers keep FTS rows
   synchronous on every write path; a repository `verify_fts_consistency`
   (row counts + sampled content hash) runs in tests and is exposed for the
   backup/restore verification path later; a `rebuild_fts` function exists
   for recovery. Startup performs only the O(1) count check, never a full
   rebuild.
5. Proof is the existing probe: `search_perf_80k` (#[ignore]) is re-run
   explicitly after cutover against the same 80k seed; the INIT-11 bar
   (p95 < 50ms, debug profile, conservative) must hold on all six queries
   before the FTS5 track is declared done.

## Consequences
- Positive: search cost moves from full-scan seconds to indexed
  milliseconds; Arabic/French normalization stays in one domain module;
  aliases become data (importable later) instead of code.
- Negative / accepted debt: FTS index grows database size (measured and
  recorded by the implementing tasks); `unicode61` handles diacritics and
  case but NOT alef/yaa/taa-marbuta variants or French-in-Arabic mapping —
  that stays in `search_normalization` at write time (pre-normalized
  columns), so the FTS tokenizer is deliberately dumb. Punctuation
  normalization (the one AUD-036 gap) is folded into the same module.
- rusqlite `bundled` is expected to ship FTS5; the implementing task
  verifies with `sqlite_compileoption_used('ENABLE_FTS5')` as step zero and
  stops with evidence if absent (no silent fallback to LIKE).
- Implementation is split: BRK-076 (migration 0016 + backfill + triggers +
  consistency test, zero query-path change) then BRK-077 (repository
  cutover + ranking + alias use + probe re-proof).
