# TASK-BRK-075 TIMINGS — 80k-row search probe (measurement only)

## Machine header
Windows 10 Pro | Intel i5-3450 @ 3.10GHz | 16 GiB RAM (2x8) |
SATA SSD (ADATA SU650 / TEAM T253) | CARGO_BUILD_JOBS=2 | debug profile
(`cargo test`, unoptimized). Same-connection reuse, WAL, synchronous=FULL,
FK ON (connection §11 defaults). Seed: LCG `0x8E5E4D219C772B11`, 80,000
products via `CreateProductUseCase` (categories None — domain-valid),
~40% unit barcodes `80K{index:08}`, every 5th product a 2nd pack unit.
Seed wall clock: 183.46s. Scratch `tempfile::tempdir` DB only.
Known barcode row: index 42424 → `80K00042424`, SKU `PERF-42424`.

## Correctness asserts (all passed, test EXIT=0)
- q5 returns exactly 1 row with SKU `PERF-42424`.
- q2 returns non-empty with ≥1 hit whose FR title contains "vis".
- All queries deterministic hit counts across 50 iters.

## 6-query table (warm 5 + measured 50 iters, ms)

| query | iters | p50 | p95 | max | hits | verdict vs 50ms |
|---|---|---|---|---|---|---|
| q1 "vis" (FR) | 50 | 13461.61 | 14561.02 | 14661.51 | 100 | OVER-TARGET (~291x) |
| q2 "فيس" (AR) | 50 | 10548.58 | 11320.74 | 11351.68 | 100 | OVER-TARGET (~226x) |
| q3 "boulon" (FR) | 50 | 18543.98 | 18794.00 | 19081.18 | 100 | OVER-TARGET (~376x) |
| q4 "oul" (infix) | 50 | 6829.27 | 6978.48 | 7350.56 | 100 | OVER-TARGET (~140x) |
| q5 "80K00042424" (barcode-exact) | 50 | 1571.17 | 1671.94 | 1716.14 | 1 | OVER-TARGET (~33x) |
| q6 "cadenas 40mm" (multi-token) | 50 | 5199.59 | 5250.79 | 5332.51 | 100 | OVER-TARGET (~105x) |

## EXPLAIN one-liner
Shape-equivalent LIKE plan: `SCAN p USING INDEX idx_products_sku`
(full index scan — no usable index for leading-`%` LIKE) + correlated
`SEARCH pu USING INDEX idx_product_units_product`. SCAN-confirmed.

## Reading for ADR-101
Even the indexed barcode-exact path misses 50ms by 33x in a debug build
on this box (LEFT JOIN + per-hit `fetch_product` hydration dominate);
the infix hybrid misses by 2 orders of magnitude. Gap PROVEN with
numbers. Caveats: debug (not release) profile; per-hit detail hydration
included in timings; single-user, warm-cache connection.
