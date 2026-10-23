# TASK-BRK-084 REPORT — Rank-then-hydrate FTS subquery (bounded bar-breaker)

## (0) Step-0 gate log

```
9b6ac07 docs(architecture): persist CTP-SESSION-ARC-027 closing SESSION-ARC-026 (bar decision pending C2)
c13df31 test(engine): genuine v15-shaped backup in restore drill (TASK-BRK-083, G3 green)
598bd25 test(engine): migration-replay drill at 0016 truth (TASK-BRK-082)
a323fc1 chore(reconcile): pnpm-lock to installed tauri-api 2.12.4 (REL-008)
d13d79f feat(engine): add fts search index + consistency check (TASK-BRK-076)
```

Box idle at launch (native `tasklist`: no cargo/rustc/test processes; MSYS
`ps` is blind to native Windows procs — verified via tasklist instead).
Query/hydration code touched ONLY inside `search()` FTS branch
(`sqlite_catalog_repository.rs:927-960`); MATCH text, ranking definition
(bm25+sku), LIMIT values, UNION barcode fast path, empty-query branch,
migrations, triggers, frontend, ipc.rs, configs all untouched. No new
dependencies. Live DB never touched (tempdir scratch only).

## (1) The change (one branch, same contract)

Retired C1 shape: `SELECT DISTINCT p.id … JOIN … WHERE MATCH … ORDER BY
bm25, p.sku LIMIT ?` — sort + DISTINCT over the joined row set, LIMIT
discards the rest. New shape: inner subquery ranks NARROW rows
(`product_id + bm25 rank + p0.sku` via PK point-lookup join) with
`ORDER BY rank, sku LIMIT ?2`; middle layer materializes
`ROW_NUMBER() OVER (ORDER BY rank, sku) AS seq`; outer query returns
`product_id ORDER BY seq` (order re-applied, never re-sorted). sku tie-break
preserved inside the subquery because sku lives in `products`, not in FTS
(spec §1 — stated shape: narrow `rowid+rank+sku` join). 500-row discovery
branch: same pattern, parameterized LIMIT unchanged (spec §2). IN-chunking
in `fetch_products_ordered` retained untouched (spec §3 — ≤100 ids now
single-chunk by construction). DISTINCT dropped: 0016 triggers maintain
exactly one FTS row per product (DELETE+INSERT on every write path).

## (2) EXPLAIN (new shape — captured pre-seed via temp probe, file deleted)

Temp test `zz_tmp_explain_084` ran the EXPLAIN of the byte-identical new SQL
on a migrated scratch DB, printed the plan, and was deleted before G3
(`NO_LEFTOVERS`; final tree has no trace — verified `grep -c zz_tmp` → 0).
G1/G2/G3 below ran on the final tree after deletion.

```
EXPLAIN[active]:
  2|0|0|CO-ROUTINE ranked
  5|2|0|CO-ROUTINE (subquery-4)
  7|5|0|CO-ROUTINE inner_ranked
  15|7|0|SCAN products_fts VIRTUAL TABLE INDEX 0:M6
  20|7|0|SEARCH p0 USING INDEX sqlite_autoindex_products_1 (id=?)
  40|7|0|USE TEMP B-TREE FOR ORDER BY
  49|5|0|SCAN inner_ranked
  58|5|0|USE TEMP B-TREE FOR ORDER BY
  73|2|0|SCAN (subquery-4)
  111|0|0|SCAN ranked
  119|0|0|USE TEMP B-TREE FOR ORDER BY
```

(`all`/inactive-off variant identical modulo the missing filter.)
Verdict: FTS subquery ✓ (`SCAN products_fts VIRTUAL TABLE INDEX 0:M6` =
FTS index scan, same row kind as C1); only bounded join is the `p0` PK
point-lookup ✓; NO `SCAN` of any base table (remaining SCAN lines address
co-routine/subquery outputs) ✓. Three TEMP B-TREEs: one over the matched
narrow set, two over ≤100 rows.

Staleness note (honest): `search_perf_80k.rs:290-293` hardcodes the C1 SQL
for its EXPLAIN block, so `audit/BRK-084/debug80k.log` lines 20-25 print the
OLD plan text (its asserts still pass; the string is out of task scope —
probe file untouched per diff limit). The table below, not that block, is
the new-code evidence (probe measures the REAL repo `search`).

