# AUDIT-REPORT-2 — Safety baseline + static evidence audit (TASK-AUD-002)

Read-only except the explicitly permitted writes (snapshot zip, `.git/`, `.gitignore`, `.gitattributes`, this file, deleted `audit/tmp/` scratch). No behavior change. Every claim carries file:line or command output; `UNKNOWN`/`NO EVIDENCE` where absent. No opinions, no recommendations, no fixes. Date 2026-09-30. Working directory: `D:/Dev/Baraka UltraPOS`.

## STEP 0 — SNAPSHOT

- Absolute project root: `D:/Dev/Baraka UltraPOS` (shell `pwd` = `/d/Dev/Baraka UltraPOS`).
- `D:/Baraka Project/Baraka UltraPOS`: EXISTS (top listing: `AGENTS.md assets/ Cargo.lock Cargo.toml CONSTITUTION_OF_EXECUTOR.md ...`) and DIFFERS from the project root: `diff -rq` (excluding `target node_modules dist .git`) reports 50 differing lines, e.g. `Cargo.lock`, `Cargo.toml`, `crates/application/src/{ports.rs,services/print_worker_service.rs,use_cases/backup.rs,use_cases/printing.rs}`, `crates/commands/src/{dto.rs,ipc.rs}`, `crates/domain/src/printing/{mod.rs,receipt_model.rs}`, `crates/infrastructure/Cargo.toml`, `crates/infrastructure/src/adapters/sqlite_backup_manager.rs` differ; `crates/domain/src/printing/decode.rs` and `audit/` exist only under `D:/Dev/Baraka UltraPOS`. Verdict: the `D:/Baraka Project/` tree is an OLDER copy (pre-identity/capture work), not a mirror.
- Snapshot: `D:/Dev/_snapshots/baraka-baseline-20260930-1956.zip` — created via filtered `tar` staging (`--exclude=./target,./node_modules,./dist,./.git,./audit/tmp`) + PowerShell `Compress-Archive`. Files archived: 374 (zip entries 383 incl. dirs). Size: 1,256,149 bytes. SHA-256: `262e21136ebf40259dfec7842ae3b85c0278ac101e11b429fa6130415010d24d`. Zip integrity verified (`ZipFile.OpenRead`, entry count read back). Staging dir removed.

## STEP 1 — GIT BASELINE

- 1.1 Pre-git SHA-256 recorded for all 13 files under `crates/infrastructure/migrations/` + `Cargo.lock` + `package-lock.json` (exists at root, 170,722 staged bytes) + `src-tauri/tauri.conf.json` (full table in §1.7).
- 1.2 `git init` (branch `master`, git default); LOCAL config only: `core.autocrlf=false core.safecrlf=false user.name="Baraka Dev" user.email="dev@baraka.local"` (`git config --local --list` verified; global untouched).
- 1.3 `.gitattributes` created with exactly `* -text` (8 bytes).
- 1.4 `.gitignore`: pre-existing file kept in full; appended `src-tauri/gen/schemas/`, `*.escpos`, `audit/tmp/`. All mandated patterns covered (`target/`, `src-tauri/target/`, `node_modules/`, `dist/`, `*.db`, `*.db-wal`, `*.db-shm` pre-existing lines 7,12,14,37-39). `assets/`, `docs/`, `audit/` (except `audit/tmp/`) not ignored.
- 1.5 Pre-commit: 367 staged files, 3,954,698 staged bytes, 91,213 added lines / 0 deleted. 10 largest staged files (bytes): `assets/fonts/Inter-Regular.ttf` 411640, `assets/fonts/NotoNaskhArabic-Regular.ttf` 199512, `package-lock.json` 170722, `crates/commands/src/ipc.rs` 142746, `Cargo.lock` 137723, `pnpm-lock.yaml` 89345, `src/lib/i18n.ts` 83422, `crates/commands/src/dto.rs` 78598, `crates/application/src/ports.rs` 73687, `src/lib/ipc.ts` 71468. Largest 411,640 bytes < 5 MB — committed.
- 1.6 Commit `05f0f3975f7e2ccd201d4d1795abbf99696b5bb6`, message `baseline: engine complete through TASK-BRK-057 (pre-redesign snapshot)`. Annotated tag `baseline-engine-pre-redesign` = `38fc39a4183be54f5667c830f5e25a41018a8a55`. Tag `v1.0.0` does NOT exist (`git tag` lists only `baseline-engine-pre-redesign`).
- 1.7 Hash comparison (pre-git tree vs post-commit tree vs committed blobs): working-tree pre==post (`diff` of sorted hash lists: `TREES_IDENTICAL`); all 17 committed blobs identical (`ALL_17_BLOBS_IDENTICAL`, loop over `git show <commit>:<path> | sha256`). Table (pre = post = blob):

