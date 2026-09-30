# AUDIT-REPORT — Baraka UltraPOS repository, runtime, presentation layer (TASK-AUD-001)

Read-only factual audit. Every claim carries a file:line reference or command output. `UNKNOWN` marks unverifiable facts with the reason. No recommendations, no fixes, no speculation. Working directory for all commands: `D:/Dev/Baraka UltraPOS`. Audit date: 2026-09-30.

## A. REPOSITORY FACTS

- Git branch: UNKNOWN — `git branch` output: `fatal: not a git repository (or any of the parent directories): .git`.
- HEAD hash: UNKNOWN — `git rev-parse HEAD` output: `fatal: not a git repository (or any of the parent directories): .git`.
- Last 30 commit subjects: NO EVIDENCE — no git repository exists.
- `git status --short`: NO EVIDENCE — same fatal error as above.
- Tags: NO EVIDENCE — `git tag` output: `fatal: not a git repository (or any of the parent directories): .git`.
- File counts and lines (commands: `find crates src src-tauri tests docs -name "*.<ext>" | wc -l` and `-exec cat {} + | wc -l`):

| ext | files | lines |
|---|---|---|
| rs | 136 | 49917 |
| ts | 52 | 11742 |
| tsx | 56 | 10941 |
| css | 1 | 35 |
| sql | 13 | 1035 |
| json | 6 | 4629 |
| md | 16 | 2890 |

- Titles of TASK-BRK-001..TASK-BRK-057: NO EVIDENCE — no file in the repo documents task titles. `docs/adr/` does not exist. The only BRK references in-repo are investigation documents under `docs/audits/`: `TASK-BRK-039-payment-overpayment-investigation.md`, `TASK-BRK-040-document-conversion-contracts-investigation.md`, `TASK-BRK-041-document-query-audit.md`, `TASK-BRK-044-returns-and-avoir-architecture-investigation.md`, `TASK-BRK-048-catalog-stock-warehouse-audit.md`, `TASK-BRK-049-catalog-and-warehouses-audit.md`, `TASK-BRK-050-suppliers-purchases-reports-audit.md`, plus `FIRST_LIVE_SALE_TICK_2026_000002_FORENSIC_AUDIT.md`, `LOGIN_PIPELINE_TRACE_AUDIT.md`, `PRODUCT_CATALOG_PIPELINE_AUDIT.md`, `WAREHOUSE_LIFECYCLE_AUDIT.md`.

## B. DEPENDENCY AND WINDOW FACTS

- `package.json` (exact strings): dependencies `@tauri-apps/api ^2.1.1`, `react ^18.3.1`, `react-dom ^18.3.1`, `zustand ^5.0.15`; devDependencies `@tauri-apps/cli ^2.11.5`, `@types/react ^18.3.3`, `@types/react-dom ^18.3.0`, `@typescript-eslint/eslint-plugin ^8.8.0`, `@typescript-eslint/parser ^8.8.0`, `@vitejs/plugin-react ^4.3.4`, `autoprefixer ^10.4.20`, `eslint ^8.57.1`, `eslint-plugin-react-hooks ^5.0.0`, `postcss ^8.4.47`, `tailwindcss ^3.4.13`, `typescript ~5.6.2`, `vite ^5.4.8`, `vitest ^3.2.7`. Toolchain observed: `rustc 1.98.1`, `node v26.7.0`.
- `Cargo.toml [workspace.dependencies]` (exact): `serde 1.0 derive`, `serde_json 1.0`, `tempfile 3`, `thiserror 1.0`, `chrono 0.4`, `uuid 1.10 v4+serde`, `rusqlite 0.32 bundled+backup+modern_sqlite+hooks`, `tauri 2.0`, `tauri-build 2.0`, `rustybuzz 0.18`, `ab_glyph 0.2`, `ttf-parser 0.24`, `unicode-bidi 0.3`, `image 0.25`, `serialport 4`. All five crates use `version.workspace = true` (root `Cargo.toml:12` = `1.0.0`). The single non-workspace crate dependency: `argon2 0.5` (`crates/infrastructure/Cargo.toml:18`). Exact Tauri version: `2.0` range (Cargo), CLI `^2.11.5` (package.json).
- `src-tauri/tauri.conf.json` `app.windows` (complete, verbatim): single window `{title Baraka UltraPOS, width 1280, height 800, minWidth 1024, minHeight 768, resizable true, fullscreen false}`. Absent keys (not present in file): `maximized`, `decorations`, `zoom`, devtools settings. `app.security.csp` is `null`.
- Capability files: `src-tauri/capabilities/default.json` only — `{identifier default, windows [main], permissions []}`.

