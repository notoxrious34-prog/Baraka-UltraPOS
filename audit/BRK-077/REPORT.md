# TASK-BRK-077 REPORT — FTS5 MATCH cutover + N+1 collapse (C1)

## (1) Step-0 gate log

```
c13df31 test(engine): genuine v15-shaped backup in restore drill (TASK-BRK-083, G3 green)
598bd25 test(engine): migration-replay drill at 0016 truth (TASK-BRK-082)
a323fc1 chore(release): reconcile pnpm-lock to installed tauri-api 2.12.4 (TASK-REL-008, drift-free)
d13d79f feat(engine): add fts search index and consistency check (TASK-BRK-076)
0bc4e99 docs(release): TASK-REL-007 delivery docs refreshed to as-built
```

`TASK-BRK-076` present → gate PASSED, proceeded.

## Fix one-liners (IN-limit note)

1. Search body: retired LIKE-chain → single `products_fts MATCH ?` (per-token
   `"tok"*` ANDed over `FTS_TEXT_COLUMNS` 5 norm columns, bound param,
   alphanumeric-only parts ⇒ parser-safe); barcode-exact fast path kept.
2. C1 fast-path fix: OR-across-`LEFT JOIN` (multi-second scan at 80k) →
   `UNION` of two indexed point-seeks (`idx_products_barcode`,
   `idx_product_units_barcode`). Same recall, no join scan.
3. C1 hydration collapse: per-id `fetch_product` N+1 loop (∼200 point queries
   per 100-hit search) → `fetch_products_ordered`: products
   `WHERE id IN (…)` + units `WHERE product_id IN (…)`, rank order preserved,
   missing rows skipped. IN-list chunked at **250 ids** — far below the
   SQLite variable limit (999 legacy / 32766 modern) — so the 500-row
   discovery branch stays safe.
4. C1 ranking: BM25-only `ORDER BY bm25(products_fts), p.sku` (deterministic
   SKU tie-break); retired the SQL-side `baraka_search_norm(...) LIKE` CASE
   tiers (per-row UDF over the matched set = hundreds of ms at 80k, debug).
   Deleted now-unused `escape_like_pattern` / `arabic_tolerant_like_pattern`.

## (2) Parity / engine lines

- `catalog_search_fts_parity` 8/8 green (20/20 prefix queries set-equal vs
  retired LIKE across inactive modes; `oul`→empty + `vis`/`فيس` FTS=125 vs
  legacy=250 pinned as accepted infix delta; barcode-exact=1 row; alias
  `دريل` recall gain; hostile tokens clean; write-cycle consistency green).
- `catalog_search_engine` 6/6, `catalog_search_enhancements` 7/7 green —
  rank pins (exact-first) hold under BM25-only ordering.
- Opaque `Port("catalog store failure")` did NOT resurface in any run.

## (3) 80k TIMINGS vs 50ms p95 (debug, i5-3450, same box, same seed procedure)

Run 1 (LIKE retired path replaced, pre-UNION): q1–q5 seconds-scale
(q5-barcode itself ∼1.5s via the join scan), q6 ∼69ms.
Run 2 (UNION fast path): q4 1.65 + q5 0.27 UNDER; q1 243.90 / q2 411.76 /
q3 271.71 / q6 77.13 OVER — residual = N+1 hydration + scalar tiers.
Run 3 (C1 hydration + BM25-only ranking, FINAL — full table in TIMINGS.md):
q1 vis 304.88/365.68 OVER; q2 فيس 297.46/399.49 OVER; q3 boulon
299.15/400.33 OVER; q4 oul 1.19/6.41 UNDER; q5 barcode 0.28/0.61 UNDER;
q6 cadenas 40mm 101.29/152.48 OVER (p50/p95 ms). Seed 6338.67s (box loaded;
run-2→3 confounded ±50%). Bar NOT met — see stage model in TIMINGS.md.
Run 4 (C2 release profile, idle box, DECISIVE — full table in TIMINGS.md):
seed 4147.19s; q1 vis 285.42/391.96 OVER; q2 فيس 281.49/336.09 OVER;
q3 boulon 267.72/351.34 OVER; q4 oul 0.38/1.31 UNDER; q5 barcode 0.13/0.39
UNDER; q6 cadenas 40mm 51.86/75.69 OVER (p50/p95 ms). EXPLAIN identical
4-row FTS-indexed shape. Bar NOT met — stage model confirmed at release
speed (MATCH ∼1ms, 1-row hydration sub-ms; residual = temp-btree bm25
sort over matched set + 100-row hydration).

## (4) EXPLAIN one-liner

`SCAN products_fts VIRTUAL TABLE INDEX 0:M6` +
`SEARCH p USING INDEX sqlite_autoindex_products_1 (id=?)` (+ TEMP B-TREE for
DISTINCT/ORDER BY) — FTS-indexed, SCAN-free on base tables. Run-3 re-check:
identical 4 plan rows on the 3rd seeded 80k DB (see TIMINGS.md).

## (5) Gates

- G1 `cargo fmt --check`: exit 0
- G2 `cargo clippy --all-targets --all-features -- -D warnings`: exit 0
  (one `vec_init_then_push` fixed at sqlite_catalog_repository.rs:944)
- G3 `cargo test --workspace -j 2 -- --skip search_80k_perf_probe` (C2 re-run
  2026-10-22, box idle after run-4, `audit/BRK-077/g3-c2.log`, EXIT=0):
  64 suites / 491 passed / 0 failed — confirms C1's in-flight numbers;
  the #[ignore]d 80k case stays out (0 references in the log); mid-sweep red in `catalog.rs:369` was the accepted infix
  delta, pin updated red-before-green)
- G4–G7: N/A — backend-only change; no `src/**`, IPC, or catalog Vitest pins
  touched (frontend search contract unchanged; behavior identical on covered queries).
- TIMINGS location: audit/BRK-077/TIMINGS.md (new; BRK-075 file untouched).
- `/tmp/parity_table.txt` harvest note: deleted (harvested 40/40 set-equal
  before deletion).

## (6) STATE / DONE / REMAINING (C2 closed)

STATE blocked/stage-capped. DONE: decisive release-profile run-4 on idle
box (seed 4147.19s; q1 285.42/391.96, q2 281.49/336.09, q3 267.72/351.34,
q4 0.38/1.31, q5 0.13/0.39, q6 51.86/75.69 p50/p95 ms; EXPLAIN identical
FTS-indexed 4-row shape; EXIT=0) — bar unmet on q1/q2/q3/q6, stage model
confirmed at release speed; G3 re-run green after run-4 (64 suites /
491 passed / 0 failed, EXIT=0, `audit/BRK-077/g3-c2.log`); no `[FILL]`
markers ever present in REPORT (verified by search); `/tmp/parity_table.txt`
re-verified absent (already harvested 40/40 set-equal in C1); run-4 table
appended to TIMINGS.md. REMAINING: Architect decides bar closure vs
redesign (per-stage MATCH-vs-hydration split and any further tuning are
new scope, not attempted). Scope flag (unchanged): diff touches 3 test
files (parity new + perf pins + `catalog.rs:369` infix pin), not 2 — the
`catalog.rs` pin was mandatory for G3 green under the accepted infix
delta; waiver requested. No commit anywhere.
NEXT READY: NO.