| file | sha256 |
|---|---|
| migrations/0001_baseline_schema.sql | 4da8fb2d…cb0db1e9a |
| migrations/0002_auth_lockout.sql | 1043eb9f…f38cff9141 |
| migrations/0003_stock_engine_schema.sql | aff4f084…b7a7a43e05a7 |
| migrations/0004_sales_engine_schema.sql | 7cb0ec2c…178621be0a44c83 |
| migrations/0005_customer_module_indexes.sql | efdc92ca…198babdd0be479723a33dd10e05c2f019ce194f77fb14c828 |
| migrations/0006_print_worker_columns.sql | 56d1a1d1…e7e0fac7183237512d88a59f9b |
| migrations/0007_document_language.sql | c93e3ba8…241e9af3413 |
| migrations/0008_cash_session_engine_schema.sql | d97ca09e…c0089888aef |
| migrations/0009_supplier_purchase_engine_schema.sql | 53cdf820…1aaa0fce5dd |
| migrations/0010_reporting_indexes.sql | b235aef2…fad3d779d01b913e32868e4ff |
| migrations/0011_add_converted_document_status.sql | 3e792a4a…92eacc2582e8801 |
| migrations/0012_seed_default_warehouse.sql | f24f4301…3d5ccc1b76ef05d6 |
| migrations/0013_add_refund_out_movement_type.sql | 3cad5d91…a14000408033395 |
| Cargo.lock | 50f1ca73…d88d526e6b86bc208 |
| package-lock.json | 6c08409c…ffd0953f640cc15de56b |
| src-tauri/tauri.conf.json | 4f9d872d…dca70cc9d3efe1b448a45 |

`git ls-files --eol`: 300 `i/lf w/lf`, 14 `i/crlf w/crlf`, 53 `i/-text` — NO file shows index↔worktree conversion (all `i/… w/…` pairs match; `-text` honored). The 14 CRLF files (e.g. `crates/commands/src/dto.rs`, `src/lib/ipc.ts`, `src/App.tsx`) are byte-identical in blob and tree.
- 1.8 Commit `05f0f397…`, tag `38fc39a4…`. `git status --short` shows one entry: `?? docs/architecture/` — a 0-byte file `docs/architecture/New Text Document.txt` created 20:00 (AFTER snapshot 19:57 and commit) by an external actor (editor/file-explorer artifact), NOT by this task; it was not snapshotted, not committed, and not deleted (deletion not permitted). Otherwise empty.

This file (`audit/AUDIT-REPORT-2.md`) was created AFTER the commit and is untracked, as required.

## A. SCHEMA FACTS (from `crates/infrastructure/migrations/*.sql` only)

Verbatim DDL (material excerpts; full texts are the hashed files above):

