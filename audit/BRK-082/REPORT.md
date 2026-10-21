# TASK-BRK-082 REPORT — migration-replay drill updated to the 0016 truth

## FAIL-before (quoted, G3 worktree-wide red)

```
test downgrade_guard_probe_future_version_is_deterministic ... FAILED
test migration_replay_preserves_legacy_business_data ... FAILED
thread '...' panicked at crates\infrastructure\tests\migration_replay_and_data_preservation.rs:462:5:
assertion `left == right` failed: no products_fts table exists in this schema line
  left: 1
 right: 0
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.80s
```

Root cause (architect-confirmed): d13d79f landed migration 0016, so the
official upgrade path now creates `products_fts`; the drill still documented
scope 0001→0015 with `fts == 0`. Stale expectation, engine intact.

## Change (one test file only)

- Module docs: scope 0001→0016; deleted the false "no products_fts anywhere"
  deviation; Stage 2 documents upgrade through 0016.
- `assert_parity`: bookkeeping scope `1..=16`, count 16, max 16 (checksum loop
  over `MIGRATIONS` unchanged — now covers 16 entries).
- Replaced `fts == 0` with: `products_fts` count == 1 + MAR-001
  `search_norm_fr`/`search_norm_ar` non-empty (legacy-row backfill proof) +
  kept byte-identical `name_fr == "Marteau 500g"` readability.
- Replay test: `verify_fts_consistency()` on a fresh connection to the
  UPGRADED drill DB (no opaque `Port` error — did not resurface).
- Stage-4 probe: `1..=16` scope comment, idempotency total 16→17.
- All Stage-3 value assertions untouched (debt 450000, stock, ledger, BL,
  0014 nullability, 0015 tiers, prices).

## New assertion lines + spot-check

```
test downgrade_guard_probe_future_version_is_deterministic ... ok
test migration_replay_preserves_legacy_business_data ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.58s
```

## Gates

- G1 `cargo fmt --check`: exit 0.
- G2 `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.
- G3 `cargo test --workspace -j 2`: NOT fully green — one remaining red,
  proven pre-existing on pristine HEAD (reproduced with my file stashed):
  `restore_tests::restore_older_schema_backup_is_migrated`
  (`restore_tests.rs:758`) → `SwapFailedAndRolledBack { detail:
  "activation migration failure: sqlite failure: duplicate column name:
  search_norm_fr" }`. Restore-flow × 0016 interaction, BRK-076 territory,
  outside this task's one-file scope (fixing it = production-code change).
  Everything else in the workspace passes.

## STATE / DONE / REMAINING

STATE done-with-carry. DONE: drill file updated, both tests green,
consistency proven on upgraded legacy-seeded DB, G1/G2 clean.
REMAINING: `restore_older_schema_backup_is_migrated` red (BRK-076-owned,
quoted above) — G3-green needs that file, not this one. NEXT READY: NO
(blocked on restore×0016, not on the drill).
