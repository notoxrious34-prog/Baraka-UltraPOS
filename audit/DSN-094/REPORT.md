# TASK-DSN-094 REPORT — Factory reset screen

Authority: BRK-091 `9495ef8` (`factory_reset_precheck` / `factory_reset_execute`,
Admin gate, token-before-write, verified-backup-enforced, `restart_required`).
Engine frozen — zero production-Rust changes. Owner taste: icon+text buttons,
full "Baraka UltraPOS" name, Arabic clarity. DO NOT COMMIT.

## 1. FAIL-before (base 9495ef8)

`git grep -l "factory_reset|FactoryReset|resetDanger" 9495ef8 -- src/` → empty.
No reset UI existed: no IPC facade, no danger-zone section, no `settings.reset*` keys.

## 2. Components (product code, DSN-094 worktree — frozen for C1)

- `src/lib/ipc.ts` +84: `ResetBackupRef`, `FactoryResetPrecheck`
  (`openSession`, `backupDirWritable`, `newestBackup`, `eligible`, `reasons`),
  `FactoryResetOutcome` (`resetBackupFileName`, `quarantinedTo`, `resetAt`,
  `restartRequired`); `getFactoryResetPrecheck(sessionToken)`,
  `executeFactoryReset(sessionToken, typedToken)` vs `factory_reset_precheck` /
  `factory_reset_execute` (`typed_token`, wire codes surface verbatim).
- `src/features/settings/factory-reset.ts` (37 lines, pure/testable):
  `expectedResetToken(storeName)` — trimmed store-name wire, blank →
  `FACTORY_RESET_BLANK_STORE_TOKEN = "DEP-01"` (documented mirror of the
  engine `expected_reset_token` fallback, engine re-validates); the precheck
  deliberately omits the token (BRK-091 §5).
  `canExecuteReset(eligible, typed, expected)` — eligible AND exact match,
  no trim (SafeDeleteModal `typedOk` convention).
- `src/features/settings/FactoryResetSection.tsx` (271 lines): red
  danger-zone section at the bottom of the backups tab (Admin-gated tab —
  convenience-only, engine enforces); precheck refresh; open-session block
  row; backup-dir writability row; newest-backup row; `reasons` rendered
  VERBATIM from the wire (mono codes, spec §1); "Créer une sauvegarde
  vérifiée" entry point reusing the `createBackup(sessionToken, "MANUAL")`
  flow + precheck refresh; typed-token input + execute button disabled unless
  the double guard holds (eligible precheck AND exact token — UI
  double-guard, engine triple-guards); success → amber restart notice
  (backup file + quarantine names) with full state lock (`locked` disables
  refresh / backup / token / execute); errors via `translateBackendError`.
- `src/lib/i18n.ts` +32: 16 `settings.reset*` FR + 16 AR (danger title/body
  with full product name, `Irréversible` / `لا رجعة فيه`, precheck rows,
  backup entry, token label `{token}`, execute/executing, restart title/body
  `{file}`/`{quarantine}`).
- `src/features/settings/BackupSettingsTab.tsx` +1: `<FactoryResetSection
  locale sessionToken>` at tab bottom. Other settings sections untouched.

## 3. Pins — 15/15 green

`src/features/settings/factory-reset.test.ts`: `npx vitest run
src/features/settings/factory-reset.test.ts` → 1 file, 15 tests passed.
Token derivation (trimmed/blank→DEP-01); guard (mismatch/empty/ineligible
blocked pre-IPC); 16-key FR/AR symmetry + product-name/Arabic-clarity copy;
wire bridge shape (snake_case fields, `typed_token`, `restart_required`);
UI shape (mounted in backups tab with sessionToken, restart lock,
verbatim reasons, backup entry point, store-identity-derived token).

## 4. Captures — 7/7 GREEN, 0 console

`audit/DSN-094/capture.mjs` (headless CDP vs vite :5173, TAURI_INTERNALS
stubbed; stub permission list widened with `CREATE_SALE` + view grants after
probe proved the default POS view gated the `<header>` wait).
C1 selector fixes (capture-script-only): refresh scoped to the danger-zone
`section[aria-label^="Zone"|"منطقة"]` (page renders several "Actualiser");
locale toggle matched by containment (Topbar icon+label) on the POS view
(Topbar absent from settings views — switch on "Caisse", back via
"الإعدادات" → "النسخ الاحتياطي والأمان"), RTL asserted on the `[dir=rtl]`
shell container (`App.tsx:234`, not `documentElement`).
`capture-results.json`: 7/7 ok, consoleErrors 0.

- `reset-danger-fr-light.png` — eligible precheck (2 green rows + newest
  backup file row), token empty → execute disabled, backup entry present,
  no h-scroll. Clean, no clipping/overlap.
- `reset-blocked-fr-light.png` — blocked rows (open session + unwritable
  dir + no backup) + `RESET_OPEN_SESSION` / `RESET_BACKUP_UNWRITABLE`
  verbatim codes + guidance, execute disabled. Codes shown verbatim is
  SPEC (SCOPE-IN §1 / acceptance) — not a leak.
- `reset-confirm-ar-light.png` — full Arabic RTL (sidebar right, icon at
  RTL-start), token typed, execute solid-red enabled, `[dir=rtl]` shell,
  no h-scroll. (The "EL" glyph reading in QA is a resolution artifact —
  the exact-match `exec-enabled` assert passed, proving byte equality.)

## 5. Gates

| Gate | Result |
| ---- | ------ |
| G4 `npm run lint` | exit 0 |
| G5 `npx tsc --noEmit` | exit 0 |
| G6 `npm run build` | exit 0, `✓ built in 6.35s` |
| G7 `npx vitest run` | exit 0, **80 files / 812 tests passed** (797 + 15 reset pins) |
| G1–G3 | N/A — zero Rust touched (state reason) |

## 6. NO-UI note / handoff

Wire only + screen done; engine enforces everything (session, permission,
token, preconditions, verified backup, restart contract). Post-success the
operator must quit and relaunch before any operation (state lock + notice).
Unblocks the owner empty-program test.

NEXT READY: YES.
