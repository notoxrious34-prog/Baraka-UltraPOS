# TASK-DSN-081 REPORT — Residual write-button gates (closes Batch 2c)

## 1. Status & diff (no commit performed)

- `git status --short`: 5 modified (StoreIdentityTab, PrinterSettingsTab,
  CategoryManageModal, settings-module.test.ts, catalog-hub.test.tsx) +
  untracked `audit/AUD-035/` (prior task) + `audit/DSN-081/`.
- `git diff --stat`: 5 files, 75 insertions / 13 deletions, all within
  SCOPE-IN. i18n untouched (0 lines — all guidance keys already existed
  FR+AR, as AUD-035 predicted). `position: fixed` in src: zero.
  Vite :5173 stopped (port DOWN). No leftover scripts.

## 2. Per-control gates + engine citations (all re-verified live)

| Control | UI gate (reactive `useSessionStore` selector, Trap 16) | Engine |
|---|---|---|
| Identity save (`StoreIdentityTab.tsx:212`) | `canWrite=hasPermission("SYSTEM_SETTINGS_MANAGE")` folded into validation-only `canSave` + `title=settings.identityWriteReservedGuidance`; `handleSave` guards `!canSave \|\| !canWrite` (no IPC when unauthorized) | `SaveStoreIdentityUseCase::execute` `printing.rs:179`, guard `printing.rs:187`; IPC `store_identity_save` `ipc.rs:2915`, require `ipc.rs:2922` |
| Printer save (`PrinterSettingsTab.tsx:155`) | `canSaveSettings=hasPermission("MANAGE_PRINTER_SETTINGS")`; `disabled={saving \|\| !canSaveSettings}` + `title=settings.printerManageReservedGuidance`; `handleSave` guards | `UpdatePrinterSettingsUseCase::execute` `printing.rs:~433`, guard `printing.rs:442`; IPC `printer_update_settings` `ipc.rs:2870` |
| Test-print FR/AR (`:168,178`) | `canTestPrint=hasPermission("SYSTEM_SETTINGS_MANAGE")`; `disabled={testing!==null \|\| !canTestPrint}` + same printer title; `handleTestPrint` guards | `TestPrintUseCase::execute` `printing.rs:~326`, guard `printing.rs:337`; IPC `printer_test_print` `ipc.rs:2976` |
| Category create/update/toggle/seed (`CategoryManageModal.tsx`: submit/toggleActive/seedStarter + 4 buttons incl. edit affordance) | `canManage=hasPermission("STOCK_ADJUSTMENT") \|\| hasPermission("SYSTEM_SETTINGS_MANAGE")`; every write button `disabled={… \|\| !canManage}` + `title=settings.catalogManageReservedGuidance`; all three write paths early-return | `MANAGE_PERMISSIONS=[StockAdjustment,SystemSettingsManage]` `catalog.rs:18-21`; `CreateCategoryUseCase` guard `catalog.rs:568`, `UpdateCategoryUseCase` guard `catalog.rs:620` |

## 3. Gates

- G1 fmt: exit 0. G2 clippy `-D warnings`: clean. G3: 469/0/0.
- G4 lint: exit 0. G5 typecheck: exit 0.
- G6 build: `✓ built in 13.80s`, JS 741.55 kB / gzip 190.71 kB.
- G7 full vitest: **72 files / 713 passed** (710 + 3 net new pins).
- Regression note: no stale busy/validation-only pins existed for these
  three controls (gates are new), so no failing-then-fixed demo was
  applicable; 3 new gate pins added (identity+printer in
  settings-module.test.ts, category panel in catalog-hub.test.tsx),
  all green first run alongside preserved IPC-shape pins.

## 4. Visual (audit/DSN-081/, 5/5 CDP steps, 0 console errors)

- `identity-1280x800-fr-light.png` (LIMITED): save `disabled=true`
  `title="Réservé — Gestion système."`, `1280x800 dpr=1 noPageScroll`.
  Vision: 4 sidebar lock badges consistent, no overlap/scroll.
- `printer-1280x800-ar-dark.png` (LIMITED): save `disabled=true`
  `title="مقيّد — إدارة النظام."`, `1280x800 noPageScroll`. Vision:
  full RTL, COM1/9600/80mm loaded, no error banner, no overlap.
- `category-1280x800-fr-light.png` (ADMIN): submit enabled,
  `title=null`, `1280x800 noPageScroll` (authorized unchanged).
- `settings-nav-1024x600-ar-dark.png` (LIMITED): `tabs=8 badges=4`,
  `1024x600 noPageScroll`.
- Drill fixes: `printer_get_settings` (not `printer_settings_get`) +
  snake_case wire (`serial_port/serial_baud/paper_width_dots/mm`);
  category disabled interior is settings-route-unreachable by design
  (route SSM‖SA ≡ gate SA‖SSM) — test-pinned, enabled state captured.
