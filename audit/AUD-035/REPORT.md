# TASK-AUD-035 REPORT — DSN-079 residual write buttons + suppliers/reports/purchases legacy sweep

## 1. Residual write controls (all three mapped, engine-cited)

| # | Control | file:line / current behavior | Engine Permission + Rust file:line | Recommended `hasPermission` expr + UX |
|---|---------|------------------------------|-------------------------------------|----------------------------------------|
| R1 | Identity save | `StoreIdentityTab.tsx:212` `disabled={!canSave}` where `canSave` (:72) is validation-only (`!loading && !saving && nameOk && phoneOk && fiscalOk`); `handleSave` (:75) guards validation only, then calls `saveStoreIdentity` — enabled for any settings visitor with valid input | `SYSTEM_SETTINGS_MANAGE` — `SaveStoreIdentityUseCase::execute` `printing.rs:179`, guard `printing.rs:187`; IPC `store_identity_save` `ipc.rs:2915`, require `ipc.rs:2922` | `hasPermission("SYSTEM_SETTINGS_MANAGE")`; Disabled-with-Guidance: `disabled={saving \|\| !canWrite}` + `title=settings.identityWriteReservedGuidance` (FR+AR exist, i18n.ts:174/:1235); guard `if (!canWrite) return` in `handleSave`; null session already covered by SettingsModule UNAUTHORIZED card (DSN-079) |
| R2a | Printer settings save | `PrinterSettingsTab.tsx:155` `disabled={saving}` → `handleSave` — no permission check | `MANAGE_PRINTER_SETTINGS` — `UpdatePrinterSettingsUseCase::execute` `printing.rs:~433`, guard `printing.rs:442`; IPC `printer_update_settings` `ipc.rs:2870` | `hasPermission("MANAGE_PRINTER_SETTINGS")`; Disabled-with-Guidance + `title=settings.printerManageReservedGuidance` (exists FR+AR, i18n.ts:176/:1237); guard `handleSave` |
| R2b | Printer test-print FR/AR | `PrinterSettingsTab.tsx:168,178` `disabled={testing !== null}` → `handleTestPrint` — no permission check | `SYSTEM_SETTINGS_MANAGE` — `TestPrintUseCase::execute` `printing.rs:~326`, guard `printing.rs:337`; IPC `printer_test_print` `ipc.rs:2976` (doc: SystemSettingsManage) | `hasPermission("SYSTEM_SETTINGS_MANAGE")`; Disabled-with-Guidance + same printer key (wording "Gestion système" covers both); guard `handleTestPrint` |
| R3 | Category create/update (+ seed) | `CategorySettingsTab.tsx` delegates to `CategoryManagePanel` (`catalog/CategoryManageModal.tsx`); write buttons gated only on `busy/seeding` (:274,:340,:349,:392,:409) — no permission check | `StockAdjustment \|\| SystemSettingsManage` (`MANAGE_PERMISSIONS`, `catalog.rs:18-21`) — `CreateCategoryUseCase::execute` guard `catalog.rs:568`, `UpdateCategoryUseCase::execute` guard `catalog.rs:620` (no DeleteCategory UC — removal via update) | `hasPermission("STOCK_ADJUSTMENT") \|\| hasPermission("SYSTEM_SETTINGS_MANAGE")`; Disabled-with-Guidance + `title=settings.catalogManageReservedGuidance` ("Réservé — Gérants et administrateurs.", FR+AR i18n.ts:175/:1236, matches the any-of grant exactly); guard submit paths |

Security note (all): engine authoritative per citations above; UI convenience
only. Missing-keys list: NONE — all guidance keys already exist FR+AR.
Edge: null session → SettingsModule UNAUTHORIZED card (no fetch attempted);
Vendeur direct action → engine `Unauthorized` → `translateBackendError`;
RTL tooltips mirror automatically (`title` attr), numbers LTR-isolated.

## 2. Suppliers/reports/purchases sweep (raw output: GREP.txt, 14 lines)

- Production code (`*.tsx/*.ts` excl tests): **CLEAN** — zero
  `isAdmin|isManager|role ===|session.role` and zero
  `ADMIN|GERANT|CAISSIER|VENDEUR` literals in all four areas.
- Test files: only `not.toContain` pins (settings-module,
  fiscal-settings-tab) — convention, no stale role pins.
- Files swept: suppliers (4: SupplierListScreen, SupplierFormModal,
  SupplierStatementView + tests), reports (11 screens + components),
  purchases (4 screens, mounted inside SuppliersModule —
  `SupplierListScreen.tsx`, no separate route), settings (12 files).
- Route gates (App.tsx `canAccessView`): suppliers `SUPPLIER_MANAGE`
  (:60); reports `REPORTS_VIEW‖CASH_REPORT_VIEW` (:63, engine guards
  each report); settings `SYSTEM_SETTINGS_MANAGE‖STOCK_ADJUSTMENT`.
  Purchases inherits the suppliers route. No per-action legacy gates
  found; engine remains authoritative.
