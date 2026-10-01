# AUD-011 — Presentation-layer inventory (read-only facts)

Date: 2026-10-01. HEAD: 1c07aeb824e3411c32a9d26c7db39f9e0411bedd. Tree clean; no code changed.
Method: grep/read only. No builds, no test runs, no app launch. Counts state the exact command used.
Conventions: `path:line` evidence. UNKNOWN = not mechanically determinable. No judgments, no recommendations.

## Commands used for counts

- LOC: `wc -l <file>` (per-file; tree list also verified with Node line splits).
- Occurrence counts: `grep -rn <pattern> src --include='*.ts' --include='*.tsx'` (tests excluded unless stated).
- IPC usage: quoted short command name (`['"`]name['"`]`) in `src/` excluding `*.test.*`.
- i18n keys: lines 8..627 of `src/lib/i18n.ts` (the `fr` dictionary block), pattern `^\s*"([^"]+)"\s*:`.
- Test counts: occurrences of `it("` / `it('` / `` it(` `` / `test("` etc. in `*.test.ts(x)` (static count; see I8 for the runner-number difference).

## Appendix A — ADR-077 Amendment 1 (verbatim, docs/adr/ADR-077 lines 22-29)

ADR-077: 7107 bytes, 29 lines, SHA-256 `24f4a3c9211d3bb91519b356bb58b9d93d4b6ad1dff8ba1a49fb0eb357b104f1` (sha256sum and Node crypto agree, 64 hex).

```text
22|## Amendment 1 (REL-010): duplicate rules, price, codes, digits, export safety
23|A. Manual entry duplicate rule: the normalized French name key (D3, D5) is enforced on every create and rename, not only on import. A duplicate is REJECTED (owner decision); the UI offers two actions: open the existing product, or change the name (for example add the brand to the name). A warning-only mode is not allowed.
24|B. Duplicates inside the import file: the first occurrence in file order is imported; later occurrences are skipped and listed in the report with their row numbers.
25|C. In D7 "price" means the SELLING price: a valid selling price is required. The purchase price is optional; when absent it is stored as absent, not as zero.
26|D. Codes: the export contains both the barcode and the internal code columns. On import, a barcode or internal code present in the file is kept when it is valid and not already used by another product or earlier in the file; otherwise the row is rejected and listed in the report. When the file has no code, Baraka generates it (D4 otherwise unchanged). A barcode identifies exactly one product; duplicate barcodes are never accepted, in entry or in import.
27|E. Digits in search: Arabic-Indic digits (U+0660-U+0669) and Extended Arabic-Indic digits (U+06F0-U+06F9) are mapped to 0-9 in stored search keys and in queries (consistent with ADR-069). This extends the D3 normalization.
28|F. Export safety: in text columns of CSV and xlsx exports (name, Arabic alias, foreign reference, category, unit, and similar), a cell whose first character is =, +, - or @ is neutralized: xlsx writes it as a string-type cell; CSV prefixes a single quote. On import exactly one leading single quote is removed only when it is followed by one of those four characters. Numeric columns (money, quantities) are written as plain numbers and are not altered.
29|G. The future implementation task must include test vectors for A to F.
```

## Appendix B — ADR index tail

`grep -n '^## ' docs/adr/INDEX.md` returns lines 5, 26, 51, 61 (`## 4. ADR-066..076 - SESSION-ARC-001 (Architect, Charter v2.0)`).
Last 15 lines of docs/adr/INDEX.md (file lines 64-78):

```text
64|| ADR-066 | Viewport baseline and window policy | ACCEPTED |
65|| ADR-067 | Single top bar; no floating action buttons | ACCEPTED |
66|| ADR-068 | Tax regime IFU: no VAT in v1 | ACCEPTED (validation gate OPEN) |
67|| ADR-069 | Locales fr + ar, Latin digits always | ACCEPTED |
68|| ADR-070 | Output formats: thermal 58, thermal 80, A4 | ACCEPTED |
69|| ADR-071 | Deployment topology: single device, single instance | ACCEPTED |
70|| ADR-072 | Catalog migration from Dolisoft export | ACCEPTED (format pending profiling) |
71|| ADR-073 | Canonical repository location | ACCEPTED |
72|| ADR-074 | Destructive-action confirmation pattern | ACCEPTED |
73|| ADR-075 | Catalog import scope: products only | ACCEPTED |
74|| ADR-076 | Isolated data directory override (BARAKA_DATA_DIR) | ACCEPTED |
75|| ADR-077 | Product entry priority, French name with optional Arabic alias, generic import/export | ACCEPTED |
76|
77|Next free id: ADR-078.
78|(line 78 = trailing newline segment; `wc -l` = 77)
```

## I1 — Constitutional modules

Passage (docs/architecture/CHARTER-v2.0.md:181, verbatim; §9.2 line 225 adds the reachability rule quoted in I1 notes):

```text
181| 3. Information architecture: a navigation map covering ALL ten constitutional modules (Vente, Stock, Clients & Crédit, Fournisseurs & Achats, Produits & Codes-barres, Documents, Caisse & Clôture, Rapports, Utilisateurs, Paramètres & Sauvegarde), per-role visibility, and the path (clicks/keys) to each job. No feature may be reachable only through a floating element.
```

Module table (screen = file with LOC; stores; IPC wrappers used; status):

- Vente | src/components/pos/PosShell.tsx 198 + 12 modal files (CheckoutModal 640, QuantityModal 93, DiscountModal 98, CustomerSelectModal 371, RecordPaymentModal 341, ProductDiscovery 166, CartPanel 63, CartTable 157, TotalsFooter 43, CreditHud 197, KeyFooter 61, Topbar 120) | cart-store, pos-ui-store, payment-modal-store, conversion-modal-store | ipcCheckout, ipcPreviewTotals, ipcSearchCatalog, ipcSearchCustomers, ipcRecordCreditPayment | EXISTS (src/App.tsx:119-121 default view)
- Stock | StockAdjustModal 454, WarehouseTransferModal 429, LowStockReport 114, StockMovementReport 178 | stock-adjust-modal-store (adjustStock, ipcGetStock, ipcSearchCatalog, listWarehouses), warehouse-transfer-store (createWarehouse, ipcSearchCatalog, listWarehouses, transferStock) | ipcGetStock, ipcSearchCatalog, listWarehouses, transferStock, adjustStock | EXISTS
- Clients & Crédit | CustomerSelectModal 371, RecordPaymentModal 341, CreditOutstandingReport 180, CreditHud 197 | cart-store (setCustomer), conversion-modal-store | ipcCreateCustomer, ipcSearchCustomers, ipcGetCustomer, ipcListCustomers, ipcRecordCreditPayment, ipcGetStatement | EXISTS
- Fournisseurs & Achats | SupplierListScreen 247, SupplierFormModal 143, SupplierStatementView 115, PurchaseOrderFormScreen 273, GoodsReceiptFormScreen 141, SupplierInvoiceFormScreen 171, RecordSupplierPaymentModal 163 | supplier-store (ipcCreateSupplier, ipcSearchSuppliers, ipcUpdateSupplier) | ipcCreateSupplier, ipcUpdateSupplier, ipcGetSupplier, ipcSearchSuppliers, ipcGetSupplierStatement, ipcCreatePurchaseOrder, ipcUpdatePurchaseOrder, ipcReceiveGoods, ipcRecordSupplierInvoice, ipcRecordSupplierPayment, ipcGetPurchaseDocument | EXISTS
- Produits & Codes-barres | CatalogModule 134, ProductCreateForm 444, ProductEditModal 258 | product-edit-store (updateProduct) | ipcSearchCatalog, ipcCreateProduct, updateProduct | EXISTS
- Documents | DocumentConversionModal 378, ReturnModal 446 | conversion-modal-store (convertBlToFacture, convertDevisToBl, getDocumentDetails, searchDocuments), return-modal-store (getDocumentReturnContext, processSaleReturn, searchDocuments) | convertBlToFacture, convertDevisToBl, getDocumentDetails, searchDocuments, getDocumentReturnContext, processSaleReturn, documents_search, sales_get | EXISTS
- Caisse & Clôture | OpenCashSessionModal 128, CloseCashSessionModal 267, RecordExpenseModal 150, CashSessionIndicator 164, CashSessionsHistoryReport 237 | cash-session-store (ipcCashGetActiveSession) | ipcCashOpen, ipcCashBeginClosing, ipcCashCancelClosing, ipcCashConfirmClosing, ipcCashGetActiveSession, ipcCashRecordExpense, ipcCashGetSessionReport | EXISTS
- Rapports | ReportsHomeScreen 95 + 9 reports (DailySales 130, SalesByProduct 95, SalesByCategory 99, GrossMargin 113, LowStock 114, CreditOutstanding 180, StockMovement 178, CashSessionsHistory 237, UserActivity 193) | none (components call ipc wrappers directly) | ipcDailySales, ipcSalesByProduct, ipcSalesByCategory, ipcGrossMargin, ipcCreditOutstanding, ipcLowStock, ipcStockMovementHistory, ipcCashSessionsHistory, ipcUserActivity | EXISTS
- Utilisateurs | no screen file found (`grep -rli 'user.*manag|manage.*user|create.*user|add.*cashier' src --include='*.tsx'` (excluding tests): empty); only LoginScreen 88, FirstRunSetupScreen 110, session-store (ipcLogin, ipcLogout, ipcBootstrapFirstAdmin, ipcListWarehouses) | ipcLogin, ipcLogout, ipcBootstrapFirstAdmin | MISSING (no user list/create/edit screen; only auth entry screens)
- Paramètres & Sauvegarde | StoreIdentityModal 245, BackupModal 587, TestPrintPreview 59, PrintStatusIndicator 122 | store-identity-store (getStoreIdentity, saveStoreIdentity, printingTestPrint), backup-modal-store (createBackup, listBackups, precheckRestoreBackup, restoreBackup, verifyBackup) | store_identity_get, store_identity_save, printer_get_settings, printer_update_settings, printer_test_print, backups_create, backups_list, backups_verify, backups_restore_precheck, backups_restore, documents_get_print_status, documents_reprint | EXISTS

Notes: the charter passage names module titles only, no sub-functions; therefore PARTIAL (a named function without UI) cannot be determined mechanically — function-level completeness is UNKNOWN for all ten rows. Reachability rule: CHARTER-v2.0.md:225 (all ten reachable from the shell by role). `grep -rn 'fixed'` notes: ProductCreateForm.tsx:7 states `is_kit is fixed false (kit UI deferred)`.

## I2 — Shell and navigation

- No router library: src/App.tsx:1-6 header comment states "no router dependency"; view switching is store state (`src/stores/app-view-store.ts:11`: `AppView = "pos" | "suppliers" | "reports" | "catalog"`, default view set at :18-22).
- Gates: `UnauthenticatedGate` (src/App.tsx:62-92) probes `ipcCheckFirstRunStatus` (App.tsx:67) then shows `FirstRunSetupScreen` or `LoginScreen` (App.tsx:91); authenticated branch at App.tsx:107-124; `isManagerRole` = GERANT|ADMIN (App.tsx:21-23); `NavRail` rendered only for managers (App.tsx:111); non-manager sessions fall through to `<PosShell />` (App.tsx:119-121).
- Layout containers: top bar `src/components/pos/Topbar.tsx:1-8` ("56px top bar: brand, operator badge, warehouse selector, sync pill"); shell grid `src/components/pos/PosShell.tsx:1-8` ("56px topbar, 7/5 main grid, 56px key..."); modal host = conditional renders in `src/components/pos/PosShell.tsx:134-155` (checkout, customer, conversion, edit-product, warehouse-transfer, stock-adjust, backup, ...); toast host: NONE — `useShellNotices`/`pushNotice` in `src/stores/app-store.ts:33-43` have zero usages outside that file.
- Top-level screens/views (path, LOC, purpose from the file's own header comment):
  - src/components/pos/PosShell.tsx 198 — sovereign POS cockpit shell (default view).
  - src/features/auth/LoginScreen.tsx 88 — industrial login screen when no session is active.
  - src/features/auth/FirstRunSetupScreen.tsx 110 — first-run setup while the users table is empty.
  - src/features/catalog/CatalogModule.tsx 134 — catalog module root (creation flow only).
  - src/features/catalog/ProductCreateForm.tsx 444 — product creation form (creation flow only).
  - src/features/suppliers/SupplierListScreen.tsx 247 — suppliers module root (search, create/edit).
  - src/features/purchases/PurchaseOrderFormScreen.tsx 273 — BC creation screen.
  - src/features/purchases/GoodsReceiptFormScreen.tsx 141 — goods receipt (BR), against BC or standalone.
  - src/features/purchases/SupplierInvoiceFormScreen.tsx 171 — supplier invoice (FF), against BR or standalone.
  - src/features/reports/ReportsHomeScreen.tsx 95 — reports home dashboard, nine tiles.
  - Entry: src/main.tsx 17 lines (StrictMode render of App; throws if #root missing). index.html references the root (not re-verified here beyond build artifacts).

## I3 — IPC surface

- Registered: 72 commands in `tauri::generate_handler![...]` at `src-tauri/src/lib.rs:192`; all 72 defined in `crates/commands/src/ipc.rs` (`grep -c 'tauri::command'` = 72; per-command `fn <name>` match 72/72). Expected 72 — matches.
- Frontend usage (`["'`]name["'`]` in src/ excluding *.test.*): USED 69, UNUSED 3.
- UNUSED: `catalog_get_product`, `catalog_define_kit`, `customers_update` (zero quoted occurrences in src/).
- Groups by prefix (short names): app_health, auth{login,logout}, check_first_run_status, bootstrap_first_admin, catalog_{create_product,update_product,get_product,search,define_kit}, warehouses_{create,list}, stock_{get,adjust,transfer}, sales_{checkout,preview_totals,create_draft,get}, convert_{devis_to_bl,bl_to_facture}, documents_{search,get_print_status,reprint,get_return_context,process_return}, returns_{get_source_summary,process}, backups_{create,list,verify,restore_precheck,restore}, customers_{create,update,search,get,list}, credit_{record_payment,get_statement}, printer_{get_settings,update_settings,test_print}, store_identity_{get,save}, cash_{open,begin_closing,cancel_closing,confirm_closing,get_active_session,record_expense,get_session_report}, suppliers_{create,update,get,search,get_statement}, purchases_{create_order,update_order,receive_goods,record_invoice,record_payment,get_document}, reports_{daily_sales,sales_by_product,sales_by_category,gross_margin,credit_outstanding,stock_movement_history,cash_sessions_history,low_stock,user_activity}.
- First-use locations for USED commands live in `src/lib/ipc.ts` wrapper functions (49 `export async function ipc*` at lines 97-2000) and a few direct call sites (e.g. `ipcCheckFirstRunStatus` at src/App.tsx:67). Full per-command first-use list: 69 entries — available on request; the 3 UNUSED above are the only ones without any quoted occurrence.
- Wrapper naming caveat: some wrappers drop the `ipc` prefix (`createBackup`, `searchDocuments`, `updateProduct`, `transferStock`, `adjustStock`, `listWarehouses`, `createWarehouse`, `convertBlToFacture`, `convertDevisToBl`, `getDocumentDetails`, `getDocumentReturnContext`, `processSaleReturn`, `getStoreIdentity`, `saveStoreIdentity`, `printingTestPrint`); usage was matched on command-name strings, UNKNOWN dynamic-invoke risk remains for computed names (none observed).

## I4 — Stores

15 store files under src/stores (LOC from tree list in I2/I5 sources). Per file: state fields (interface), IPC wrappers imported from @/lib/ipc, persists:

- app-store.ts (48): notices via useSyncExternalStore (pushNotice/useShellNotices); IPC no; persists no.
- app-view-store.ts (22): view, setView; IPC no; persists no.
- backup-modal-store.ts (373): modal + restore state machine (isOpen, restorePhase, restoreReport, restoreOutcome, confirmationInput, ...); IPC: createBackup, listBackups, precheckRestoreBackup, restoreBackup, verifyBackup; persists no.
- cart-store.ts (179): lines, globalDiscountCentimes, customer, docType + cart actions; IPC no; persists no.
- cash-session-store.ts (50): session, loading, error, refresh/setSession/clear; IPC: ipcCashGetActiveSession; persists no.
- conversion-modal-store.ts (405): search/selection/tender state; IPC: convertBlToFacture, convertDevisToBl, getDocumentDetails, searchDocuments; persists no.
- payment-modal-store.ts (30): openPayment, target, clearPayment; IPC no; persists no.
- pos-ui-store.ts (59): locale, activeModal, searchQuery, selectedLineIndex, lastReceipt, banner + setters; IPC no; persists no (no zustand/middleware persist, no localStorage in prod code).
- product-edit-store.ts (305): product form + unitsSnapshot; IPC: updateProduct; persists no.
- return-modal-store.ts (377): return context/quantities/refund; IPC: getDocumentReturnContext, processSaleReturn, searchDocuments; persists no.
- session-store.ts (99): session, warehouses, warehouse, auth flags; IPC: ipcLogin, ipcLogout, ipcBootstrapFirstAdmin, ipcListWarehouses; persists no.
- stock-adjust-modal-store.ts (316): adjustment inputs/computed delta; IPC: adjustStock, ipcGetStock, ipcSearchCatalog, listWarehouses; persists no.
- store-identity-store.ts (319): identity form + caps + test-print; IPC: getStoreIdentity, saveStoreIdentity, printingTestPrint; persists no.
- supplier-store.ts (64): suppliers, selected, search/create/update/select/clear; IPC: ipcCreateSupplier, ipcSearchSuppliers, ipcUpdateSupplier; persists no.
- warehouse-transfer-store.ts (328): transfer lines/warehouses/tabs; IPC: createWarehouse, ipcSearchCatalog, listWarehouses, transferStock; persists no.
- Persistence statement: `grep -rn 'zustand/middleware|createJSONStorage|persist(' src` = empty; `grep -rn 'localStorage' src` (prod) = empty. No store persists anything.

## I5 — Shared components and hooks

- src/components tree (path, LOC, importing-file count by basename substring match — approximate):
  pos/CartPanel 63 ~1, CartTable 157 ~1, CashSessionIndicator 164 ~2, CreditHud 197 ~1, KeyFooter 61 ~1, PosShell 198 ~4, PosShellView 45 ~1, PrintStatusIndicator 122 ~1, ProductDiscovery 166 ~1, Topbar 120 ~1, TotalsFooter 43 ~1, status-card 23, modals: BackupModal 587, CheckoutModal 640, CloseCashSessionModal 267, CustomerSelectModal 371, DiscountModal 98, DocumentConversionModal 378, ModalShell 29, OpenCashSessionModal 128, ProductEditModal 258, QuantityModal 93, RecordExpenseModal 150, RecordPaymentModal 341, ReturnModal 446, StockAdjustModal 454, StoreIdentityModal 245, TestPrintPreview 59, WarehouseTransferModal 429, checkout-submission.ts 81, stock-gate.ts 46.
- src/hooks: use-app-status.ts 38, use-catalog-search.ts 149 (global keydown at :138), use-pos-keyboard.ts 223 (global keydown at :217; focus helper at :23).
- src/lib (non-test): format.ts 17, i18n.ts 1486, ipc.ts 2339 (49 ipc* wrappers + plain-name wrappers), money.ts 151, purchase-lines.ts 61 (shared purchase-line helpers, centime strings), tauri.ts 61.
- src/types: cash.ts 61, index.ts 24, pos.ts 180 (Locale at :8), reports.ts 135, supplier.ts 106.

## I6 — Styling baseline

(a) tailwind.config.js (35 lines, verbatim):

```text
1|/** @type {import('tailwindcss').Config} */
2|export default {
3|  content: ["./index.html", "./src/**/*.{ts,tsx}"],
4|  darkMode: "class",
5|  theme: {
6|    extend: {
7|      colors: {
8|        baraka: {
9|          primary: "#0F3D5C",
10|          primaryDark: "#0A2C44",
11|          primaryLight: "#1A5A85",
12|          accent: "#F59E0B",
13|          accentDark: "#D97706",
14|          accentLight: "#FBBF24",
15|          steel: "#64748B",
16|          paper: "#F8FAFC",
17|          ink: "#0F172A",
18|        },
19|        primary: {
20|          900: "#0F3D5C",
21|          600: "#1E6091",
22|        },
23|        accent: {
24|          500: "#F59E0B",
25|        },
26|      },
27|      fontFamily: {
28|        sans: ["Inter", "Cairo", "system-ui", "Segoe UI", "Tahoma", "sans-serif"],
29|        arabic: ["Cairo", "Inter", "system-ui", "Tahoma", "sans-serif"],
30|        mono: ["JetBrains Mono", "ui-monospace", "SFMono-Regular", "Menlo", "monospace"],
31|      },
32|    },
33|  },
34|  plugins: [],
35|};
```

(b) Global CSS: src/index.css only (36 lines; full text verified by read). Custom properties defined: `--baraka-primary`, `--baraka-accent` (2 names).
(c) Counts (command: `grep -rn <re> src --include='*.ts' --include='*.tsx'` unless noted; tests excluded):
- Hard-coded colors `#[0-9a-fA-F]{3,8}|rgb(|hsl(` in src ts/tsx: 0. In src/index.css: 4 (`#0f3d5c`, `#f59e0b` variable defs; `#f8fafc`, `#0f172a` body rules). tailwind.config.js palette: 12 hex values (config, outside src/).
- `fixed` Tailwind class: every occurrence of `grep -rn 'fixed' src` (14 total): ModalShell.tsx:11 (overlay div), PosShell.tsx:86,99,113,128,149,164 (6 floating action buttons), RecordSupplierPaymentModal.tsx:71, CashSessionsHistoryReport.tsx:147, SupplierFormModal.tsx:94, SupplierStatementView.tsx:45 (overlay divs), TotalsFooter.tsx:2 (comment), ProductCreateForm.tsx:7 (comment), DocumentConversionModal.tsx:29 (comment), ipc.ts:1044 (comment). `position: fixed` / `position:fixed`: 0.
- Physical-direction utilities (189) vs logical (153) with the exact re above; top-15 physical files: BackupModal 14, ReturnModal 14, StockAdjustModal 14, WarehouseTransferModal 12, DocumentConversionModal 10, CloseCashSessionModal 8, CheckoutModal 7, CreditHud 6, CustomerSelectModal 6, RecordExpenseModal 6, RecordPaymentModal 6, PurchaseOrderFormScreen 6, RecordSupplierPaymentModal 6, ProductEditModal 5, FirstRunSetupScreen 5. Logical top: CashSessionsHistoryReport 20, CreditOutstandingReport 16, DailySalesReport 12, GrossMarginReport 11, SalesByProductReport 10, StockMovementReport 10, UserActivityReport 10, SupplierListScreen 10, LowStockReport 8, SalesByCategoryReport 7, SupplierStatementView 7, PosShell 6, CartTable 5, CashSessionIndicator 3, CreditHud 3.
- `dark:` variants: 558 (top: BackupModal 49, WarehouseTransferModal 33, StockAdjustModal 32, ReturnModal 31, DocumentConversionModal 29).
- Fixed-pixel utilities `[Npx]`: 92 total = `[11px]` 58, `[10px]` 28, `[44px]` 5, `[13px]` 1 (command: `grep -rho '\[[0-9]+px\]' src --include='*.tsx' --include='*.ts' | sort | uniq -c | sort -rn`).
- `overflow-` on root containers: src/components/pos/PosShell.tsx:64 `overflow-hidden` on the root flex column (which also carries `h-screen w-screen`); html/body/#root height 100% in index.css:10-14; no other overflow- on App/main shells.
- font-family declarations: index.css:17 (body Inter/Cairo stack), :30 (.font-arabic Cairo-first), :34 (.font-mono-baraka JetBrains Mono); tailwind config sans/arabic/mono above.
(d) Arabic-Indic digits U+0660-U+0669/U+06F0-U+06F9 in src/ outside i18n: 0 (`grep -rn '[٠-٩۰-۹]' src`, i18n.ts skipped).

## I7 — I18N

Keys use `.` as the hierarchy separator (`I18nKey = keyof typeof fr`, src/lib/i18n.ts:627; `ar: Record<I18nKey, string>` at :629 enforces parity at compile time). French dictionary: 604 keys, 73 distinct first-segment prefixes:

backups 81, rep 73, settings 42, returns 35, stockAdjust 30, conversion 30, return 28, pur 27, cash 23, checkout 22, sup 22, cat 22, payment 19, customer 17, cart 15, key 11, setup 8, topbar 7, search 7, print 7, login 6, totals 5, credit 5, discount 5, nav 4, quantity 3, common 3, app 2, plus 45 singleton prefixes (=1 each): warehousesTitle, tabTransfer, tabCreateWarehouse, sourceWarehouse, destinationWarehouse, sameWarehouseError, selectSourceHelp, selectDestHelp, addItemToTransfer, transferQty, transferNotesPlaceholder, executeTransfer, transferSuccess, warehouseCode, warehouseName, createWarehouseBtn, createWarehouseSuccess, noItemsInTransfer, adminOnlyTab, transferProductSearch, transferNoResults, transferRemoveItem, transferSubmitting, transferClose, transferCancel, editProductTitle, skuImmutableNotice, productNameAr, productNameFr, sellingPrice, costPrice, barcode, unitName, productCategory, activeStatus, inactiveWarning, saveChanges, cancelEdit, savingChanges, updateSuccess, invalidPrice, nameRequired, editProduct, marginLabel, parked. (73 prefixes; counts sum to 604 exactly; no key text reproduced per the task rule.)

## I8 — Tests

22 `*.test.ts(x)` files under src/ (list with static `it(`/`test(` counts from `grep -c` per file): backup-modal.test.tsx 14, checkout-double-submit 4, checkout-stock-gate 7, conversion-modal 17, product-edit-modal 9, record-payment-wire 11, return-modal 12, stock-adjust-modal 8, warehouse-transfer-modal 10, create-product-wire 3, ipc-backups 12, ipc-conversion 8, ipc-documents 6, ipc-preview-totals 3, ipc-printing 5, ipc-product-update 4, ipc-returns 13, ipc-warehouses 5, money 10, backup-modal-restore 5, pos-ui-store 3, store-identity-store 9. Subtotals per directory: components/pos/modals 92, features/catalog 3, lib 66, stores 17. Static grand total: 178 occurrences. Vitest-reported total was 183 at HEAD 97d84aae (plus 3 new pos-ui tests since); the static count undercounts because `it.each(...)` (record-payment-wire.test.ts:146) and per-row expansion are not literal `it("` occurrences. vitest was NOT re-run in this task.

## I9 — Money handling

Display/parse functions (path:line + signature):
- src/lib/money.ts:44 `formatMoney(centimesStr: string, locale): string` (BigInt string arithmetic)
- src/lib/money.ts:59 `formatQty(microStr: string): string`
- src/lib/money.ts:73 `parseQtyToMicro(input: string): string`
- src/lib/money.ts:89 `parseMoneyToCentimes(input: string): string` (accepts 1250.50 / 1250,50 / 1 250,50 styles; no float)
- src/lib/money.ts:107 `addBig`, :112 `subBig`, :117 `maxZero`, :126 `mulQtyPrice`, :138 `creditUsageBp` (all string/bigint)
- src/lib/format.ts:11 `formatDzd(centimes: number): string` — guards `Number.isInteger`, then `dzdFormatter.format(centimes / 100)` at :15
- src/features/reports/components/report-format.ts:9 `formatBasisPoints(bp: string): string` (+ `isIntString` at :19); file header comment (:5) states "no parseFloat, no Number() on financial magnitudes"

Pattern occurrences near money identifiers (within 3 lines of price|amount|total|centimes|balance|credit|payment|cash|change), command `grep -rn` per pattern, tests excluded:
- `toFixed(`: 1 — src/stores/backup-modal-store.ts:47 `value.toFixed(value < 10 ? 2 : 1)` (duration display; nearMoney=false).
- `parseFloat(`: 0.
- `Number(`: 9 — CreditHud.tsx:139 `Number(usage / 100n)` (bigint percent, not money); StockMovementReport:51,165, UserActivityReport:79,180 (PAGE_SIZE pagination); report-format.ts:5 (comment); ipc.ts:1960-1961,2011-2012 (limit/offset pagination). None converts money.
- `* 100`: 2 — money.ts:98 `(whole * 100n ...)` BigInt; conversion-modal-store.ts:59 same BigInt pattern.
- `/ 100`: 9 — all BigInt (`/ 100n`) except src/lib/format.ts:15 `centimes / 100` (Number division feeding Intl formatter; input guarded integer).
- `Math.round(`: 0.
Plain statement: one visible floating-point division on money exists — `src/lib/format.ts:15` (`centimes / 100` on a `number` guarded by `Number.isInteger`, formatted by `Intl.NumberFormat("fr-DZ")`). All other conversion sites use BigInt/string arithmetic. No `parseFloat` on money. Whether `centimes/100` can lose precision for large values: the guard ensures integer input; float64 represents integers exactly up to 2^53 — stated as fact, no judgment.

## I10 — Interaction facilities

- Keyboard: global `keydown` listeners in src/hooks/use-pos-keyboard.ts:105,217 (POS shortcuts; :4 comment describes input scoping), src/hooks/use-catalog-search.ts:76,138; modal Escape handlers (window keydown + removeEventListener triples) in BackupModal:410,424,426, CheckoutModal:323,350,352, DocumentConversionModal:91,97,99, ProductEditModal:36,42,44, ReturnModal:78,95,97 (+ input-level :161), StockAdjustModal:86,103,105 (+ input :232), WarehouseTransferModal:58,64,66 (+ input :194); input-level `onKeyDown` in CatalogModule:90, ProductCreateForm:351 (`swallowEnter`), PurchaseOrderFormScreen:77, SupplierListScreen:72.
- Barcode scanner buffering (fast-keystroke accumulation): no dedicated code located by grep (`scan|barcode` hits are field/storage references in ProductEditModal, ProductDiscovery, ProductCreateForm, use-catalog-search, ipc.ts, cart-store, product-edit-store, types/pos.ts, i18n). Scanner-as-keyboard wedge behavior: UNKNOWN (no timing-based buffer found).
- Autofocus/focus: QuantityModal.tsx:62 `autoFocus`; CheckoutModal.tsx:311 `search.focus()`; use-pos-keyboard.ts:23 `element.focus()`.
- Command palette (ADR-067): none found (`grep -rni 'palette' src` empty).
- Toast mechanism: `src/stores/app-store.ts` (`pushNotice`/:33, `useShellNotices`/:41) with zero render/use sites outside that file — no visible toast host. (store-identity-store.ts:258 comment confirms "no toast" for that surface.)
- Modal components (19): BackupModal, CheckoutModal, CloseCashSessionModal, CustomerSelectModal, DiscountModal, DocumentConversionModal, ModalShell (shell at modals/ModalShell.tsx:11 `fixed inset-0`), OpenCashSessionModal, ProductEditModal, QuantityModal, RecordExpenseModal, RecordPaymentModal, ReturnModal, StockAdjustModal, StoreIdentityModal, WarehouseTransferModal (all src/components/pos/modals/), RecordSupplierPaymentModal (features/purchases), SupplierFormModal (features/suppliers); modal state hosts: useBackupModalStore, useConversionModalStore, usePaymentModalStore + pos-ui `activeModal`.
- Confirmation dialogs: typed `RESTORE` phrase gate (`src/stores/backup-modal-store.ts:71` `input.trim() === "RESTORE"`, wired to BackupModal confirm UI at BackupModal.tsx:341-384); sale-on-shortage explicit confirmation (CheckoutModal.tsx:133,168).
- Print triggers: `PrintStatusIndicator` (polls `ipcGetPrintStatus`, reprint via `ipcReprint`, PrintStatusIndicator.tsx:37,59-63); `printingTestPrint` wrapper (store-identity-store); backend print commands documents_get_print_status / documents_reprint / printer_test_print.