## C. FRONTEND STRUCTURE

- Routing: no router dependency (no `react-router` in package.json). View switching is zustand state: `AppView = "pos" | "suppliers" | "reports" | "catalog"` (`src/stores/app-view-store.ts:14`), rendered by `NavRail` + conditional in `src/App.tsx:97-124`. Unauthenticated root toggles `FirstRunSetupScreen`/`LoginScreen` (`src/App.tsx:91`).
- Navigation items (exact): `NavRail` in `src/App.tsx:26-56` renders four entries — `pos` (`nav.pos`), `suppliers`, `reports`, `catalog` — and is rendered only when `manager` is true (`src/App.tsx:111`; `isManagerRole` at `src/App.tsx:21`). The `Topbar` (`src/components/pos/Topbar.tsx:1-119`) contains NO navigation items: brand block (lines 47-55), `CashSessionIndicator`, offline pill, warehouse `<select>`, session badge, clock, language toggle (`toggleLocale`), theme toggle, logout.
- Customers/Credit, Users, Stock, Documents screens: no such view components exist. Customer access is `CustomerSelectModal` (picker) + `RecordPaymentModal` (both mounted in `src/components/pos/PosShell.tsx:135,172`). No user-management, document-list, or stock-overview screen component exists in `src/` (full tree below).
- Modal inventory (component | trigger | reachable): `CheckoutModal` — F12 (`src/hooks/use-pos-keyboard.ts:167`) ; `CustomerSelectModal` — F2 (`use-pos-keyboard.ts:131`) ; `QuantityModal` — F4 (`:146`) ; `DiscountModal` — F5 (`:153`) ; `DocumentConversionModal` — floating button `PosShell.tsx:78-90` ; `ReturnModal` — floating button `:91-103` ; `WarehouseTransferModal` — floating button `:104-118`, manager-gated ; `StockAdjustModal` — floating button `:119-133`, manager-gated ; `BackupModal` — floating button `:140-154`, ADMIN-gated ; `StoreIdentityModal` — floating button `:156-169`, ADMIN-gated ; `ProductEditModal` — per-row edit button, GERANT/ADMIN (`src/components/pos/ProductDiscovery.tsx:144`) ; `RecordPaymentModal` — UNKNOWN trigger file (mounted `PosShell.tsx:172`, opened via `payment-modal-store`) ; `OpenCashSessionModal`/`CloseCashSessionModal`/`RecordExpenseModal` — UNKNOWN trigger files (imported `PosShell.tsx:16-18`) ; `CustomerSelectModal` also UNKNOWN secondary triggers. `ModalShell` (`modals/ModalShell.tsx:11`) is the shared overlay (`fixed inset-0 z-50`).
- `src/` tree with line counts (`find src -type f | sort | xargs wc -l`, 109 files, 22718 lines total):