- `settings` (0001:14-18): `(key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL) STRICT`.
- `users` (0001:21-30): `(id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL, password_hash TEXT NOT NULL, pin_hash TEXT, role TEXT NOT NULL, is_active INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL, updated_at TEXT NOT NULL) STRICT`. Role CHECK: NONE — `role` is free `TEXT`.
- `categories` (0001:33-40): `(id TEXT PRIMARY KEY, code TEXT UNIQUE NOT NULL, name_ar TEXT NOT NULL, name_fr TEXT NOT NULL, parent_id TEXT REFERENCES categories(id), is_active INTEGER NOT NULL DEFAULT 1) STRICT`.
- `products` (0001:43-56): `(id TEXT PRIMARY KEY, sku TEXT UNIQUE NOT NULL, barcode TEXT, name_ar TEXT NOT NULL, name_fr TEXT NOT NULL, category_id TEXT REFERENCES categories(id), is_kit INTEGER NOT NULL DEFAULT 0, is_active INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 1, origin_device_id TEXT NOT NULL) STRICT`. Answers: category reference YES (`category_id`); image column NO; price-tier columns NO (prices live on `product_units`: `sale_price_centimes`, `purchase_price_centimes`, single tier).
- `product_units` (0001:59-68): `(id TEXT PRIMARY KEY, product_id TEXT NOT NULL REFERENCES products(id) ON DELETE RESTRICT, unit_name_ar TEXT NOT NULL, unit_name_fr TEXT NOT NULL, factor_micro INTEGER NOT NULL, barcode TEXT UNIQUE, sale_price_centimes INTEGER NOT NULL, purchase_price_centimes INTEGER NOT NULL, is_base_unit INTEGER NOT NULL, is_default_selling INTEGER NOT NULL) STRICT`.
- `customers` (0001:113-128): `(id TEXT PRIMARY KEY, code TEXT UNIQUE NOT NULL, name TEXT NOT NULL, phone TEXT, address TEXT, credit_policy TEXT NOT NULL, credit_limit_centimes INTEGER NOT NULL DEFAULT 0, current_balance_centimes INTEGER NOT NULL DEFAULT 0, is_active INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 1, origin_device_id TEXT NOT NULL) STRICT`. Answers: RC/NIF/NIS/AI columns NO; address YES; phone YES.
- `tax_rules` (0001:260-268): `(id TEXT PRIMARY KEY, rule_code TEXT NOT NULL, name TEXT NOT NULL, rate_micro INTEGER NOT NULL, effective_from TEXT NOT NULL, effective_to TEXT, is_active INTEGER NOT NULL DEFAULT 1) STRICT`.
- `cash_sessions` authoritative shape is the 0008 rebuild (`cash_sessions_new`, 0008:18-36): `(id TEXT PRIMARY KEY, warehouse_id TEXT NOT NULL REFERENCES warehouses(id), terminal_id TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('OPEN','CLOSING','CLOSED')), opened_by TEXT NOT NULL REFERENCES users(id), opened_at TEXT NOT NULL, opening_float_centimes INTEGER NOT NULL CHECK(opening_float_centimes >= 0), closed_by TEXT NULL REFERENCES users(id), closed_at TEXT NULL, actual_counted_centimes INTEGER NULL CHECK(...>= 0), system_expected_centimes INTEGER NULL, variance_centimes INTEGER NULL, closing_notes TEXT NULL, revision INTEGER NOT NULL DEFAULT 1, origin_device_id TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now'))) STRICT`. (The 0001:233-247 shape is superseded by the 0008 table swap.)
- Other tables (names only, DDL in hashed files): `schema_migrations, product_kit_items, warehouses, stock_levels, stock_movements, customer_ledger, suppliers, supplier_ledger, documents, document_items, payments, cash_movements, document_sequences, print_jobs, command_log, audit_log, backups_log` (+ `*_new` rebuild tables in 0008/0009/0013).
- Triggers (19): `prevent_customer_ledger_update/delete`, `prevent_supplier_ledger_update/delete`, `prevent_stock_movements_update/delete`, `prevent_cash_movements_update/delete` (0001 + re-created in 0008:134-140), `prevent_payments_update/delete`, `prevent_audit_log_update/delete`, `prevent_command_log_update/delete`, `prevent_documents_update_finalized` (+ `prevent_documents_delete_finalized` 0004:23), `prevent_document_items_update/delete_finalized`, `trg_cash_sessions_immutable_when_closed` (0008:119), `trg_cash_sessions_no_delete` (0008:127).
- Indexes: `idx_products_sku/barcode`, `idx_product_units_product/barcode`, `idx_stock_movements_product_warehouse`, `idx_customer_ledger_customer`, `idx_supplier_ledger_supplier`, `idx_document_items_document`, `idx_payments_document/customer`, `idx_cash_movements_session`, `idx_audit_log_entity`, `idx_print_jobs_document` (0001:327-342); `idx_stock_movements_transfer_batch` (0003:33); `idx_customers_phone_unique` UNIQUE + `idx_customers_name` (0005:13-16); `idx_cash_sessions_active_per_terminal` UNIQUE (0008:112).
- Settings keys seeded by migrations: NONE (no `INSERT INTO settings` in any migration). Settings keys exist only in code defaults: `store.name/address/phone/nif/nis/rc/ai/footer_message` + `printing.device.kind` (`sqlite_printer_settings.rs:89-105`); `printer.serial_port`/`printer.serial_baud` read with `unwrap_or_default` (`:46-54`) — i.e. no seeded rows, reads default to empty.
- Seeded `tax_rules` rows: NONE (no `INSERT INTO tax_rules` in migrations or `crates/**/*.rs`). Only seeded row in all migrations: warehouse `01950000-0000-7000-8000-000000000001 / DEP-01 / Dépôt Principal` (0012, `WHERE NOT EXISTS`).
- Users role values: schema allows any TEXT; code enum `Role {Vendeur, Caissier, Gerant, Admin}` (`crates/domain/src/role.rs:13-22`, semantics: Vendeur counter-sales; Caissier +cash; Gerant +overrides/returns/stock/purchases/reports; Admin +users/settings/backup).

