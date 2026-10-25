# TASK-BRK-092 REPORT — Backup hardening engine

Authority: AUD-043 gaps G-BK1..5 (`919e45c`). Manual backup + restore +
retention-30 frozen (behavior unchanged). No schema change (`backup.*`
keys in the existing `settings` table). No new dependencies
(USB SHA-256 re-check reuses the audited `persistence::sha256::Sha256`
core). Live DB never touched (tempdir file-DBs only). DO NOT COMMIT.

## 1. Hook points (wiring lines)

- Scheduler trigger — no durable background tick exists in the app
  (Tauri `setup` in `src-tauri/src/lib.rs:87` is one-shot; the print worker
  is the only loop and is job-scoped). Choice STATED: **on-launch
  due-check** (`src-tauri/src/lib.rs` setup calls
  `ScheduledBackupUseCase::run_system`, silent when not due, never a
  startup failure) **+ on-action** admin-gated `backup_schedule_run`
  command for the operator to trigger the due-check by hand.
- Cash-close hook — `cash_confirm_closing` (`crates/commands/src/ipc.rs`):
  `ConfirmClosingUseCase` commits first, THEN
  `CashCloseBackupUseCase::run_after_close` fires `SHIFT_CLOSE`.
- Read path — `backup_hardening_status` returns stored schedule + USB
  destination + durable last-status + due-check report (Admin, read-only).
- Configure path — `backup_hardening_configure` persists cadence/time/USB
  (Admin; `INVALID_BACKUP_CADENCE` / `INVALID_BACKUP_TIME` pre-write).
- IPC registration: 3 commands in `src-tauri` invoke_handler after
  `backups_restore`; `AppState.backup_hardening` second handle on a new
  `SqliteBackupHardening` sharing the live connection.

## 2. Rules (window / ordering / failure)

- Window: daily = local calendar day at stored `HH:MM`; weekly = ISO week
  (Monday) at stored time. Idempotency: skip when a SUCCESS backup already
  landed inside the current window (`now >= window_start &&
  last_success < window_start`) → `BACKUP_NOT_DUE`. Due-check is read-only
  (zero writes — proven by test).
- Ordering: close hook runs AFTER the close transaction commits
  (CLOSING → CLOSED sealed first). Reason: a stranded till is worse than a
  missed automatic backup. Failure rule CLOSE_WINS_WARN: a failed
  post-close backup surfaces as a non-blocking `backup_warning` code on
  the close response; the committed close is never rolled back, and no
  SUCCESS row is logged for the failed attempt.
- USB: copy (never move) → size check → SHA-256 re-check; mismatch removes
  the copy (`USB_COPY_SIZE_MISMATCH` / `USB_COPY_HASH_MISMATCH`); the local
  snapshot is never touched. Mirror failure is non-fatal to the backup
  (`usb: None` in the scheduled outcome, local `SUCCESS` stands).
- Codes only, no new prose: `BACKUP_NOT_DUE`, `INVALID_BACKUP_CADENCE`,
  `INVALID_BACKUP_TIME`, `USB_COPY_INVALID_SOURCE`,
  `USB_DESTINATION_UNWRITABLE`, `USB_COPY_FAILED`, `USB_COPY_SIZE_MISMATCH`,
  `USB_COPY_HASH_MISMATCH` (all pass `error_code` → stable wire codes).

## 3. Tests (tempdir file-DBs, `backup_hardening_tests.rs`)

10 tests, all green: disabled-default + never-due; unknown cadence →
disabled; daily round-trip + due → run (Scheduled) → skip + `BACKUP_NOT_DUE`
+ exactly one `.db` (WAL sidecars ride along); weekly due → skip;
due-check zero-writes (`backups_log` count unchanged); close → SHIFT_CLOSE
SUCCESS created after `closed_at`; close-failure injection (dest is a file)
→ error, session still CLOSED, no SUCCESS row; USB copy verified +
byte-identical + local kept; USB failures → `USB_COPY_FAILED`; last-status
reads newest row (empty → blank status).

## 4. Gates

| Gate | Result |
| ---- | ------ |
| G1 `cargo fmt --check` | CLEAN |
| G2 `cargo clippy --all-targets --all-features -- -D warnings` | 0 errors, GREEN |
| G3 `cargo test --workspace -j 2` | GREEN, exit 0 — **531 passed, 0 failed, 7 ignored** (70 suites; 521 baseline + 10 new) |
| G4–G7 | N/A — backend-only, no shared TS surface touched |

Dead-code verdict: `SHIFT_CLOSE` now called (close hook
`cash_confirm_closing` → `CashCloseBackupUseCase`); `SCHEDULED` now called
(`ScheduledBackupUseCase::run` via `backup_schedule_run` + setup
`run_system`). GSprite `MANUAL` unchanged. Quote:
`crates/commands/src/ipc.rs:2184-2185` (parsing kept),
`BackupTriggerType::ShiftClose` (close hook),
`BackupTriggerType::Scheduled` (scheduler run).

## 5. Handoff

Engine sealed; UI follow-up reads `backup_hardening_status`,
writes `backup_hardening_configure`, fires `backup_schedule_run`,
and displays `backup_warning` from `cash_confirm_closing`.
Blocks backup-settings UI + owner trust test.

NEXT READY: YES (after G3 counts land).