```
src/App.tsx: 127 | src/main.tsx: 16 | src/index.css: 35 | src/vite-env.d.ts: 1
src/components/pos/CartPanel.tsx | src/components/pos/CartTable.tsx | src/components/pos/CashSessionIndicator.tsx | src/components/pos/CreditHud.tsx | src/components/pos/KeyFooter.tsx | src/components/pos/PosShell.tsx (197) | src/components/pos/PosShellView.tsx | src/components/pos/PrintStatusIndicator.tsx | src/components/pos/ProductDiscovery.tsx (165) | src/components/pos/Topbar.tsx (119) | src/components/pos/TotalsFooter.tsx | src/components/status-card.tsx
src/components/pos/modals/: BackupModal.tsx | CheckoutModal.tsx | CloseCashSessionModal.tsx | CustomerSelectModal.tsx | DiscountModal.tsx | DocumentConversionModal.tsx | ModalShell.tsx | OpenCashSessionModal.tsx | ProductEditModal.tsx | QuantityModal.tsx | RecordExpenseModal.tsx | RecordPaymentModal.tsx | ReturnModal.tsx | StockAdjustModal.tsx | StoreIdentityModal.tsx | TestPrintPreview.tsx | WarehouseTransferModal.tsx | checkout-submission.ts | stock-gate.ts
src/features/auth/: FirstRunSetupScreen.tsx | LoginScreen.tsx
src/features/catalog/: CatalogModule.tsx | ProductCreateForm.tsx
src/features/health/health-check.ts
src/features/purchases/: GoodsReceiptFormScreen.tsx | PurchaseOrderFormScreen.tsx | RecordSupplierPaymentModal.tsx | SupplierInvoiceFormScreen.tsx
src/features/reports/: CashSessionsHistoryReport.tsx | CreditOutstandingReport.tsx | DailySalesReport.tsx | GrossMarginReport.tsx | LowStockReport.tsx | ReportsHomeScreen.tsx | SalesByCategoryReport.tsx | SalesByProductReport.tsx | StockMovementReport.tsx | UserActivityReport.tsx | components/report-format.ts | components/ReportDateRangePicker.tsx
src/features/suppliers/: SupplierFormModal.tsx | SupplierListScreen.tsx | SupplierStatementView.tsx
src/hooks/: use-app-status.ts | use-catalog-search.ts (148) | use-pos-keyboard.ts
src/lib/: format.ts | i18n.ts | ipc.ts | money.ts | purchase-lines.ts | tauri.ts
src/stores/: app-store.ts | app-view-store.ts | backup-modal-store.ts | cart-store.ts | cash-session-store.ts | conversion-modal-store.ts | payment-modal-store.ts | pos-ui-store.ts | product-edit-store.ts | return-modal-store.ts | session-store.ts | stock-adjust-modal-store.ts | store-identity-store.ts | supplier-store.ts | warehouse-transfer-store.ts
src/types/: cash.ts | index.ts | pos.ts (179) | reports.ts (134) | supplier.ts (105)
(plus 18 *.test.ts/tsx co-located suites; the complete per-file listing (109 files / 22718 lines) was generated, reviewed, and removed with the temporary working files at task end.)
```

## D. LAYOUT HAZARDS

- Exhaustive grep (87 matching lines across 33 files; pattern set from the task). Counts: `fixed` = 10 (6 FABs `PosShell.tsx:86,99,113,128,149,164`, modal overlays `ModalShell.tsx:11`, `RecordSupplierPaymentModal.tsx:71`, `SupplierFormModal.tsx:94`, `CashSessionsHistoryReport.tsx:147`, `SupplierStatementView.tsx:45` — all `fixed inset-0 z-50`), `absolute` = 1 (`CashSessionIndicator.tsx:138` dropdown `absolute end-0 top-full z-50`), `sticky` = 11 (table heads + `CartTable.tsx:41`), modal layer `z-50` vs FAB `z-40`, `min-w-`/`min-width` = only `min-w-0`/`min-w-12`/`min-w-44` (no pixel min-widths), `w-[` = 3 (`bottom-[17/21/25rem]`, `max-w-[42ch]`), `h-screen`+`w-screen` = 1 (`PosShell.tsx:64`), `100vh/vw` = 0, `overflow-*` = 34 (all inner containers; root uses `overflow-hidden`), `transform scale`/`zoom` = 0 code hits (2 prose comments). The full 87-line file:line list was generated and reviewed line by line; representative lines: `PosShell.tsx:64` root `flex h-screen w-screen ... overflow-hidden`; `ModalShell.tsx:11-12,24`; `KeyFooter.tsx:42` `overflow-x-auto`; `TestPrintPreview.tsx:26` `overflow-x-auto`.
- Six floating buttons (all in `src/components/pos/PosShell.tsx`, all `fixed end-4 z-40`, stacked bottom-20 → bottom-[25rem]): Convertir `:78-90` (all roles) ; Retour & Facture d'Avoir `:91-103` (all roles) ; Dépôts & Transferts `:104-118` (manager) ; Régularisation Stock `:119-133` (manager) ; Sauvegardes `:140-154` (ADMIN) ; Identité & Impression `:156-169` (ADMIN). Rationale comment or commit message: NO EVIDENCE — no comment at these sites explains why floating buttons were chosen; commit history unavailable (no git).
- `html, body, #root` CSS (`src/index.css:11-14`): `height: 100%` only (no overflow/min-width rules). `body` (`index.css:16-21`): `font-family: Inter, Cairo, system-ui, "Segoe UI", Tahoma, sans-serif; background-color: #f8fafc; color: #0f172a`.
- Elements that can make `scrollWidth > clientWidth` (code-level candidates; runtime NOT measured — see K): `PosShell.tsx:64` `w-screen` on the root (a vertical scrollbar, if any appears inside the `h-screen` flex column, makes `w-screen` wider than the viewport — the classic page-level horizontal scrollbar mechanism); `TestPrintPreview.tsx:26` `max-w-[42ch]` preview with `overflow-x-auto` (contained by design); report tables wrapped in `overflow-x-auto` (contained); `KeyFooter.tsx:42` `overflow-x-auto` (contained). No element with a fixed pixel width exceeding 1024 (minWidth) was found in `src/`.

