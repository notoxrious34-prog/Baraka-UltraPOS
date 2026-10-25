# TASK-BRK-094 REPORT — Real backup size + trigger (unblocks DSN-095)

Authority: DSN-095 blocked REPORT `e581cd9` §2. No schema edits to
existing migrations. No new dependencies. Live DB never touched
(tempdir file-DBs + node vitest only). DO NOT COMMIT.

## 1. Removed hardcodes

- `crates/application/src/use_cases/backup.rs:175` — `file_size_bytes: 0`
  deleted; the list path now maps `row.file_size_bytes` (line ~179).
- Same file `:177` / `:245` — `trigger_type: Manual` hardcodes deleted;
  both paths map `BackupTriggerType::from_code(&row.trigger)`.
- `formatBytes` "0 B" fallbacks deleted (both copies): unknown now
  renders "—", never 0.

## 2. Migration verdict — NEEDED, 0017, replay-proven

- The table shape accepts additive change cleanly: `backups_log` is a
  plain STRICT table, no trigger enumerates its columns (repo-wide grep:
  only `sqlite_backup_manager.rs` references it — no drill enumerates it).
- `crates/infrastructure/migrations/0017_backup_size_trigger.sql` (new):
  two NULL-able columns (`file_size_bytes INTEGER NULL`,
  `trigger TEXT NULL`), no backfill. Registered as version 17 in
  `persistence/migration.rs` (the registry is a hardcoded
  `include_str!` list — the file alone does nothing without it).
- Rollback-safe: additive NULL columns; old binaries' explicit SELECT
  lists ignore them; old INSERT lists omit them (defaults NULL).
- Replay proof: `upgrade_path_applies_only_v16` (now asserts count 17 +
  both columns via PRAGMA), `fresh_apply_registers_v16_with_checksum`
  still green, new `migration_0017_adds_size_and_trigger_columns` asserts
  both columns on a fresh migrate.

## 3. Test lines

Rust (`backup_tests.rs`, tempdir): per-trigger recording for all four
paths (manual/scheduled/cash_close/reset) with `size == fs::metadata`
equality; legacy NULL row → `None` + manual default (UI contract);
verify drift-correction (poison size → verify → repaired);
`backup_tests` create-assert strengthened to exact byte equality.
Fallout fixed in new code only: `fts_migration_tests` count 16→17
(pre-existing pin on the old max version), three `Option` asserts.
TS: `catalog-roundtrip` untouched; `formatBytes` unknown-pins updated
in `settings-module.test.ts` + `backup-modal.test.tsx` (old tests pinned
the silent "0 B").

Wire contract (no UI code changed): `fileSizeBytes: number | null`,
`null` renders "—". Formatter widening (`settings-helpers.ts`,
`backup-modal-store.ts`) is wire-enabling only — DSN-095 owns the final
o/Ko/Mo pass. Facade `BackupMetadata.fileSizeBytes` widened to
`number | null` (`ipc.ts`); `PreRestoreReport.fileSizeBytes` stays
`number` (verify stats a live file — always real).

## 4. Gates

| Gate | Result |
| ---- | ------ |
| G1 `cargo fmt --check` | CLEAN (exit 0) |
| G2 `cargo clippy --all-targets --all-features -- -D warnings` | 0 errors, GREEN |
| G3 `cargo test --workspace -j 2` | EXIT 0 — **70 suites, 541 passed, 0 failed, 7 ignored** |
| G4 `npm run lint` | exit 0, GREEN |
| G5 `npx tsc --noEmit` | clean, GREEN |
| G7 `npx vitest run` | **819 passed, 0 failed (81 files)**, GREEN |

## 5. Scratch + status

- Scratch: `brk094-g3.log` deleted, `ls scratch | grep brk094` empty
  (SCRATCH-CLEAN, NO_LEFTOVERS). No other ad-hoc scripts created.

Wire surface touched (`ipc.ts` type only) → G5 run, state reason. RESTORE
verify/precheck paths untouched in behavior (size already live-statted).

NEXT READY: YES (after tallies land).
