# TASK-DSN-079 REPORT — Settings admin gates → granular hasPermission

## 1. Status & scope

- `git status --short`: 7 modified (`SettingsModule.tsx`,
  `FiscalSettingsTab.tsx`, `BackupSettingsTab.tsx`,
  `settings-module.test.ts`, `fiscal-settings-tab.test.ts`,
  `settings-command-center.test.ts`, `src/lib/i18n.ts`) + untracked
  `audit/DSN-079/`. No commit ordered.
- `git diff --stat`: 7 files, 136 insertions / 40 deletions; i18n +10
  lines (≤12 allowed). `position: fixed` in src: zero. Vite :5173 stopped
  (port DOWN). No `session?.role` / `session.role` / `isAdmin` residue in
  `src/features/settings/` outside tests.

## 2. Per-tab permission + engine citation

| Tab | Write gate (UI) | Engine citation |
|---|---|---|
| identity | `SYSTEM_SETTINGS_MANAGE` (lock badge; save control itself out of scope) | save: ipc.rs:2922 (`require SystemSettingsManage`); get: no guard (any authenticated) |
| categories | `STOCK_ADJUSTMENT \|\| SYSTEM_SETTINGS_MANAGE` (badge) | MANAGE_PERMISSIONS, catalog.rs:19-21, enforced :568/:612 |
| pack (starter seed) | same as categories (badge) | MANAGE_PERMISSIONS, ≈catalog.rs:707 |
| exchange (import) | same as categories (badge) | MANAGE_PERMISSIONS, ≈catalog.rs:918 |
| printer | `MANAGE_PRINTER_SETTINGS` (badge; transport controls out of scope) | update: printing.rs:442; reads: printing.rs:187/337 (`SystemSettingsManage`) |
| fiscal | `SYSTEM_SETTINGS_MANAGE` (switch disabled + `settings.fiscalWriteReservedGuidance` FR/AR title; `handleToggle` guards) | write: sales.rs:1395 |
| backups | `SYSTEM_SETTINGS_MANAGE` (note kept + disabled create + `settings.backupCreateReservedGuidance`; `refresh`/`handleCreate` guard, no doomed IPC) | create+list: ipc.rs:2128-2136 (`require_backup_admin`, applied :2173/:2205) |
| about | none (never locked) | — |

- 5 new guidance keys FR+AR (i18n.ts +10): identity/catalog/printer/fiscal/
  backup `*ReservedGuidance`; terse commercial FR ("Réservé — Gestion
  système."; catalog variant "Réservé — Gérants et administrateurs."),
  MSA AR ("مقيّد — إدارة النظام."; catalog "مقيّد — للمديرين والمسؤولين فقط.").
- Out-of-scope residual (documented, engine still denies): identity save
  button, printer transport/test-print, category/pack/exchange write
  controls have no UI gate yet — follow-up task; lock badges already warn.

## 3. Gates

- G1 fmt: exit 0. G2 clippy `-D warnings`: clean. G3 cargo: 469/0/0.
- G4 lint: clean. G5 typecheck: clean.
- G6 build: `✓ built in 10.93s`, JS 740.60 kB / gzip 190.48 kB.
- G7 vitest full: **72 files / 708 tests passed** (704 + 4 new).
- Regressions fixed per Law of Regression (stale role-string pins,
  demonstrated failing then fixed): `fiscal-settings-tab.test.ts`
  (`disabled={saving || !isAdmin}` → canWrite+guidance pin) and
  `settings-command-center.test.ts` (`lockedForNonAdmin` → managePermissions
  pin). Both files were outside SCOPE-IN's test list; edits are confined to
  stale-pin replacement, no behavior change.

## 4. Visual (audit/DSN-079/, Gerant-like session: STOCK_ADJUSTMENT only)

- `settings-fiscal-1280x800-fr-light.png`: switch `disabled=true`,
  `title="Réservé — Gestion système."`, no page scroll. AR-dark frame
  vision-verified: full RTL, note + disabled gold create button, 4 lock
  badges (identity/printer/fiscal/backups), no h-scroll.
- `settings-backups-1280x800-ar-dark.png`: create `disabled=true`,
  `title="مقيّد — إدارة النظام."`, no page scroll.
- `settings-nav-1024x600-fr-light.png` / `settings-nav-1024x600-ar-dark.png`:
  `tabs=8 badges=4`, no page scroll.
- 5/5 CDP steps green, 0 console errors. (One drill-harness note:
  `body.innerText` Arabic polling proved flaky in headless; replaced with
  deterministic disabled-button probing; end-state identical and
  screenshot-verified.)