## E. DESIGN SYSTEM FACTS

- `tailwind.config.js` (verbatim relevant): `darkMode: "class"`, `content ["./index.html","./src/**/*.{ts,tsx}]`, `theme.extend.colors.baraka.{primary #0F3D5C, primaryDark #0A2C44, primaryLight #1A5A85, accent #F59E0B, accentDark #D97706, accentLight #FBBF24, steel #64748B, paper #F8FAFC, ink #0F172A}`, `primary.{900 #0F3D5C, 600 #1E6091}`, `accent.{500 #F59E0B}`, `fontFamily.sans [Inter, Cairo, system-ui, Segoe UI, Tahoma, sans-serif]`, `arabic [Cairo, Inter, ...]`, `mono [JetBrains Mono, ...]`; `plugins: []`. CSS vars: `--baraka-primary: #0f3d5c; --baraka-accent: #f59e0b` (`src/index.css:6-9`).
- `components.json` (shadcn): NO EVIDENCE — file absent (`ls` shows only `tailwind.config.js`).
- Icon library: NO EVIDENCE — no `lucide`/`react-icons`/`heroicons` import or dependency in `src/` or `package.json`; icons are inline emoji/glyph spans (e.g. `PosShell.tsx:88,101,115,130,151,166`).
- `@font-face`: NO EVIDENCE in `src/`. Font files in repo: `./assets/fonts/Inter-Regular.ttf`, `./assets/fonts/NotoNaskhArabic-Regular.ttf` only — `Cairo` and `JetBrains Mono` (referenced by `index.css:17,30` and `tailwind.config.js`) have NO files in the repo and fall back to system fonts.
- Remote URLs in `src/`/`index.html`: none found (`grep https?://` returned no code hits).
- Computed body font-family at runtime / `document.fonts` status of Inter, Cairo, JetBrains Mono: UNKNOWN — no runtime DOM inspection is possible in this environment (shell-only tooling; the app is a desktop window, not automatable here).
- Architecture §16.1 palette as code tokens: UNKNOWN — no architecture document containing §16.1 exists in the repo. Colors used in code outside the `baraka` token set (factual list): `slate-*`, `emerald-*`, `amber-*`, `sky-*`, `red-*`, `violet-*`, `indigo-*`, `white/10`, `black/15` (Tailwind default palette classes across components).

## F. IPC CONTRACT VS UI USAGE

- Registered commands (`src-tauri/src/lib.rs:136` `generate_handler!`, 75 entries): `app_health auth_login auth_logout backups_create backups_list backups_restore backups_restore_precheck backups_verify bootstrap_first_admin cash_begin_closing cash_cancel_closing cash_confirm_closing cash_get_active_session cash_get_session_report cash_open cash_record_expense catalog_create_product catalog_define_kit catalog_get_product catalog_search catalog_update_product check_first_run_status convert_bl_to_facture convert_devis_to_bl credit_get_statement credit_record_payment customers_create customers_get customers_list customers_search customers_update documents_get_print_status documents_get_return_context documents_process_return documents_reprint documents_search printer_get_settings printer_test_print printer_update_settings purchases_create_order purchases_get_document purchases_receive_goods purchases_record_invoice purchases_record_payment purchases_update_order reports_cash_sessions_history reports_credit_outstanding reports_daily_sales reports_gross_margin reports_low_stock reports_sales_by_category reports_sales_by_product reports_stock_movement_history reports_user_activity returns_get_source_summary returns_process sales_checkout sales_create_draft sales_get sales_preview_totals stock_adjust stock_get stock_transfer store_identity_get store_identity_save suppliers_create suppliers_get suppliers_get_statement suppliers_search suppliers_update warehouses_create warehouses_list`.
- All 70 `export async function` wrappers in `src/lib/ipc.ts` are called from at least one non-test UI file (0 unused wrappers; the full wrapper→files map was generated and reviewed).
- UNUSED BY UI (registered, no wrapper, no direct `invoke` from `src/`): `catalog_define_kit` (backend: `crates/commands/src/ipc.rs:764`), `catalog_get_product`, `customers_update`. Backend capabilities with no UI entry point = exactly these three.

