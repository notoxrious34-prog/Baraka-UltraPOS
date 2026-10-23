# TASK-BRK-087 REPORT — Gated timing: debug 20k gate → SKIP release

Stop-ruled, no tuning. NO COMMIT. No code changes in final tree.

## (0) Method + cited deviations

- Gate: debug, idle box (IDLE-CLEAN at launch, no concurrent cargo),
  background + log `audit/BRK-087/gate20k.log`, GATE-EXIT=0, same 6
  queries × 30 iters + 3 warmup on a 20k use-case seed (spec §3).
- Deviation (cited per §3): the committed probe hardcodes 80k/50/5, so a
  TEMPORARY env-param patch (`BRK087_N/ITERS/WARMUP`, barcode-witness wrap
  — same shape as the BRK-085 allowance) enabled the specified 20k/30/3
  gate. Created, run once, then REVERTED: probe file is byte-identical to
  committed HEAD (`git diff` empty, zero `BRK087` strings — §4). Final-tree
  diff limited to `audit/BRK-087/`. Production sources never touched.

## (1) Gate numbers + verdict: FAIL — release SKIPPED

20k seed wall time: **396.76s** (BRK-085 baseline 305.88s, +30% — triggers
now write two extra STORED columns).

| query | p50 | p95 | max | hits | gate (≤20ms) |
|---|---|---|---|---|---|
| q1 vis | 68.96 | 97.77 | 97.88 | 100 | FAIL |
| q2 فيس | 73.05 | 130.80 | 144.92 | 100 | FAIL |
| q3 boulon | 69.11 | 95.98 | 124.23 | 100 | FAIL |
| q4 oul | 1.40 | 4.64 | 7.82 | 0 | pass |
| q5 barcode | 0.27 | 0.49 | 0.58 | 1 | pass |
| q6 cadenas 40mm | 40.70 | 95.49 | 99.61 | 84 | n/a (gate on q1–q3) |

Gate rule: PASS iff q1 AND q2 AND q3 p95 ≤ 20ms. Actual: 97.77 / 130.80 /
95.98 — **GATE FAIL by ~5–6.5×**. Per handoff: STOP immediately, NO
release run, no tuning. (q2 p95 carries outlier-iter noise as in BRK-085,
but p50s alone 69–73ms already fail the gate 3× over.)

## (2) Release table: SKIP (documented)

No release 80k run executed — not gated. C2 release baselines stand:
q1/q2/q3/q6 p95 = 391.96/336.09/351.34/75.69ms (all OVER 50ms).

## (3) Per-query verdicts vs 50ms bar (gate table, informational)

q1 OVER, q2 OVER, q3 OVER, q4 UNDER (4.64), q5 UNDER (0.49), q6 OVER
(95.49). Bar NOT met at 20k debug, let alone 80k release.

## (4) Key observation (no fix applied)

Fts-local ranking made 20k SLOWER than the BRK-085 joined version it
replaced (q1 p50 39.09→68.96, p95 41.61→97.77; q3 40.73→69.11). Working
hypothesis only: the `products_fts.is_active = 1` non-MATCH predicate on
an UNINDEXED column forces a full-mirror scan per query where the joined
version rode the MATCH-driven row set + PK lookups (EXPLAIN shape was
SCAN-free in both, but row volume differs). Deciding this needs a
diagnostic (e.g. is_active folded into the MATCH vs stored-flag scan
measurement) — explicitly out of scope here.

## (5) G1-G3 on final tree

- G1 `cargo fmt --check -p baraka-infrastructure`: exit 0 (post-revert).
- G2/G3: cite BRK-086 — final tree is byte-identical to its G3 tree
  (temp patch fully reverted, only `audit/BRK-087/` added): clippy
  `-D warnings` exit 0; G3 64 suites ok, 491 passed, 0 failed.
- G4-G7 N/A backend-only (no frontend files touched — state reason).

## (6) STATE / DONE / REMAINING

STATE blocked/stage-capped with numbers (gate FAIL, no tuning per rule).
DONE: honest 20k gate (GATE-EXIT=0, full table + seed time), temp patch
reverted with byte-identity proof, release correctly SKIPPED, raw log
kept. REMAINING: Architect decides — bar closure vs ADR tier-restatement
vs a new diagnostic task on the is_active-predicate hypothesis. No commit.
NEXT READY: YES.
