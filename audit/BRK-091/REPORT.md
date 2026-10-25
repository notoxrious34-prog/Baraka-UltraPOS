# TASK-BRK-091 REPORT — Factory reset engine (use case + IPC + tests)

- Authority: AUD-043 §3 (KEEP/WIPE table + safety spec). Design decision: **fresh-file quarantine-swap, no wipe-by-DELETE** — `BEFORE DELETE` abort-triggers guard every ledger/log table (migrations 0001/0004/0008/0009: customer_ledger, supplier_ledger, stock_movements, cash_movements, payments, audit_log, command_log, document_items, documents, cash_sessions), so DELETE-based wipe is impossible without a schema change. No schema change was made: the fresh DB is produced by the embedded migration runner.
- Base `919e45c`. No commit. UI explicitly out (DSN-094 follows — wire only). Live DB never touched (tempdir file-DBs only).

## 1. Preconditions + lines (codes only, no new prose)

- `crates/application/src/use_cases/factory_reset.rs` [CREATED]: `PrecheckFactoryResetUseCase` (read-only: `INVALID_TERMINAL_ID`, open-session read via `CashSessionRepository::get_active_session`, dir write-probe → `RESET_BACKUP_UNWRITABLE`, `newest_backup`, `eligible` + `reasons`) + `ExecuteFactoryResetUseCase` with `ResetExecution` params bundle (clippy 7-arg cap): G1 open-session read → `RESET_OPEN_SESSION`; G2 typed-token constant-shape compare (`constant_time_eq`) vs `FactoryResetRepository::expected_reset_token` → `RESET_TOKEN_MISMATCH`; G3 FS probe → `RESET_BACKUP_UNWRITABLE`; Step 1 mandatory backup via existing `CreateBackupUseCase` (Manual trigger — no backup-protocol change) + `VerifyBackupIntegrityUseCase` re-verifying the created file → `RESET_BACKUP_FAILED`; Step 2 `execute_reset`. Token validated BEFORE any write including the backup.
- `crates/application/src/ports.rs` [MODIFIED]: `FactoryResetRepository` trait (`has_open_cash_session`, `expected_reset_token` = `store.name` or `DEP-01` fallback per SafeDeleteModal entity-code-or-name convention, `execute_reset`); zero backup logic in port.
- `crates/domain/src/backup.rs` [MODIFIED]: `FactoryResetOutcome` (`reset_backup_file_name`, `quarantined_db_name`, `reset_at`, `restart_required: true`) + impossibility doc comment.

## 2. Wipe/reseed lines

- `crates/infrastructure/src/adapters/sqlite_backup_manager.rs` [MODIFIED]: `impl FactoryResetRepository for SqliteBackupManager` — F0 process-wide guard shared with restore (`RESET_IN_PROGRESS`); F1 quiesce (checkpoint + park connection); F2 BUILD fresh migrated DB via embedded `run_migrations` (first-run state incl. DEP-01 by construction) + `ATTACH … INSERT INTO main.backups_log SELECT` (history carried); F3 QUARANTINE previous live DB (never delete, `unique_quarantine_base` reuse) + atomic same-dir rename; F4 chain-linked `RESET_PERFORMED` as FIRST audit row (empty table → `GENESIS` via `insert_cash_audit_row`) + checkpoint/close/reopen shared conn. Rollback: pre-swap failure reopens untouched original; post-swap activation failure moves quarantined original back. Helpers: `coded_reset`, `remove_reset_staging`, `move_reset_file`.
- KEEP/WIPE verdict (proven by tests §4): KEEP `schema_migrations` (full chain), `backups_log` + files (safety net); WIPE+reseed users→empty (first-run bootstrap-ready), warehouses→DEP-01 seed (`01950000-0000-7000-8000-000000000001`, `Dépôt Principal`, active), sequences/tax/seeds→runner state, settings→exactly the 4 migration-0006 rows (transport back to `SERIAL`), store identity blank (token falls back to `DEP-01`); WIPE catalog/stock/documents+items+payments/customers+ledgers/suppliers+ledgers/cash/print_jobs/document_sequences/command_log/settings-merchant/audit-logs + first row `RESET_PERFORMED` (actor, GENESIS-chained).
- Crash-ordering guarantee: backup completes + verifies BEFORE staging begins; crash before the swap = old DB intact + fresh backup present (retryable). Nothing is ever wiped before the backup verifies.

## 3. IPC lines (wire only)

- `crates/commands/src/dto.rs` [MODIFIED, +55]: `FactoryResetPrecheckRequestDto`, `ResetBackupRefDto`, `ResetPrecheckDto`, `FactoryResetExecuteRequestDto`, `FactoryResetOutcomeDto`.
- `crates/commands/src/ipc.rs` [MODIFIED]: `factory_reset_precheck` + `factory_reset_execute` (both `require_backup_admin` = `SystemSettingsManage`, same gate as restore, stated); execute wires `ExecuteFactoryResetUseCase::new(backups, cash, factory_reset)` with retention 30 + `CARGO_PKG_VERSION`; validation→backup→swap ordering; `error_code` mapping, no new prose. `AppState::factory_reset: Arc<dyn FactoryResetRepository>` added (second handle to the same SQLite manager, mirroring the `store_identity: printer_settings.clone()` precedent).
- `src-tauri/src/lib.rs` [MODIFIED]: shared `backups` manager construction + both commands registered in `invoke_handler`.

## 4. Test lines + gate table

- `crates/infrastructure/tests/factory_reset_tests.rs` [CREATED, 5 tests, tempdir-only]: `precheck_reports_writable_state_and_writes_nothing` (unwritable-probe reason, zero writes); `execute_rejects_open_session_with_zero_writes` (`RESET_OPEN_SESSION`, zero log rows/files, data intact); `execute_rejects_unwritable_backup_dir_with_zero_writes` (file-as-dir blocker); `execute_rejects_wrong_token_with_zero_writes` (validation before backup: zero rows/files); `execute_full_reset_returns_first_run_state` (restart_required, backup file exists + carried in log, 18-table WIPE-zero asserts, settings seed-state asserts, DEP-01 reseed + `schema_migrations`>10, single `RESET_PERFORMED`/`factory_reset`/GENESIS audit row, `quick_check=ok`, quarantine holds old data, token→`DEP-01`). Plus 1 unit test (`constant_time_eq`).
- Gate fallout fixed in NEW code only (C1): verify-use-case 3rd arg, `ResetExecution` bundle (clippy 8/7), `AppState::factory_reset` second handle, test seed-schema drift (suppliers/supplier_ledger/cash_sessions/cash_movements 0008/0009 shapes, settings upsert, DEP-01 UUID `0195…`), wrong-UUID const typo. No pre-existing suite touched.
- Gates: G1 `cargo fmt --check` CLEAN; G2 `cargo clippy --all-targets --all-features -- -D warnings` GREEN zero warnings; **G3 `cargo test --workspace -j 2` GREEN — 521 passed, 0 failed, 7 ignored** (515 baseline + 5 reset integration + 1 reset unit). G4-G7 N/A backend-only (no TS touched — state reason).

## 5. NO-UI note

No screens, no settings tab, no modal: the two commands are registered and Admin-gated but unwired in the frontend. DSN-094 builds the reset screen on `factory_reset_precheck`/`factory_reset_execute` (typed-token echo = store name shown by precheck? — precheck does not return the token by design; UI reads store identity via existing settings wire).

## 6. UNKNOWNs

- None blocking. Owner empty-program test (DSN-094 screen + hardware trial) is downstream.

NEXT READY: YES
