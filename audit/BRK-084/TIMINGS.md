# BRK-084 TIMINGS — rank-then-hydrate, DEBUG 80k, idle box (Step-1 gate run)

Box: Windows 10 Pro | Intel i5-3450 @ 3.10GHz | 16GiB RAM | SATA SSD |
`CARGO_BUILD_JOBS=2`. Binary `search_perf_80k-eb2112a1cf9f6f4a.exe`
(post-edit). Same 80k seed procedure + same 6 queries as BRK-077 runs.
Bar (unchanged): all six p95 < 50ms. Step-1 gate (Architect-set): proceed
to release ONLY if debug p95 improves ≥3× vs run-3 on ALL six AND every
debug p95 < 200ms.

## Debug run — new subquery shape

Seed 6658.45s (legs per 20k: 451.98 / 1254.80 / 2336.12 / 2615.56s).
Finished 6683.73s, EXIT=0.

| query   | p50_ms | p95_ms | max_ms | hits | verdict     | run-3 p95 | ratio |
|---------|--------|--------|--------|------|-------------|-----------|-------|
| q1 vis  | 130.05 | 132.15 | 207.98 | 100  | OVER-TARGET | 365.68    | 2.77× |
| q2 فيس  | 127.93 | 131.55 | 201.66 | 100  | OVER-TARGET | 399.49    | 3.04× |
| q3 boulon| 131.75| 135.20 | 171.72 | 100  | OVER-TARGET | 400.33    | 2.96× |
| q4 oul | 1.07   | 1.80   | 1.87   | 0    | UNDER-TARGET| 6.41      | 3.56× |
| q5 barcode 80K00042424| 0.26| 0.32 | 0.35   | 1    | UNDER-TARGET| 0.61      | 1.91× |
| q6 cadenas 40mm | 40.41| 43.27| 78.06  | 100  | UNDER-TARGET| 152.48    | 3.52× |

Gate check: <200ms on all six → PASS (max 135.20). ≥3× on all six →
FAIL (q1 2.77×, q3 2.96×, q5 1.91× — q5 is the untouched UNION fast path,
sub-millisecond both runs, ratio is noise by construction). No release run
per the gate (STOP with evidence).

Effect shape: q1/q2/q3 p95 365–400ms → 131–135ms; q6 152.48 → 43.27ms
(now UNDER the 50ms bar even in debug); q4/q5 flat (fast paths untouched).
Residual ≈ 130ms at debug speed on multi-thousand-row MATCH sets
(temp-btree sort over matched narrow rows + 100-row hydration IN-queries).

## Release run

NOT RUN — Step-1 gate not met (see above + REPORT.md §4). No release
table, no further tuning (out of scope on STOP).
