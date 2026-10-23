# BRK-077 TIMINGS — 80k catalog search probe (debug, reference box)

Box: Windows 10 Pro | Intel i5-3450 @ 3.10GHz | 16GiB RAM | SATA SSD |
`CARGO_BUILD_JOBS=2`. Same 80k seed procedure in all three runs
(`search_perf_80k --ignored`). Bar: all six p95 < 50ms.

## Run 1 — LIKE retired path replaced by FTS MATCH (pre-UNION fast path)

Seed 4156.78s.

| query   | p50_ms | p95_ms | max_ms | hits | verdict     |
|---------|--------|--------|--------|------|-------------|
| q1 vis  | 1763.50| 1868.25| 1938.81| 100  | OVER-TARGET |
| q2 فيس  | 1913.26| 2006.21| 2255.24| 100  | OVER-TARGET |
| q3 boulon| 1751.91| 1833.99| 1840.68| 100  | OVER-TARGET |
| q4 oul | 1564.74| 1864.68| 2338.52| 0    | OVER-TARGET |
| q5 barcode| 1486.89| 1503.89| 1681.31| 1   | OVER-TARGET |
| q6 cadenas 40mm | 68.69 | 70.87 | 73.38 | 100 | OVER-TARGET |

Diagnosis: every single-token query paid the barcode fast path's
OR-across-`LEFT JOIN` scan (∼1.5s at 80k — the OLD q5 bar was 1.57s, same
shape). EXPLAIN already SCAN-free on base tables (FTS index used).

## Run 2 — UNION fast path (two indexed point-seeks)

Seed 4307.55s.

| query   | p50_ms | p95_ms | max_ms | hits | verdict     |
|---------|--------|--------|--------|------|-------------|
| q1 vis  | 238.97 | 243.90 | 317.29 | 100  | OVER-TARGET |
| q2 فيس  | 405.45 | 411.76 | 461.26 | 100  | OVER-TARGET |
| q3 boulon| 241.75 | 271.71 | 368.19 | 100  | OVER-TARGET |
| q4 oul | 1.11   | 1.65   | 2.35   | 0    | UNDER-TARGET|
| q5 barcode| 0.20  | 0.27   | 0.43   | 1    | UNDER-TARGET|
| q6 cadenas 40mm | 71.27 | 77.13 | 97.76  | 100  | OVER-TARGET |

Diagnosis: residual = per-hit hydration N+1 (∼200 point queries per 100-hit
search) + SQL-side `baraka_search_norm(...) LIKE` CASE tiers evaluated over
the whole matched set (thousands of rows) + bm25/temp B-trees.

## Run 3 — C1: hydration collapse + BM25-only ranking (FINAL)

Seed 6338.67s (box under heavier load than runs 1–2; cross-run comparison
confounded ±50% — see note).

| query   | p50_ms | p95_ms | max_ms | hits | verdict     |
|---------|--------|--------|--------|------|-------------|
| q1 vis  | 304.88 | 365.68 | 511.72 | 100  | OVER-TARGET |
| q2 فيس  | 297.46 | 399.49 | 631.20 | 100  | OVER-TARGET |
| q3 boulon| 299.15 | 400.33 | 525.28 | 100  | OVER-TARGET |
| q4 oul | 1.19   | 6.41   | 16.02  | 0    | UNDER-TARGET|
| q5 barcode| 0.28  | 0.61   | 2.39   | 1    | UNDER-TARGET|
| q6 cadenas 40mm | 101.29| 152.48| 251.83 | 100  | OVER-TARGET |

Note on load confounding: run-3 seed took 47% longer than run 2 on the same
procedure (6338s vs 4307s), and every query's p50 rose ∼25–50% in lockstep —
including q5-barcode (0.20→0.28ms), whose two-index-seek shape cannot be
affected by the C1 changes. The C1 effect is therefore masked, not measured,
by box load. A load-controlled per-stage breakdown (MATCH+ORDER vs hydration,
timed separately at 80k) is the defined next step — it requires a 4th seeded
run and was NOT executed (budget boundary; Architect decides).

## Stage model (from q4/q5/q6 invariants across runs)

- q4-empty (MATCH only, no hydration, no sort rows): ∼1–6ms ⇒ FTS MATCH
  evaluation itself is millisecond-scale.
- q5-barcode (2 indexed seeks + 1-row hydration): <1ms ⇒ hydration of one
  row is sub-millisecond.
- q1/q2/q3 (MATCH over ∼3300-row sets + temp-btree bm25 sort + 100-row
  hydration): 240–410ms ⇒ residual dominated by sort-over-matched-set and
  the two hydration IN-queries at debug speed on this i5.

## Run 4 — C2: release profile, idle box (DECISIVE)

Seed 4147.19s total (legs per 20k: 221.64 / 672.18 / 1297.82 / 1955.55s —
same superlinear shape as debug runs, faster per row). Test binary
`search_perf_80k-bad58fd324a0d1ea.exe` (unchanged C1 code), 50 measured
iterations + 5 warmup per query, EXIT=0, finished in 4198.45s.
Box-idle: yes — no cargo/node/test contention during seed+queries (only
trivial `tail`/`tasklist` polls); one pre-existing idle `powershell.exe`
(not mine, untouched). Box: same i5-3450, `CARGO_BUILD_JOBS=2`.

| query   | p50_ms | p95_ms | max_ms | hits | verdict     |
|---------|--------|--------|--------|------|-------------|
| q1 vis  | 285.42 | 391.96 | 484.09 | 100  | OVER-TARGET |
| q2 فيس  | 281.49 | 336.09 | 432.41 | 100  | OVER-TARGET |
| q3 boulon| 267.72| 351.34 | 484.32 | 100  | OVER-TARGET |
| q4 oul | 0.38   | 1.31   | 4.74   | 0    | UNDER-TARGET|
| q5 barcode 80K00042424| 0.13| 0.39 | 4.44   | 1    | UNDER-TARGET|
| q6 cadenas 40mm | 51.86| 75.69| 112.23 | 100  | OVER-TARGET |

EXPLAIN (identical 4-row shape as runs 1–3): `SCAN products_fts VIRTUAL
TABLE INDEX 0:M6` + `SEARCH p USING INDEX sqlite_autoindex_products_1
(id=?)` + `USE TEMP B-TREE FOR DISTINCT` + `USE TEMP B-TREE FOR ORDER BY`
⇒ FTS-indexed, SCAN-free on base tables.

Verdict vs 50ms p95 bar: NOT MET — q1/q2/q3 p95 336–392ms (6–8× over),
q6 p95 75.69ms (p50 51.86, closest miss); q4/q5 comfortably under.
vs run 3 (debug, loaded): release improves q2 (399.49→336.09),
q3 (400.33→351.34), q6 (152.48→75.69); q1 p95 flat-to-noise
(365.68→391.96, p50 304.88→285.42). Stage model confirmed at release
speed: MATCH ∼1ms (q4), 1-row hydration sub-ms (q5) ⇒ residual is the
temp-btree bm25 sort over the matched set + 100-row hydration IN-queries.

Note: a prior C2 attempt (stale `release80k.log` @19:01, seed 20000/80000)
died with no test process left; adopted run above is a fresh relaunch of
the identical command on unchanged code.