## B. PERMISSION MAP (72 registered commands; method below)

Method: handler file:line from `crates/commands/src/ipc.rs`; use-case from `XxxUseCase::new` in the handler body; permission literals from the use-case `impl` block (test modules stripped) plus file-level `require_*` gate helpers (`catalog.rs:23-37`, `reports.rs:58`, `backup.rs:258-261`); DTO names from handler signatures. `perm=[]` = no `Permission::` gate in the use-case impl block (session token may still be required by the handler arg). Role→permission mapping is `Role::has_permission` (`role.rs`); file-level policy comments override ambiguous rows (notably catalog: mutations `StockAdjustment/SystemSettingsManage`, reads `PosSell` — `catalog.rs:1-4`).

| command | handler | use-case | permission(s) | in-DTO | out-DTO |
|---|---|---|---|---|---|
| app_health | ipc.rs:182 | — | — | — | HealthDto |
| auth_login | :555 | AuthenticateUserUseCase | — | LoginRequestDto | LoginResponseDto |
| auth_logout | :579 | LogoutUseCase | — | LogoutRequestDto | ? |
| check_first_run_status | :596 | — | — | — | CheckFirstRunStatusResponse |
| bootstrap_first_admin | :615 | BootstrapFirstAdminUseCase | — | BootstrapFirstAdminRequest | BootstrapFirstAdminResponse |
| catalog_create_product | :641 | CreateProductUseCase (+Get) | PosSell* | CreateProductDto | ProductResponseDto |
| catalog_update_product | :683 | UpdateProductUseCase (+Get) | PosSell* | UpdateProductDto | ProductResponseDto |
| catalog_get_product | :721 | GetProductWithUnitsUseCase | PosSell | GetProductDto | ProductResponseDto |
| catalog_search | :736 | SearchCatalogUseCase | PosSell | CatalogSearchRequestDto | Vec<ProductResponseDto> |
| catalog_define_kit | :766 | DefineKitUseCase | —(row unattributed; file policy: mutation gate) | DefineKitDto | ? |
| warehouses_create | :794 | CreateWarehouseUseCase | ManageWarehouses | CreateWarehouseRequestDto | WarehouseDto |
| warehouses_list | :817 | ListWarehousesUseCase | — | — | ListWarehousesResponseDto |
| stock_get | :840 | GetStockLevelUseCase | — | GetStockLevelRequestDto | GetStockLevelResponseDto |
| stock_adjust | :888 | AdjustStockUseCase | AdjustStock, NegativeStockOverride | AdjustStockRequestDto | AdjustStockResponseDto |
| stock_transfer | :923 | TransferStockUseCase | TransferStock | TransferStockRequestDto | TransferStockResponseDto |
| sales_checkout | :976 | CheckoutSaleUseCase | CreateSale, CreditLimitOverride, NegativeStockOverride (+PriceOverride line 83) | CheckoutSaleRequest | CheckoutSaleResponse |
| sales_preview_totals | :1088 | PreviewSaleTotalsUseCase | CreateSale | PreviewTotalsRequest | PreviewTotalsResponse |
| convert_devis_to_bl | :1142 | ConvertDevisToBlUseCase | CreateSale, NegativeStockOverride | ConvertDevisToBlRequest | ConvertDevisToBlResponse |
| convert_bl_to_facture | :1179 | ConvertBlToFactureUseCase | CreateSale, CreditLimitOverride | ConvertBlToFactureRequest | ConvertBlToFactureResponse |
| documents_search | :1226 | SearchDocumentsUseCase | CreateSale, ReportsView | DocumentSearchRequest | DocumentSearchResponse |
| returns_get_source_summary | :1266 | — (repo-direct) | — | — | ReturnableDocumentSummaryDto |
| returns_process | :1383 | ProcessReturnUseCase | ProcessReturn | ProcessReturnRequestDto | ProcessReturnResponseDto |
| documents_get_return_context | :1485 | — (repo-direct) | — | — | ReturnContextResponseDto |
| documents_process_return | :1601 | ProcessSaleReturnUseCase | —(no literal in block) | DocumentsProcessReturnRequestDto | DocumentsProcessReturnResponseDto |
| backups_create | :1723 | CreateBackupUseCase | —(admin enforced at handler/store layer; use-case block has no literal) | CreateBackupRequestDto | BackupMetadataDto |
| backups_list | :1754 | ListBackupsUseCase | — | — | Vec<BackupMetadataDto> |
| backups_verify | :1768 | VerifyBackupIntegrityUseCase | SystemSettingsManage | VerifyBackupRequestDto | VerifyBackupResponseDto |
| backups_restore_precheck | :1920 | PrecheckRestoreUseCase | SystemSettingsManage | PreRestoreRequestDto | PreRestoreReportDto |
| backups_restore | :1938 | RestoreBackupUseCase | SystemSettingsManage | RestoreBackupRequestDto | RestoreOutcomeDto |
| sales_create_draft | :1957 | CreateDraftSaleUseCase | CreateSale, PriceOverride | CreateDraftSaleRequest | DraftSaleResponse |
| sales_get | :2023 | GetSaleUseCase | — | — | GetSaleResponse |
| customers_create | :2087 | CreateCustomerUseCase | CreateCustomer, ManageCustomerCredit | NewCustomerRequest | CustomerDetailDto |
| customers_update | :2124 | UpdateCustomerUseCase | CreateCustomer, ManageCustomerCredit | UpdateCustomerRequest | CustomerDetailDto |
| customers_search | :2174 | SearchCustomersUseCase (+Get) | ViewCustomers | SearchCustomersRequest | SearchCustomersResponse |
| customers_get | :2220 | — (repo-direct) | — | GetCustomerRequestDto | CustomerDetailDto |
| customers_list | :2233 | ListCustomersUseCase | ViewCustomers | — | SearchCustomersResponse |
| credit_record_payment | :2248 | RecordCreditPaymentUseCase | ManageCustomerCredit, RecordCreditPayment | RecordCreditPaymentRequest | RecordCreditPaymentResponse |
| credit_get_statement | :2275 | GetCustomerStatementUseCase | ViewCustomers | GetStatementRequest | GetStatementResponse |
| documents_get_print_status | :2303 | GetPrintStatusUseCase | — | — | PrintStatusResponse |
| documents_reprint | :2334 | ReprintDocumentUseCase | ReprintDocument | ReprintRequest | ReprintResponse |
| printer_get_settings | :2353 | — (settings-direct) | — | — | PrinterSettingsDto |
| printer_update_settings | :2371 | — (settings-direct) | — | UpdatePrinterSettingsRequest | PrinterSettingsDto |
| store_identity_get | :2394 | — (settings-direct) | — | — | StoreIdentityResponse |
| store_identity_save | :2416 | SaveStoreIdentityUseCase | SystemSettingsManage | SaveStoreIdentityRequest | StoreIdentityResponse |
| printer_test_print | :2449 | TestPrintUseCase | SystemSettingsManage | TestPrintRequest | TestPrintResponse |
| cash_open | :2564 | OpenCashSessionUseCase | CashSessionOpen | OpenCashSessionRequestDto | CashSessionDto |
| cash_begin_closing | :2587 | BeginClosingUseCase | CashSessionClose | CashClosingCommandDto | CashSessionDto |
| cash_cancel_closing | :2600 | CancelClosingUseCase | CashSessionClose | CashClosingCommandDto | CashSessionDto |
| cash_confirm_closing | :2613 | ConfirmClosingUseCase | CashSessionClose | ConfirmClosingRequestDto | CashSessionDto |
| cash_get_active_session | :2634 | GetActiveSessionUseCase | — | — | Option<CashSessionDto> |
| cash_record_expense | :2647 | RecordExpenseUseCase | CashExpenseRecord | RecordExpenseRequestDto | CashMovementDto |
| cash_get_session_report | :2667 | GetSessionReportUseCase | CashReportView | GetSessionReportRequestDto | CashSessionReportDto |
| suppliers_create | :2767 | CreateSupplierUseCase | SupplierManage | CreateSupplierRequestDto | SupplierDto |
| suppliers_update | :2791 | UpdateSupplierUseCase | SupplierManage | SupplierIdRequestDto→(Update req) | SupplierDto |
| suppliers_get | :2817 | GetSupplierUseCase | SupplierManage | SupplierIdRequestDto | SupplierDto |
| suppliers_search | :2831 | SearchSuppliersUseCase | SupplierManage | SearchSuppliersRequestDto | SearchSuppliersResponseDto |
| suppliers_get_statement | :2847 | GetSupplierStatementUseCase | SupplierManage | SupplierIdRequestDto | SupplierStatementDto |
| purchases_create_order | :2878 | CreatePurchaseOrderUseCase | PurchaseOrderCreate | CreatePurchaseOrderRequestDto | PurchaseOrderDto |
| purchases_update_order | :2906 | UpdatePurchaseOrderUseCase | PurchaseOrderCreate | UpdatePurchaseOrderRequestDto | PurchaseOrderDto |
| purchases_receive_goods | :2938 | ReceiveGoodsUseCase | PriceOverride, PurchaseReceive | ReceiveGoodsRequestDto | PurchaseOrderDto |
| purchases_record_invoice | :2976 | RecordSupplierInvoiceUseCase | PurchaseInvoiceRecord | RecordSupplierInvoiceRequestDto | PurchaseOrderDto |
| purchases_record_payment | :3016 | RecordSupplierPaymentUseCase | SupplierPaymentRecord | RecordSupplierPaymentRequestDto | SupplierPaymentOutcomeDto |
| purchases_get_document | :3044 | GetPurchaseDocumentUseCase | SupplierManage | SupplierIdRequestDto | PurchaseOrderDto |
| reports_daily_sales | :3071 | GetDailySalesSummaryUseCase | ReportsView | ReportDateRangeRequestDto | Vec<DailySalesSummaryDto> |
| reports_sales_by_product | :3084 | GetSalesByProductUseCase | ReportsView | ReportDateRangeRequestDto | Vec<SalesByProductRowDto> |
| reports_sales_by_category | :3107 | GetSalesByCategoryUseCase | ReportsView | ReportDateRangeRequestDto | Vec<SalesByCategoryRowDto> |
| reports_gross_margin | :3129 | GetGrossMarginReportUseCase | ReportsView | ReportDateRangeRequestDto | Vec<GrossMarginRowDto> |
| reports_credit_outstanding | :3169 | GetCreditOutstandingReportUseCase | ReportsView | — | Vec<CreditOutstandingRowDto> |
| reports_stock_movement_history | :3205 | GetStockMovementHistoryUseCase | ReportsView | StockMovementHistoryRequestDto | Vec<StockMovementRowDto> |
| reports_cash_sessions_history | :3240 | GetCashSessionsHistoryUseCase | ReportsView | CashSessionsHistoryRequestDto | Vec<CashSessionHistoryRowDto> |
| reports_low_stock | :3271 | GetLowStockReportUseCase | ReportsView | LowStockRequestDto | Vec<LowStockRowDto> |
| reports_user_activity | :3294 | GetUserActivityLogUseCase | ReportsView | UserActivityRequestDto | Vec<UserActivityRowDto> |