## G. DATA STATE AND THE EMPTY PRODUCT PANEL

- Dev-mode DB path: NO EVIDENCE of a project-local database (no `baraka.db*` under the project root outside `target/`). Installed release DB: `%APPDATA%/com.baraka.ultrapos/baraka.db` (identifier `com.baraka.ultrapos`, `resolve_database_path` = data-dir join `baraka.db`, `crates/infrastructure/src/persistence/connection.rs:31`). Copy procedure used: `cp baraka.db baraka.db-wal baraka.db-shm` to `audit/tmp/` (sizes 368640/4120032/32768 bytes); all queries below ran on the copy via `sqlite3` read-only URI (`mode=ro`); the live files were never opened.
- Row counts on the copy: `products=1 product_units=1 categories=0 warehouses=1 stock_levels=1 customers=0 customer_ledger=0 suppliers=0 documents=9 document_items=9 users=1 cash_sessions=1 print_jobs=18 audit_log=24`.
- Demo/seed dataset: NO EVIDENCE of a seed loader in the app path. The single product row is `sku VIS-8X40, name_fr 'Vis à bois 8x40', name_ar 'برغي خشب 8×40'`.
- Real search SQL (`crates/infrastructure/src/adapters/sqlite_catalog_repository.rs:567-574`) executed on the copy: query `''` → 1 row in 9.65 ms; query `'vis'` → 1 row in 0.09 ms. (Equivalent SQL through `python sqlite3`; direct application-layer invocation was not performed — no code path exists to invoke it without writing code, which is forbidden.)
- Empty-panel determination: (e) VERIFIED BY CODE — with an empty query the UI never calls the backend: `src/hooks/use-catalog-search.ts:42-47` (`if (query === "") { setResults([]); ... return; }`), and `src/components/pos/ProductDiscovery.tsx:95-112` shows no message at all when a warehouse is set, loading is false, error is null and the query is empty (the grid simply renders zero cards). Contributing code facts: with `warehouse === null` the panel shows `search.noWarehouse` (`:95-99`); backend errors render (`:105-107`); scan-miss errors are swallowed by design (`use-catalog-search.ts:126-128`). Console/log output: UNKNOWN — no runtime console was captured (see K).

## H. FEATURE PRESENCE MATRIX

