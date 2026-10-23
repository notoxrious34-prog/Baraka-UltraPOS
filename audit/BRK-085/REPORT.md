# TASK-BRK-085 REPORT — Search scaling curve 1k→20k + FTS-vs-total stage split

Diagnostic only. Zero production changes (prod body frozen — see §5).
Decides bar closure (ADR) vs targeted micro-fix. NO FIX APPLIED.

## (0) Method + deviations (all temporary, all reverted — §5)

- Temp env-param patch to `crates/infrastructure/tests/search_perf_80k.rs`
  ONLY: `BRK085_N` (default 80000), `BRK085_ITERS` (default 50),
  `BRK085_WARMUP` (default 5), `BRK085_SPLIT=1` (raw fts-only timing block).
  Same use-case seed path, same 6 queries, same assertions; barcode witness
  wrapped into range (`42424 % N`) with parameterized sku assert so q5 stays
  exact at every N; seed progress printed at quarters. Patch REVERTED after
  the matrix (probe file byte-identical — §5). No temp test file was needed
  (the "ONE temporary test" allowance was not used; nothing to delete).
- Matrix (debug, idle box, background, single log `audit/BRK-085/matrix.log`,
  `MATRIX-EXIT=0`, no concurrent cargo, `tasklist` idle at launch):
  `BRK085_ITERS=30 BRK085_WARMUP=3` for all N; N = 1000, 5000, 10000, 20000;
  split enabled on the 20k run only.
- Documented deviations from run-3 comparability: 30/3 iters instead of
  50/5 (spec-mandated — p95 over 30 is noisier; the 5k q1 p95 below shows
  it); seed DB is per-run tempdir (same as all runs).
- Box: same i5-3450, `CARGO_BUILD_JOBS=2`. Binaries: same post-BRK-084 code
  for all four runs (single debug compile, warm).

## (1) Scaling table (debug, 30 iters + 3 warmup)

Seed wall time per N: 1k **11.03s** · 5k **50.46s** · 10k **156.72s** ·
20k **305.88s** (per-1k: 11.0 / 10.1 / 15.7 / 15.3 — steepens with N;
log-log slope 1k→20k ≈ **1.11**, slightly superlinear).

| N | q1 p50/p95 | q2 p50/p95 | q3 p50/p95 | q4 p95 | q5 p95 | q6 p50/p95 | q1 hits |
|---|-----------|-----------|-----------|--------|--------|-----------|---------|
| 1k | 5.79 / 7.84 | 5.17 / 6.54 | 6.71 / 8.54 | 2.60 | 0.45 | 3.27 / 5.27 | 42 |
| 5k | 17.48 / 60.49* | 16.89 / 26.93 | 21.19 / 25.07 | 4.57 | 0.97 | 5.45 / 11.26 | 100 |
| 10k | 39.39 / 48.38 | 38.54 / 53.05 | 38.48 / 61.41 | 4.29 | 1.53 | 10.38 / 14.38 | 100 |
| 20k | 39.09 / 41.61 | 38.12 / 41.89 | 40.73 / 42.26 | 1.97 | 0.60 | 15.99 / 24.15 | 100 |

(ms; verdicts vs 50ms: only 5k-q1, 10k-q2, 10k-q3 OVER — all else UNDER.)
\* 5k q1 p95 (60.49, max 63.39) is outlier iters against a 17.48 p50 —
30-iter p95 noise, reported honestly; slopes below use p50.

Reference anchors (debug): BRK-084 80k q1–q3 p50 ~128–132 / p95 132–135,
q6 40.41/43.27.

## (2) Stage split at 20k (q1 vis)

- (a) raw fts-only subquery (`SELECT rowid … MATCH … ORDER BY bm25 LIMIT
  100`, no products join, no hydration): p50 **2.76** / p95 **4.73** ms.
- (b) full `repo.search` (same MATCH): p50 **39.09** / p95 **41.61** ms.
- Residual (b)−(a): **≈36.3 / 36.9 ms** — attributed to per-match products
  PK lookups (sku + is_active) + temp-btree sort over the matched set +
  100-row hydration (2 IN queries). Finer split (sort vs hydration) needs
  more probes — not attempted (out of scope).

## (3) Slope verdict + cheapest-lever hypothesis (no fix)

- Log-log slope 1k→20k (p50): q1 **0.64**, q2 **0.67**, q3 **0.60**,
  q6 **0.53** — sublinear; q4/q5 flat (~1–4ms / ~0.3–1.5ms, N-invariant).
  Linearized: q1–q3 ≈ **1.75ms per 1k rows**, q6 ≈ 0.67ms/1k.
- But 20k→80k (BRK-084 debug p50) q1 slope ≈ **0.87** — near-linear once the
  matched set is large. Two-regime picture: at ≤20k a ~35ms constant base
  (100-row hydration + fixed costs) dominates and growth looks flat
  (q1 p50 10k→20k: 39.39 → 39.09); toward 80k the matched-set sort takes
  over (80k residual ≈ 130 − ~5 ≈ 125ms).
- Dominant stage at the 80k bar: sort-over-matched-set + hydration
  (FTS MATCH itself is ~5ms even at 20k and q4 proves ~1ms MATCH-only).
- Cheapest-lever HYPOTHESIS (not applied): mirror `sku` + `is_active` as
  STORED (non-UNINDEXED) FTS columns so the ranking subquery goes fully
  fts-local (kills ~thousands of per-match PK lookups and narrows the sort
  input); second candidate is capping the matched set pre-sort. Both are
  new scope — Architect decides (bar ADR vs micro-fix task).

## (4) Temp-artifact deletion proof

- No `zz_*`/temp test file was ever created for this task (matrix ran
  through the reverted env-param patch — allowance unused).
- Probe patch fully reverted: `git diff -- crates/infrastructure/tests/search_perf_80k.rs`
  prints nothing (verified post-revert); `ls …/tests | grep -c zz` → 0.
- Stray check: no cargo/rustc/search_perf processes left (`tasklist` clean).

## (5) Gates + diff discipline

- G1 `cargo fmt --check -p baraka-infrastructure`: exit 0 (after `cargo fmt`
  on the temp patch; final tree re-checked clean).
- Targeted suites with temp patch applied (prod body untouched throughout):
  parity **8/8**, engine **6/6**, enhancements **7/7** — all green.
- Full G3 NOT re-run: zero production change since BRK-084's 64/491/0
  (stated reason per acceptance (e)); lib hash unchanged
  (`search_perf_80k-eb2112a1c…` across all runs).
- Diff: probe file CLEAN (reverted); prod body, migrations, triggers,
  frontend, ipc.rs, configs untouched. Only `audit/BRK-085/` added
  (`REPORT.md`, `matrix.log`). No commit.

## (6) STATE / DONE / REMAINING

STATE done (diagnostic complete, no bar verdict claimed — that is the
Architect's ADR call). DONE: 4-point scaling table + seed times,
20k stage split (fts-only 2.76/4.73 vs full 39.09/41.61ms), log-log slopes
+ two-regime verdict + cheapest-lever hypothesis, temp patch reverted with
byte-identity proof, raw `matrix.log` kept as evidence. REMAINING: Architect
decides bar closure vs micro-fix task. No commit anywhere.
NEXT READY: YES.
