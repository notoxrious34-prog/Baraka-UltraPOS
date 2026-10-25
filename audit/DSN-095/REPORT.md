# TASK-DSN-095 REPORT — Backup/restore/reset screen redesign

STATUS: **BLOCKED** (engine-side 0 B gap — no UI fakery, zero production
changes made). DO NOT COMMIT.

## 1. FAIL-before (current chaos refs)

- Raw UTC timestamps: `src/features/settings/BackupSettingsTab.tsx:171`
  (`<bdi dir="ltr">{row.createdAt}</bdi>` — RFC3339 string, no locale
  formatting).
- "0 B" sizes: same file `:172` (`formatBytes(row.fileSizeBytes)` — the
  formatter is correct; the wire feeds it `0`, see §2).
- Truncated raw checksums as a status column: `:176`
  (`checksumPrefix(row.sha256Checksum)` — 12 hex chars + ellipsis where a
  VERIFIED badge belongs).
- Duplicated backup actions: tab-level create (`:96-104`) + refresh
  (`:106-114`) AND a second verified-backup entry inside the danger zone
  (`src/features/settings/FactoryResetSection.tsx:188`,
  `handleCreateBackup` `:65-72` reusing `createBackup`).
- Type pill fed by a hardcoded wire value (§2) via
  `triggerBadgeClass(row.triggerType)` (`settings-helpers.ts:57-62`).
- Restore: wire exists (`backups_restore_precheck` / `backups_restore`)
  but has **zero UI callers** (no hits in `src/features/settings`,
  `src/components`) — polishable only after unblock.

## 2. 0 B verdict + lines — ENGINE-SIDE (blocked fork taken)

`backups_list` (`crates/commands/src/ipc.rs:2218-2226`) maps
`ListBackupsUseCase`, which hardcodes the size:

- `crates/application/src/use_cases/backup.rs:175` —
  `file_size_bytes: 0` (literal zero, with the comment "Load rows and
  rejoin each file name with `dest_dir`").
- Root cause: `BackupLogEntry` (`crates/application/src/ports.rs:2225-2246`)
  carries **no size field**, and `backups_log`
  (`0001_baseline_schema.sql:314-324`) has **no size/trigger columns** —
  `record_backup` (`sqlite_backup_manager.rs:154-176`) and `list_backups`
  (`:209-236`) / `get_backup` select only the 9 sizeless columns.
- Same gap forces `trigger_type: BackupTriggerType::Manual`
  (`backup.rs:177`) — every row's pill reads MANUAL regardless of the
  SHIFT_CLOSE/SCHEDULED capture that wrote it.

No UI formatter can honestly fix this: any displayed size would be
fabricated. Per the task fork: STATE BLOCKED, no UI fakery.

**QUESTION FOR ARCHITECT**: authorize a BRK follow-up persisting
`file_size_bytes` (+ `trigger`) per backup row — (a) append-only
migration `00XX` adding the columns, populated in `record_backup` from
the already-known snapshot size, selected in `list_backups`/`get_backup`
and threaded through `BackupLogEntry` → `BackupMetadata` → DTO?

**PROPOSED SOLUTION** (not implemented): option (a) above; list-time
`fs::metadata` stat rejected (TOCTOU + I/O per row, PRUNED rows have no
file). UI redesign resumes on the follow-up's green.

## 3. Component lines — NONE (frozen by block)

Zero files changed. On unblock the redesign touches only:
`BackupSettingsTab.tsx`, `FactoryResetSection.tsx` (elegance pass),
`settings-helpers.ts` (locale date + o/Ko/Mo formatters), `lib/ipc.ts`
facade (unchanged wire), `lib/i18n.ts` (FR/AR keys), pins + captures.

## 4. Pin lines — NONE (blocked before pins)

## 5. PNG names + asserts — NONE (no redesigned states to capture)

## 6. G4-G7 exits + counts (untouched tree, baseline proof)

- G4 `npm run lint`: exit 0, GREEN.
- G5 `npx tsc --noEmit`: clean, GREEN.
- G7 `npx vitest run`: **819 passed, 0 failed**.
- G1-G3: N/A — zero Rust changes (state reason); engine untouched.

## 7. git status --short

Clean (this task changed nothing — verify at seal time).

## 8. STATE / DONE / REMAINING

- STATE: **BLOCKED** (engine-side size/trigger gap, DTO lines quoted §2).
- DONE: full trace (wire → field), FAIL-before refs, baseline G4/G5/G7 green.
- REMAINING: entire redesign (items 2-7) pending the BRK size/trigger
  follow-up; restore UI has no callers yet (wire ready).

NEXT READY: NO.