## (3) Parity / engine lines (zero pin changes)

- `catalog_search_fts_parity` **8/8** green (153.25s) — recall/order contract holds.
- `catalog_search_engine` **6/6** green (3.14s) — exact-first rank pins hold.
- `catalog_search_enhancements` **7/7** green (2.91s).
- No expected-set touched; any recall/order change would have been STOP, none occurred.

## (4) Step-1 gate — DEBUG 80k, idle box (DECISIVE, release NOT run)

Command: `export CARGO_BUILD_JOBS=2 && cargo test -p baraka-infrastructure
--test search_perf_80k -- --ignored --nocapture > audit/BRK-084/debug80k.log
2>&1` (background `proc_b34a5679e6cc`, binary
`search_perf_80k-eb2112a1cf9f6f4a.exe` — new hash, post-edit compile).
Seed 6658.45s (legs/20k: 451.98 / 1254.80 / 2336.12 / 2615.56s), 50+5
iterations, EXIT=0, finished 6683.73s.

| query | p50_ms | p95_ms | max_ms | hits | vs 50ms | run-3 p95 | ratio |
|-------|--------|--------|--------|------|---------|-----------|-------|
| q1 vis | 130.05 | 132.15 | 207.98 | 100 | OVER | 365.68 | 2.77× |
| q2 فيس | 127.93 | 131.55 | 201.66 | 100 | OVER | 399.49 | 3.04× |
| q3 boulon | 131.75 | 135.20 | 171.72 | 100 | OVER | 400.33 | 2.96× |
| q4 oul | 1.07 | 1.80 | 1.87 | 0 | UNDER | 6.41 | 3.56× |
| q5 barcode | 0.26 | 0.32 | 0.35 | 1 | UNDER | 0.61 | 1.91× |
| q6 cadenas 40mm | 40.41 | 43.27 | 78.06 | 100 | UNDER | 152.48 | 3.52× |

Gate (both required): (i) debug p95 <200ms on every query → PASS (max
135.20); (ii) ≥3× improvement vs run-3 on ALL six → **FAIL** (q1 2.77×,
q3 2.96×, q5 1.91×). q5 is the untouched UNION barcode fast path —
sub-millisecond both runs, ratio is pure noise by construction; q1/q3 miss
by 0.23×/0.04× (note: run-3 itself was box-loaded, i.e. the baseline was
inflated, making ≥3× easier — still missed). Verdict rule executed, not
renegotiated: **Step-2 release run SKIPPED, STOP with evidence.**

## (5) Gates on final tree

- G1 `cargo fmt --check -p baraka-infrastructure`: exit 0.
- G2 `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.
- G3 `cargo test --workspace -j 2 -- --skip search_80k_perf_probe`
  (`audit/BRK-084/g3.log`, EXIT=0): **64 suites / 491 passed / 0 failed**.
- G4–G7: N/A — backend-only change; no `src/**`, IPC, or catalog Vitest pins
  touched (frontend search contract unchanged).
- No opaque `Port("catalog store failure")` in any run.

## (6) STATE / DONE / REMAINING

STATE blocked/stage-capped. DONE: rank-then-hydrate subquery (narrow MATCH
→ LIMIT → seq-ordered hydrate, UNION + empty-query branches untouched);
EXPLAIN proves FTS-subquery + bounded PK joins, no base SCAN; parity 8/8 +
engine 6/6 + enhancements 7/7 with zero pin changes; G1/G2/G3 green
(64/491/0); Step-1 debug 80k on idle box (6658.45s seed, EXIT=0) with
honest gate computation (q6 now UNDER at 43.27ms p95; q1/q2/q3 at
132–135ms p95, ~3× better but bar still missed). REMAINING: bar decision
belongs to the Architect — options are ADR-101 renegotiation (e.g. lower
the bar, accept ~135ms debug/~390ms release p95 on multi-thousand-row
MATCH sets, or bound the matched set) or a new-scope redesign (NOT
attempted — no tuning beyond the prescribed shape). No commit anywhere.
NEXT READY: NO.
