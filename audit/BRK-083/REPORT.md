# TASK-BRK-083 REPORT — restore test's fake-older backup genuinely v15-shaped

## Root cause (architect-diagnosed, verified live)

`restore_older_schema_backup_is_migrated` fakes an older backup by deleting
the latest `schema_migrations` row + (if >= 15) the v15 columns. With 0016
landed, the "older" backup still carried v16 artifacts, so activation replay
of 0016 died on `ADD COLUMN search_norm_fr` (already present). Real v15
backups never carry v16 columns — hence fail-closed stays and no production
code changes: the drill, not the engine, was wrong.

## FAIL-before (quoted, `restore_tests.rs:758`)

```
thread 'restore_older_schema_backup_is_migrated' (...) panicked at crates\infrastructure\tests\restore_tests.rs:758:40:
restore: SwapFailedAndRolledBack { detail: "activation migration failure: sqlite failure: duplicate column name: search_norm_fr" }
test result: FAILED. 0 passed; 1 failed; ... 13 filtered out
```

## Fix (test-only, one block, order mandatory)

New `if app_version >= 16` block after the v15 block: (1) DROP the 8 FTS
triggers (names verified live in `0016_fts_search_index.sql:55-233`:
trg_products_fts_{ai,ad,au}, trg_units_fts_{ai,au,ad},
trg_aliases_fts_{ai,ad}) — first, because DROP COLUMN errors while a trigger
references the table; (2) DROP TABLE products_fts (FTS5 shadows go with it);
(3) DROP TABLE product_aliases (+ its index); (4) DROP COLUMN search_norm_fr /
search_norm_ar. Unconditional DROPs mirror the v15 block: the harness DB is
built at the current version, so `>= 16` already implies presence; a genuine
older backup never carries them. Hash re-pointing + all assertions identical.
No table-rebuild needed — DROP COLUMN succeeded on the harness SQLite.

Fail-closed rationale: restore production code deliberately untouched. A real
older backup genuinely lacks v16 artifacts, so activation replay of 0016
succeeds; a tampered backup carrying extra columns still fails closed with
`SwapFailedAndRolledBack` instead of silently half-migrating. The drill now
proves the honest path.

## Gates

- New block: `crates/infrastructure/tests/restore_tests.rs` (~+30 lines,
  `if app_version >= 16` after the v15 block).
- `restore_older_schema_backup_is_migrated` → ok (1.09s).
- G1 `cargo fmt --check`: exit 0. G2 clippy `--all-targets --all-features
  -- -D warnings`: exit 0.
- G3 `cargo test --workspace -j 2`: FULLY GREEN — 63 suites, 483 passed,
  0 failed. The 80k case stays out via `#[ignore]`
  (`fts_migration_tests.rs:397`, `search_perf_80k.rs:9`).

## STATE / DONE / REMAINING

STATE done. DONE: v15-shaped backup, restore test green, G3 fully green,
G1/G2 clean, REPORT written. REMAINING: none — G3-green unblocks BRK-077 →
packaging. NEXT READY: YES.