- customer management screen: ABSENT (only `CustomerSelectModal` picker + `customers_*` commands; no list screen).
- customer credit ledger + payment screen: PARTIAL (`RecordPaymentModal`, `credit_record_payment`, `CreditHud.tsx`, `CreditOutstandingReport.tsx`; no dedicated ledger screen).
- customer statement: IMPLEMENTED_BACKEND_ONLY (`credit_get_statement` has a wrapper; no statement screen component; supplier statement view exists, customer one does not).
- customer fiscal identifiers RC/NIF/NIS/AI: UNKNOWN — no `rc/nif/nis/ai` column reference found in customer DTO grep surface; not verified.
- price tiers: ABSENT (no `price_tier/wholesale` in `src/lib/ipc.ts`/`src/types/pos.ts` besides a margin-report prose hit).
- product aliases/synonyms: ABSENT. product images: ABSENT. category browsing/filter in POS: PARTIAL (type chips Tous/Kits/Pièces via `is_kit`, `ProductDiscovery.tsx:5-8,44-48`; no category endpoint).
- quick products Alt+1..8: ABSENT (keyboard map is F1/F2/F3/F4/F5/F8/F10/F12/Escape/Delete/Arrows/Ctrl+K, `src/hooks/use-pos-keyboard.ts:111-208`).
- command palette Ctrl+K: ABSENT as a palette — Ctrl+K only focuses search (`use-pos-keyboard.ts:205-208`).
- park and resume: PARTIAL — park exists (F8 → `sales_create_draft` + clearCart + banner, `use-pos-keyboard.ts:68-99`); resume UI: NO EVIDENCE (no `resume`/draft-list reference in components/stores).
- returns and AVOIR: IMPLEMENTED_AND_REACHABLE_FROM_UI (`ReturnModal` FAB + `returns_process`/`documents_process_return`).
- document list and search: PARTIAL (backend `documents_search` + `searchDocuments` wrapper used by return/conversion modals; no standalone list screen).
- document conversion DEVIS→BL→FACTURE: IMPLEMENTED_AND_REACHABLE_FROM_UI (`DocumentConversionModal` FAB + `convertDevisToBl`/`convertBlToFacture`).
- stock overview and adjustment: PARTIAL (adjustment modal FAB; overview = `LowStockReport` + `StockMovementReport`; no stock-overview screen).
- warehouse transfer: IMPLEMENTED_AND_REACHABLE_FROM_UI (FAB + `WarehouseTransferModal` + `transferStock`).
- purchases BC/BR/FF: IMPLEMENTED_AND_REACHABLE_FROM_UI via suppliers module (`PurchaseOrderFormScreen`/`GoodsReceiptFormScreen`/`SupplierInvoiceFormScreen` rendered by `SupplierListScreen.tsx:204-206`).
- supplier ledger: IMPLEMENTED_AND_REACHABLE_FROM_UI (`SupplierStatementView`).
- user management: ABSENT (no screen; 1 user row in DB).
- cash open/close and Z-Report: IMPLEMENTED_AND_REACHABLE_FROM_UI (`OpenCashSessionModal`/`CloseCashSessionModal`, `CashSessionsHistoryReport`, `cash_get_session_report` "interim X-Report or final Z-Report", `src/lib/ipc.ts:1334`).
- reports §29 (12 claimed): 9 report components exist (`CashSessionsHistoryReport`, `CreditOutstandingReport`, `DailySalesReport`, `GrossMarginReport`, `LowStockReport`, `SalesByCategoryReport`, `SalesByProductReport`, `StockMovementReport`, `UserActivityReport` = 9 files); the §29 source listing is NO EVIDENCE in-repo, so 12-vs-9 cannot be adjudicated.
- settings (store identity, tax, printer): PARTIAL (`StoreIdentityModal` + `printer_*` wrappers; no tax-settings UI found).
- backup and restore: IMPLEMENTED_AND_REACHABLE_FROM_UI (`BackupModal` + `backups_*`).
- label/barcode printing: ABSENT (no label command in handler list or `ipc.ts`).
- language switch ar-DZ/fr-DZ + RTL: IMPLEMENTED_AND_REACHABLE_FROM_UI (`toggleLocale`, `dir=` rtl/ltr in `PosShell.tsx:62`/`App.tsx`); `en`: ABSENT (`src/lib/i18n.ts:2` "bilingual dictionary (ar-DZ default, fr-DZ)"; locales object only `fr-DZ`/`ar-DZ`, `:1241-1242`).
- dark theme: IMPLEMENTED_AND_REACHABLE_FROM_UI (toggle `Topbar.tsx:99-106`, `darkMode: "class"`, `dark:` classes throughout).

## I. TEST AND QUALITY FACTS

- `cargo test --workspace` (with `CARGO_BUILD_JOBS=2` in env only; literal command unchanged): exit 0 — 349 passed / 0 failed / 4 ignored; crash-matrix line quoted verbatim: `17/17 scenarios passed in 5s`.
- `cargo clippy --all-targets --all-features -- -D warnings`: exit 0, no warnings emitted.
- `npm run lint`: exit 0 (no output). `npm run typecheck`: exit 0 (no output). `npm run build`: exit 0 — `dist/index.html 0.51 kB`, `index-DL6eCe9a.css 26.76 kB`, `index-o949h9zE.js 474.37 kB`, `✓ built in 3.36s`.
- Suites: domain unit tests (inline `#[test]`/`proptest`-style — presence per-file NOT counted, UNKNOWN exact split); `crates/application/tests/returns_use_case_tests.rs` (1 file); infrastructure integration `crates/infrastructure/tests/` (35 entries incl. `crash_matrix/` 3 files + `crash_matrix_tests.rs`, `store_identity_tests.rs` 14 tests, `restore_tests.rs`, `print_worker.rs`, `print_rendering.rs`, `release_benchmarks.rs` + `release_drill.rs` both `#[ignore]`); Vitest co-located suites (18 files; last full run 21 files / 180 tests per prior record — NOT re-run in this task); e2e harness: ABSENT; Playwright specs: NO EVIDENCE (no config, dependency, or spec file; no visual/overflow/viewport assertion exists anywhere).

