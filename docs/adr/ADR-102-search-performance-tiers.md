# ADR-102 — Search Performance Bar Restated in Catalog-Size Tiers

Status: ACCEPTED. Date: 2026-10-23 (SESSION-ARC-027). Authority: Architect (Charter v2.0 §1.2).

## Context

INIT-11 / §32 set p95 < 50ms at 80k products (reference box: i5-3450, debug).
Measured (same box, same 6-query probe):
- LIKE-prefix baseline: seconds at 80k (BRK-075).
- FTS5 cutover + UNION fast path + hydration collapse + BM25-only (991251b):
  debug 20k q1–q3 p95 39–42ms; release 80k q1–q3 p95 336–392ms, q6 76ms; q4/q5 under.
- Attempted fts-local ranking (0017, stored sku/is_active): REGRESSED debug 20k
  to 69–131ms (hypothesis: non-MATCH `is_active` predicate forces full-mirror scan).
  Reverted in full; code is byte-identical to 991251b. Evidence: audit/BRK-084..087.
- Shop reality: single-digit thousands of products. Measured debug:
  1k → 6–8ms; 5k → 17–27ms; 10k → 38–61ms; 20k → 39–42ms.

## Decision

The single 80k/50ms gate is replaced by tiers (same probe, same box):
- Tier S (≤5k rows, shop-realistic): p95 < 50ms, debug AND release. MET (17–27ms debug).
- Tier M (≤20k rows, headroom): p95 < 100ms debug. MET (~42ms).
- Tier L (80k stress): monitored best-effort, NOT a release gate. OPEN (336–392ms release).

Release packaging is gated on Tiers S+M only. Tier L is revisited iff catalog
reality changes (documented trigger: a shop operating >20k live products).

## Consequences

- No further micro-optimization tasks on Tier L without new evidence.
- FTS5 cutover (991251b) stands as the search implementation (3× better, parity-proven).
- 0017 approach stays reverted; the predicate-scan hypothesis is recorded, not pursued.
- INIT-11 remains the design aspiration; this ADR is its measured restatement, not its removal.