*`catalog_create/update` rows carry `PosSell` only via the shared `GetProductWithUnitsUseCase` in the same handler; the file policy (`catalog.rs:1-4`) governs mutations. Registration: 72 `baraka_commands::ipc::` entries in `src-tauri/src/lib.rs:136` block; command name = fn name (`#[tauri::command(rename_all="snake_case")]`, e.g. `ipc.rs:181`). (The "75" figure in the prior report included 3 awk noise tokens — corrected here to 72.)

## C. TEST-TO-COMPONENT COUPLING (21 Vitest files)

Method: static `it(` count + import analysis; VIEW-COUPLED = imports a `.tsx` component or queries DOM roles/text.

VIEW-COUPLED (4): `backup-modal.test.tsx` 14 (BackupModal, react-dom/server) ; `conversion-modal.test.ts` 17 (DocumentConversionModal) ; `record-payment-wire.test.ts` 11 (RecordPaymentModal) ; `create-product-wire.test.ts` 3 (ProductCreateForm).
STORE/LIB-ONLY (17): `checkout-double-submit.test.ts` 4 (checkout-submission helper) ; `checkout-stock-gate.test.ts` 7 (stock-gate helper) ; `product-edit-modal.test.ts` 9 (product-edit-store) ; `return-modal.test.tsx` 12 (stores only, no DOM query) ; `stock-adjust-modal.test.tsx` 8 ; `warehouse-transfer-modal.test.ts` 10 ; `ipc-backups.test.ts` 12 ; `ipc-conversion.test.ts` 8 ; `ipc-documents.test.ts` 6 ; `ipc-preview-totals.test.ts` 3 ; `ipc-printing.test.ts` 5 ; `ipc-product-update.test.ts` 4 ; `ipc-returns.test.ts` 13 ; `ipc-warehouses.test.ts` 5 ; `money.test.ts` 10 ; `backup-modal-restore.test.ts` 5 ; `store-identity-store.test.ts` 9.
Static `it(` total ≈ 178 vs last green run 180 (delta: parameterized cases counted once statically; method limit stated).

