# TASK-BRK-086 REPORT — FTS-local ranking columns (0017)

Bounded bar-breaker prep: ranking subquery 100% fts-local, no timing runs.
NO FIX beyond scope. DO NOT COMMIT.

## (0) Method

- New migration `crates/infrastructure/migrations/0017_fts_rank_local_columns.sql`
  + registration in `persistence/migration.rs` `MIGRATIONS`.
- Repository ranking subquery rewritten fts-local; `verify_fts_consistency()`
  extended to `sku_raw` + `is_active`; replay drill + restore drill updated
  for latest version 17; one 0016-apparatus test renamed/updated
  (`upgrade_path_applies_only_fts_migrations`, count 17).
- Temp EXPLAIN/backfill probe created, run green, then DELETED (proof §4).

## (1) Table kind + DDL method + why

- Table kind: content-mode (fully stored) FTS5 `products_fts`, synced by
  DELETE + re-INSERT triggers (NOT external-content, NOT contentless).
- Method: REBUILD (DROP TRIGGERs → DROP TABLE → CREATE VIRTUAL TABLE with
  two extra columns → recreate 8 triggers with new column lists → INSERT
  backfill from relational truth), per the 0014 precedent. Chosen over
  ALTER TABLE ... ADD COLUMN so the migration depends on no FTS5-ALTER
  support for this table kind on any SQLite; the whole file runs inside the
  runner's single transaction (like 0016): any failure rolls back to 0016,
  no partial FTS state possible. Backfill uses the SAME scalar +
  aggregates as the triggers, so fresh and upgraded DBs converge.
- New columns (both UNINDEXED = stored, readable, never tokenized):
  `sku_raw` (raw `products.sku` — tie-break must match ORDER BY p.sku
  byte-identically; normalized norm_sku would sort differently),
  `is_active` (raw 0/1 flag for the active filter).
- Trigger names UNCHANGED (same 8 as 0016): the restore-drill teardown that
  drops the whole FTS apparatus covers v17 implicitly (documented in the
  drill comment; no separate v17 teardown by design — §3).

## (2) EXPLAIN (temp probe, fresh 1→17 DB, then deleted)

MAX(version)=17; `pragma_table_info` shows `sku_raw` + `is_active`;
trigger smoke: seeded product mirrors `sku_raw='T086-001'`, `is_active=1`.
Both ranking variants plan fts-local (representative, active variant):

```
EXPLAIN-active 11|7|0|SCAN products_fts VIRTUAL TABLE INDEX 0:M8
EXPLAIN-active 30|7|0|USE TEMP B-TREE FOR ORDER BY
```

Full plans (10 rows each): CO-ROUTINE ranked/inner_ranked scaffolding,
single `SCAN products_fts VIRTUAL TABLE INDEX 0:M8`, TEMP B-TREE only for
the narrow (rank, sku) ORDER BY. NO `SEARCH p`, NO `SCAN products` — zero
`products` join before LIMIT in either variant (incl-inactive identical
minus the predicate). Fts-local proven.

## (3) Drill lines

- `fts_migration_tests`: 8 passed, 0 failed, 1 ignored (slow 80k seed).
- `migration_replay_and_data_preservation`: 2 passed, 0 failed —
  legacy v13 → 17 upgrade, 17/17 bookkeeping rows, 0017 shape asserts
  (`sku_raw`/`is_active` present + MAR-001 mirror `('MAR-001', 1)`),
  extended consistency green on the upgraded scratch DB (legacy rows incl).
- `restore_tests`: 14 passed, 0 failed — after ONE honest fix: the
  older-schema teardown drops the whole FTS apparatus (incl.
  `product_aliases`) while keeping the v16 bookkeeping row, so restore
  replays ONLY 0017 on that shape and its backfill hit `no such table:
  product_aliases`. Fix (in 0017, matching 0016's idempotent idiom):
  `CREATE TABLE IF NOT EXISTS product_aliases` + index at the top of the
  rebuild — no-op on genuine v16 DBs. Raw log: `audit/BRK-086/restore.log`.
  Note: on the torn-down drill shape the alias seed (a v16-runner step)
  does not re-run, so backfilled `norm_aliases` is empty there; on a
  genuine v16→17 upgrade the seeded aliases backfill normally.
- Downgrade probe (in replay file): green with 18-row (17 known + future)
  idempotence assert.

## (4) Temp-artifact deletion proof

- `zz_tmp_explain_086.rs`: created, run green (1.00s), DELETED post-capture;
  `ls tests | grep -c zz` → 0. No other ad-hoc scripts. No commit.

## (5) Gates + diff discipline

- Parity 8/8, engine 6/6, enhancements 7/7 — all green, zero pin changes
  (order contract holds with fts-local ranking).
- G1 `cargo fmt --check -p baraka-infrastructure`: exit 0 (after fmt).
- G2 `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.
- G3: full workspace (80k probe skipped): 64 suites `ok`, 491 passed,
  0 failed, G3-EXIT=0. Raw log `audit/BRK-086/g3b.log`.
- G4-G7 N/A (backend-only — no frontend files touched; state reason).
- Diff: `0017_fts_rank_local_columns.sql` [CREATED], `migration.rs`
  (0017 registration), `sqlite_catalog_repository.rs` (ranking body +
  consistency), `migration_replay_and_data_preservation.rs`,
  `restore_tests.rs` (comment only), `fts_migration_tests.rs`
  (rename + count); plus `audit/BRK-086/REPORT.md`. 0001…0016 untouched.
  MATCH text, ranking definition, LIMITs untouched. No commit.

## (6) STATE / DONE / REMAINING

STATE done. DONE: 0017 (rebuild, fts-local ranking columns, atomic,
backfill from truth) + registration, fts-local ranking subquery, consistency
extended to new columns, replay + restore drills updated and green
(restore needed the one honest fix, documented), parity/engine/
enhancements green with zero pin changes, G1/G2/G3 green (64/491/0),
temp probe created/run/deleted, raw logs kept. REMAINING: none in scope —
timing runs belong to the gated follow-up (BRK-087). No commit anywhere.
NEXT READY: YES.