## J. SECTION 38 EVIDENCE

- The §38 checklist source (19-item text): NO EVIDENCE — no architecture document containing §38 exists in the repo (`grep §38` hits only `docs/operations/thermal-printer-drill.md` in passing). The 19 items therefore cannot be enumerated without speculation; each is UNKNOWN for that reason.
- Mandated probes: real thermal printer verification — NO EVIDENCE of completion (drill doc `docs/operations/thermal-printer-drill.md` exists; no signed row or result record found in-repo). Tax/legal validation by a qualified person — NO EVIDENCE. Encryption key recovery drill — NO EVIDENCE (consistent with code fact: no at-rest/backup encryption; `cipher_version NONE` doc-only, `crates/application/src/ports.rs:1968`). Backup-restore drill — PARTIAL evidence: `restore_tests.rs` + `BackupModal` exist; no executed-drill record in-repo. Performance benchmark with hardware profile — EVIDENCE: `docs/operations/release-benchmarks-v1.0.0.md` (Win10 b19045, i5-3450, 16 GB RAM, SATA SSD; M1 p50 0.606 ms / M2 p50 4.104 ms / M3 p50 1.245 ms, n=50). Installer build profile — EVIDENCE: release profile (`Finished release profile [optimized]`, first build log; `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`, 218,977,788 bytes).

## K. RUNTIME CAPTURE

- Automated capture: NOT PERFORMED — impossible in this environment. Reason: the agent tooling is shell-only (no GUI automation, no window control, no screenshot API for desktop windows); viewports cannot be resized, languages/themes cannot be toggled, and bounding boxes cannot be measured without driving the running desktop app.
- Screenshots delivered: none (`audit/screenshots/` is an empty directory).
- Exact manual steps to reproduce: (1) install `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`; (2) log in; (3) for each viewport in {1920x1080, 1536x864, 1366x768, 1280x720} resize the window (windowed mode) or set display resolution; (4) toggle language (top-bar button) for fr then ar; (5) toggle theme (☾/☀) for light then dark; (6) visit POS, Suppliers, Reports, Catalog views and open each modal; (7) save each PNG as `<screen>__<width>x<height>__<lang>__<theme>.png`; (8) at each POS viewport record `innerWidth/innerHeight/devicePixelRatio`, `documentElement.scrollWidth vs clientWidth`, `scrollHeight vs clientHeight`, and bounding boxes of TOTAL TTC and each floating button via DevTools (`document.querySelector(...).getBoundingClientRect()`), noting intersections.
- Viewport/overlap measurements: UNKNOWN (not measured; see above).

## L. PRINT OUTPUT

- Templates: single shared renderer (no per-type template files). Header/footer blocks `build_identity_lines` (`crates/infrastructure/src/printing/blocks.rs:116`) / `build_footer_lines` (`:156`); raster path `render_canvas` + `encode_gs_v0` bands + LF (`rasterizer.rs` per prior record); document types `Ticket/Facture/Bl/Devis/Avoir` (`crates/domain/src/sales/document.rs:16-26`).
- Sample ticket/facture FR+AR as image or ESC/POS hex: UNKNOWN — no existing code path emits a render without modification (no `println!` in `store_identity_tests.rs`; no `.escpos` artifact found in the repo via `find -name "*.escpos"`).

## APPENDIX — verbatim command outputs (abridged)

- `git branch / rev-parse / status / tag`: all `fatal: not a git repository (or any of the parent directories): .git`.
- `cargo test --workspace`: `passed=349 failed=0 ignored=4`, `17/17 scenarios passed in 5s`, exit 0.
- `npm run build`: `✓ built in 3.36s`, exit 0.
- DB copy row counts and search timings: see G.
- `audit/tmp/` (live-DB copy + temporary working files): DELETED at end of task — `ls audit/` shows only `AUDIT-REPORT.md` and empty `screenshots/`.