## D. RESOLVED UNKNOWNS

- RecordPaymentModal trigger: `src/components/pos/CreditHud.tsx:38-44` (`openPayment({...})` + `openModal("payment")`) and `src/components/pos/modals/CustomerSelectModal.tsx:137-143` (same pair).
- OpenCashSessionModal trigger: `src/components/pos/CashSessionIndicator.tsx:87` (`openModal("cash-open")`).
- CloseCashSessionModal trigger: `CashSessionIndicator.tsx:105` and `:153` (`openModal("cash-close")`).
- RecordExpenseModal trigger: `CashSessionIndicator.tsx:143` (`openModal("cash-expense")`).
- Park/resume: F8 calls `parkSale()` (`use-pos-keyboard.ts:163`), which `ipcCreateDraft(...)` then `cart.clearCart()` + banner (`:68-99`). After F8 the draft row persists server-side and the cart is empty. List/load/resume/discard drafts: NO EVIDENCE — no draft-list command among the 72; `sales_get` has no non-test UI caller; no `resume`/draft UI reference in components/stores.
- CustomerSelectModal CAN create a customer: `ipcCreateCustomer(token, {...})` at `CustomerSelectModal.tsx:181` (imported `:16`; search at `:68`, get at `:102,:135`). No UI edits or lists customers beyond this picker: sole non-test caller file of `ipcCreateCustomer/ipcSearchCustomers/ipcListCustomers` is `CustomerSelectModal.tsx`.
- Categories/users/import creation via UI: NO EVIDENCE — no `createCategory/create_category/categories_create/import*product*/createUser/users_create` reference in components/features (only `FirstRunSetupScreen.tsx:16,32` calls `bootstrap`). Product creation exists (`ProductCreateForm.tsx:236` → `ipcCreateProduct`); kit creation has no UI (`catalog_define_kit` UNUSED BY UI, prior report §F).

## E. DATA-DIRECTORY AND STARTUP FACTS

- Data directory resolution: exactly one place — `src-tauri/src/lib.rs:68-74` (`app.app_data_dir()` → `ensure_database_dir` → `resolve_database_path` = join `baraka.db`); `backup_dir = app_data_dir.join("backups")` (`:116`). `std::env::var` / `std::env::args` / `args()`: NO occurrences in `src-tauri/src/`. Verdict: the database path CANNOT be redirected without code changes.
- Single-instance lock: NO EVIDENCE (no `single_instance`/`SingleInstance`/lock in `src-tauri/src/*.rs` or `src-tauri/Cargo.toml`; only `SystemClock` substring hits).
- Pre-login screens on fresh data dir (`src/App.tsx:66-90`): loading splash (`cash.loading` string) while `requiresSetup === null`; then `FirstRunSetupScreen` if users table empty else `LoginScreen`. Backend unreachable (browser preview): `.catch` sets `requiresSetup=false` → login form (`:71-77`).

## F. FORMATTING AND FONT FACTS

- Money: `formatMoney(centimesStr, locale)` (`src/lib/money.ts:44-53`) → `"1 250,00 DZD"` (fr-DZ) / `"1 250,00 د.ج"` (ar-DZ); digits built from `BigInt.toString()` — LATIN (0-9) in BOTH languages; only the suffix localizes. Quantity: `formatQty` (`:59-70`) trims micro decimals, Latin digits, `.` separator. Parsing accepts `,` or `.` (`parseQtyToMicro :73-74`, `parseMoneyToCentimes :89-90`). `formatDzd` (`src/lib/format.ts:11`) exists alongside.
- `dist/` contents: `index.html`, `assets/index-DL6eCe9a.css`, `assets/index-o949h9zE.js` — css/js only; NO font files, NO images in `dist/`.
- `C:/Windows/Fonts` (365 entries): PRESENT — `Inter-Regular.ttf`, `NotoNaskhArabic-{Regular,Medium,SemiBold,Bold}.ttf`, `tahoma.ttf/tahomabd.ttf`, `arial*.ttf`, `segoeui*.ttf`. ABSENT — `Cairo*`, `JetBrains*` (grep `cairo|jetbrains` returns nothing). So the `Cairo`/`JetBrains Mono` stacks fall back to system fonts on this machine.

## G. FORENSIC DOCUMENT DIGESTS

- `FIRST_LIVE_SALE_TICK_2026_000002_FORENSIC_AUDIT.md` — symptom: first live sale `TICK-2026-000002` correctness under question (schema v12 DB, read-only). Stated outcome: all 10 invariants PASS, key sentence: `| Exact money reconciliation | **PASS** — TTC/tendered/change recomputed to the centime |` with findings `CRITICAL-1` (payment display divergence) and `HIGH-2` (negative-stock policy caveat) (§8 table, :151-165).
- `LOGIN_PIPELINE_TRACE_AUDIT.md` — symptoms S1 (post-bootstrap lands on LoginScreen) + S2 (login click: nothing observable). Stated root cause (ranked #1): `Never-settling invoke("auth_login") latching authBusy=true` — `the *only* mechanism yielding zero error text AND zero navigation AND persistence across clicks; fully matches S2` (:554-558). Outcome: no fix authorized; runtime instrumentation recommended.
- `PRODUCT_CATALOG_PIPELINE_AUDIT.md` — symptom: `searching "a" returns "Aucun produit trouvé"` on fresh DB. Stated root cause: zero rows, not a broken query; key sentence: `Conclusion: the path is **correctly wired, no broken query, no wrong table, no FTS index mismatch**. "Aucun produit trouvé" for "a" is the truthful output of a correct query over zero rows` (:123-126). Classification (b): backend exists, frontend missing/orphaned.
- `WAREHOUSE_LIFECYCLE_AUDIT.md` — symptom: empty Dépôt selector on fresh DB. Stated root cause: `warehouses` empty on fresh installs — `no migration seeds it and no UI can invoke warehouses_create yet` (Q2, :46-51). Outcome path: migration 0012 (DEP-01 seed) now exists in-tree; fix-option analysis at :129.
- BRK-039 `Payment Overpayment / Change Attribution`: fact-finding only; conclusion line not isolated by grep — recorded as conclusion NOT extracted (method limit), status `fact-finding only — no code changed`.
- BRK-040 `Document Conversion IPC Contracts`: `Frontend wrappers in src/lib/ipc.ts: completely absent for both` (conversion wrappers gap).
- BRK-041 `Document Retrieval / Query Capabilities`: `Lookup by human number — GAP`; `List/search by type and status — GAP`.
- BRK-044 `Returns & Avoir Architecture`: AVOIR storage/numbering design captured (`"AVOIR"`, gapless `AVO` prefix arms).
- BRK-048 `Catalog, Stock & Warehouse Capabilities`: `UI gap: no adjust modal exists and no TS wrapper exists` (at audit time; since superseded in-tree).
- BRK-049 `Catalog Product Editing & Multi-Warehouse`: `Verdict: backend READY, needs IPC wiring (one wrapper) + edit modal.`
- BRK-050 `Suppliers, Purchases, Reporting & Backup`: `Backup, encryption & maintenance — GAP (largest in this audit)`; purchase reference-cost design gap noted.

## H. DOCUMENT PRESENCE

- `docs/architecture/CONSTITUTION-v1.0.md`: DOES NOT EXIST. `docs/architecture/MASTER-ARCHITECTURE-v1.0.0.md`: DOES NOT EXIST. `docs/architecture/` contains only `New Text Document.txt` (0 bytes, post-commit external artifact — see §1.8). Line counts: N/A. Baseline commit: NOT included (`git ls-files docs/architecture/` empty; dir post-dates the commit).
- Corollary: the §16.1 palette, §29 report list, and §38 checklist sources are confirmed absent from the committed tree.

## Acceptance notes

- Snapshot/root/git steps: §§0–1.8 above. `v1.0.0` tag NOT created (rule 4 held).
- Live DB and `%APPDATA%` never opened (rule 3 held); application never launched; no network; no installs/updates (rule 2 held).
- No project file modified: `.gitignore` appended (permitted 1b), `.gitattributes` + `.git/` created (permitted), report + `audit/tmp/` scratch only.
