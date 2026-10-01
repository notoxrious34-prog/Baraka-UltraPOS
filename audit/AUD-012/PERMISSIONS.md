# AUD-012 — Permission-per-command truth table, verbatim Appendix A parts, logo file inspection (read-only)

Date: 2026-10-01. HEAD before: fcb60347ec9bdfc86a0f5e654802b14eafc44d49. Tree clean. Method: grep/read/NIH-lib parsing only. No builds, no app launch. Evidence is file:line; NOT FOUND used only when a search genuinely found nothing.

## A. Verbatim parts missing from the DSN-002 report

### A1. docs/design/01-information-architecture.md lines 284-438 verbatim with line numbers
```text
284|        /// Exact permission set for this role (additive hierarchy).
285|        #[must_use]
286|        pub const fn permissions(self) -> &'static [Permission] {
287|            use Permission::{
288|                AdjustStock, BackupRestoreManage, CashExpenseRecord, CashReportView, CashSessionClose,
289|                CashSessionOpen, CashierCollectPayment, CreateCustomer, CreateSale,
290|                CreditLimitOverride, DiscountOverride, ManageCustomerCredit, ManagePrinterSettings,
291|                ManageWarehouses, NegativeStockOverride, PosParkResume, PosSell, PriceOverride,
292|                ProcessReturn, PurchaseInvoiceRecord, PurchaseManage, PurchaseOrderCreate,
293|                PurchaseReceive, RecordCreditPayment, ReportsView, ReprintDocument, StockAdjustment,
294|                SupplierManage, SupplierPaymentRecord, SystemSettingsManage, TransferStock, UserManage,
295|                ViewCustomers, ViewStock,
296|            };
297|            match self {
298|                Self::Vendeur => &[PosSell, PosParkResume, ViewStock, CreateSale, ViewCustomers],
299|                Self::Caissier => &[
300|                    PosSell,
301|                    PosParkResume,
302|                    CashierCollectPayment,
303|                    CashSessionOpen,
304|                    CashSessionClose,
305|                    CashExpenseRecord,
306|                    CashReportView,
307|                    ViewStock,
308|                    CreateSale,
309|                    ViewCustomers,
310|                    CreateCustomer,
311|                    RecordCreditPayment,
312|                    ReprintDocument,
313|                ],
314|                Self::Gerant => &[
315|                    PosSell,
316|                    PosParkResume,
317|                    CashierCollectPayment,
318|                    CashSessionOpen,
319|                    CashSessionClose,
320|                    CashExpenseRecord,
321|                    CashReportView,
322|                    PriceOverride,
323|                    DiscountOverride,
324|                    NegativeStockOverride,
325|                    CreditLimitOverride,
326|                    ProcessReturn,
327|                    StockAdjustment,
328|                    PurchaseManage,
329|                    ReportsView,
330|                    ViewStock,
331|                    AdjustStock,
332|                    TransferStock,
333|                    CreateSale,
334|                    ViewCustomers,
335|                    CreateCustomer,
336|                    ManageCustomerCredit,
337|                    RecordCreditPayment,
338|                    ReprintDocument,
339|                    SupplierManage,
340|                    PurchaseOrderCreate,
341|                    PurchaseReceive,
342|                    PurchaseInvoiceRecord,
343|                    SupplierPaymentRecord,
344|                ],
345|                Self::Admin => &[
346|                    PosSell,
347|                    PosParkResume,
348|                    CashierCollectPayment,
349|                    CashSessionOpen,
350|                    CashSessionClose,
351|                    CashExpenseRecord,
352|                    CashReportView,
353|                    PriceOverride,
354|                    DiscountOverride,
355|                    NegativeStockOverride,
356|                    CreditLimitOverride,
357|                    ProcessReturn,
358|                    StockAdjustment,
359|                    PurchaseManage,
360|                    ReportsView,
361|                    UserManage,
362|                    SystemSettingsManage,
363|                    BackupRestoreManage,
364|                    ViewStock,
365|                    AdjustStock,
366|                    TransferStock,
367|                    ManageWarehouses,
368|                    CreateSale,
369|                    ViewCustomers,
370|                    CreateCustomer,
371|                    ManageCustomerCredit,
372|                    RecordCreditPayment,
373|                    ReprintDocument,
374|                    ManagePrinterSettings,
375|                    SupplierManage,
376|                    PurchaseOrderCreate,
377|                    PurchaseReceive,
378|                    PurchaseInvoiceRecord,
379|                    SupplierPaymentRecord,
380|                ],
381|            }
382|        }
383|    
384|        /// Whether this role grants `permission`.
385|        #[must_use]
386|        pub fn has_permission(self, permission: Permission) -> bool {
387|            self.permissions().contains(&permission)
388|        }
389|
390|Source: src/App.tsx:21-23
391|
392|    function isManagerRole(role: string): boolean {
393|      return role === "GERANT" || role === "ADMIN";
394|    }
395|
396|Source: src/App.tsx:99-121
397|
398|      const manager = session !== null && isManagerRole(session.role);
399|    
400|      return (
401|        <div
402|          dir={locale === "ar-DZ" ? "rtl" : "ltr"}
403|          lang={locale === "ar-DZ" ? "ar" : "fr"}
404|          className={`h-full ${locale === "ar-DZ" ? "font-arabic" : "font-sans"}`}
405|        >
406|          {session === null ? (
407|            <UnauthenticatedGate />
408|          ) : (
409|            <div className="flex h-full flex-col">
410|              {manager && <NavRail view={view} onSelect={setView} />}
411|              <div className="min-h-0 flex-1">
412|                {view === "suppliers" && manager ? (
413|                  <SuppliersModule />
414|                ) : view === "reports" && manager ? (
415|                  <ReportsHomeScreen />
416|                ) : view === "catalog" && manager ? (
417|                  <CatalogModule />
418|                ) : (
419|                  <PosShell />
420|                )}
421|
422|Observations (each one sentence, tied to a row of table 2.2):
423|
424|1. Vente: today App.tsx:119-121 falls every non-manager session through to PosShell with no Vendeur/Caissier distinction, while the engine gives Caissier 13 permissions and Vendeur only 5.
425|2. Documents: no list screen exists although documents_search and the convert/return commands exist, and conversion plus return open only from the two ungated FABs.
426|3. Clients & Crédit: no customer list or edit screen exists and customers_update has no UI caller, against the PROPOSED Clients tab with create/edit.
427|4. Produits & Codes-barres: the catalog module covers the creation flow only with manager-only entry (CatalogModule.tsx:38), while the PROPOSED Liste/Catégories/Import-Export/Étiquettes tabs do not exist.
428|5. Stock: no overview screen exists and Caissier/Vendeur hold only ViewStock with no read-only UI surface, against the PROPOSED R cells.
429|6. Fournisseurs & Achats: the existing screens are manager-gated by the App view switch, matching the PROPOSED N cells for Caissier and Vendeur.
430|7. Caisse & Clôture: the cash modals open from CashSessionIndicator for any authenticated role although Vendeur holds no Cash* permission, against the PROPOSED N cell for Vendeur.
431|8. Rapports: the reports view is manager-gated, matching the PROPOSED N cells, but Caissier holds CashReportView with no reachable report surface.
432|9. Utilisateurs: no screen of any kind exists although UserManage is Admin-only in the engine, against the PROPOSED Admin-only row.
433|10. Paramètres & Sauvegarde: backup and identity open only from ADMIN-gated FABs with no Impression or Langue & affichage sections, against the PROPOSED P cells for Gérant, Caissier and Vendeur.
434|11. The frontend authorizes by role-name string compares (isManagerRole at App.tsx:21-23 with copies at CatalogModule.tsx:19 and warehouse-transfer-store.ts:65, plus inline ADMIN checks) and SessionInfo (src/types/pos.ts:111-117) carries no permission set, so the PROPOSED permission-derived visibility needs the section-10 engine item.
435|12. Caissier lacks the engine ProcessReturn permission yet the return FAB is ungated for every role, so the PROPOSED disabled-with-reason treatment is future work with the backend as decider.
436|
437|## Appendix B - Floating functions evidence (from audit/AUD-011/INVENTORY-C1.md section C2)
438|Six `<button type="button">` elements with `fixed ... end-4 z-40 ... rounded-full` classes in src/components/pos/PosShell.tsx (fixed-class lines 86, 99, 113, 128, 149, 164 per AUD-011). Openers verified exhaustively with `grep -rn 'openModal("'`: conversion, return, warehouse, stock-adjust, backup and identity modals are opened ONLY from these six elements. Cashier (non-manager) does not render the warehouse, stock-adjust, backup and identity buttons (PosShell.tsx role checks at lines 104, 119, 140, 156 per AUD-011-C1).
```

### A2. crates/domain/src/role.rs lines 100-215 verbatim with line numbers
```text
100|            "ADMIN" => Ok(Self::Admin),
101|            other => Err(DomainError::InvalidCanonicalString(format!(
102|                "unknown role code: {other}"
103|            ))),
104|        }
105|    }
106|
107|    /// Exact permission set for this role (additive hierarchy).
108|    #[must_use]
109|    pub const fn permissions(self) -> &'static [Permission] {
110|        use Permission::{
111|            AdjustStock, BackupRestoreManage, CashExpenseRecord, CashReportView, CashSessionClose,
112|            CashSessionOpen, CashierCollectPayment, CreateCustomer, CreateSale,
113|            CreditLimitOverride, DiscountOverride, ManageCustomerCredit, ManagePrinterSettings,
114|            ManageWarehouses, NegativeStockOverride, PosParkResume, PosSell, PriceOverride,
115|            ProcessReturn, PurchaseInvoiceRecord, PurchaseManage, PurchaseOrderCreate,
116|            PurchaseReceive, RecordCreditPayment, ReportsView, ReprintDocument, StockAdjustment,
117|            SupplierManage, SupplierPaymentRecord, SystemSettingsManage, TransferStock, UserManage,
118|            ViewCustomers, ViewStock,
119|        };
120|        match self {
121|            Self::Vendeur => &[PosSell, PosParkResume, ViewStock, CreateSale, ViewCustomers],
122|            Self::Caissier => &[
123|                PosSell,
124|                PosParkResume,
125|                CashierCollectPayment,
126|                CashSessionOpen,
127|                CashSessionClose,
128|                CashExpenseRecord,
129|                CashReportView,
130|                ViewStock,
131|                CreateSale,
132|                ViewCustomers,
133|                CreateCustomer,
134|                RecordCreditPayment,
135|                ReprintDocument,
136|            ],
137|            Self::Gerant => &[
138|                PosSell,
139|                PosParkResume,
140|                CashierCollectPayment,
141|                CashSessionOpen,
142|                CashSessionClose,
143|                CashExpenseRecord,
144|                CashReportView,
145|                PriceOverride,
146|                DiscountOverride,
147|                NegativeStockOverride,
148|                CreditLimitOverride,
149|                ProcessReturn,
150|                StockAdjustment,
151|                PurchaseManage,
152|                ReportsView,
153|                ViewStock,
154|                AdjustStock,
155|                TransferStock,
156|                CreateSale,
157|                ViewCustomers,
158|                CreateCustomer,
159|                ManageCustomerCredit,
160|                RecordCreditPayment,
161|                ReprintDocument,
162|                SupplierManage,
163|                PurchaseOrderCreate,
164|                PurchaseReceive,
165|                PurchaseInvoiceRecord,
166|                SupplierPaymentRecord,
167|            ],
168|            Self::Admin => &[
169|                PosSell,
170|                PosParkResume,
171|                CashierCollectPayment,
172|                CashSessionOpen,
173|                CashSessionClose,
174|                CashExpenseRecord,
175|                CashReportView,
176|                PriceOverride,
177|                DiscountOverride,
178|                NegativeStockOverride,
179|                CreditLimitOverride,
180|                ProcessReturn,
181|                StockAdjustment,
182|                PurchaseManage,
183|                ReportsView,
184|                UserManage,
185|                SystemSettingsManage,
186|                BackupRestoreManage,
187|                ViewStock,
188|                AdjustStock,
189|                TransferStock,
190|                ManageWarehouses,
191|                CreateSale,
192|                ViewCustomers,
193|                CreateCustomer,
194|                ManageCustomerCredit,
195|                RecordCreditPayment,
196|                ReprintDocument,
197|                ManagePrinterSettings,
198|                SupplierManage,
199|                PurchaseOrderCreate,
200|                PurchaseReceive,
201|                PurchaseInvoiceRecord,
202|                SupplierPaymentRecord,
203|            ],
204|        }
205|    }
206|
207|    /// Whether this role grants `permission`.
208|    #[must_use]
209|    pub fn has_permission(self, permission: Permission) -> bool {
210|        self.permissions().contains(&permission)
211|    }
212|}
213|
214|#[cfg(test)]
215|mod tests {
```

### A3. Permission x role matrix (34 Permission variants, from role.rs:27-80; YES/NO from code in A2)
```text
Permission | Vendeur | Caissier | Gerant | Admin
---|---|---|---|---
PosSell | YES | YES | YES | YES
PosParkResume | YES | YES | YES | YES
CashierCollectPayment | NO | YES | YES | YES
CashSessionOpen | NO | YES | YES | YES
CashSessionClose | NO | YES | YES | YES
PriceOverride | NO | NO | YES | YES
DiscountOverride | NO | NO | YES | YES
NegativeStockOverride | NO | NO | YES | YES
CreditLimitOverride | NO | NO | YES | YES
ProcessReturn | NO | NO | YES | YES
StockAdjustment | NO | NO | YES | YES
PurchaseManage | NO | NO | YES | YES
ReportsView | NO | NO | YES | YES
UserManage | NO | NO | NO | YES
SystemSettingsManage | NO | NO | NO | YES
BackupRestoreManage | NO | NO | NO | YES
ViewStock | YES | YES | YES | YES
AdjustStock | NO | NO | YES | YES
TransferStock | NO | NO | YES | YES
ManageWarehouses | NO | NO | NO | YES
CreateSale | YES | YES | YES | YES
ViewCustomers | YES | YES | YES | YES
CreateCustomer | NO | YES | YES | YES
ManageCustomerCredit | NO | NO | YES | YES
RecordCreditPayment | NO | YES | YES | YES
ReprintDocument | NO | YES | YES | YES
ManagePrinterSettings | NO | NO | NO | YES
CashExpenseRecord | NO | YES | YES | YES
CashReportView | NO | YES | YES | YES
SupplierManage | NO | NO | YES | YES
PurchaseOrderCreate | NO | NO | YES | YES
PurchaseReceive | NO | NO | YES | YES
PurchaseInvoiceRecord | NO | NO | YES | YES
SupplierPaymentRecord | NO | NO | YES | YES
Totals: Vendeur=5 Caissier=13 Gerant=29 Admin=34. Expected 5/13/29/34 — matches; Admin = all 34 (role.rs:168-203 test also asserts &ALL at :320).
```

## B. Permission required by each IPC command

Where checks happen: permission checks are performed in the application use cases via `session.role.has_permission(...)`; the single authority is `Role::has_permission` at crates/domain/src/role.rs:209-211, backed by the table `Role::permissions()` at role.rs:107-205. Use cases call it through local helpers: `require` (cash.rs:28, customers.rs:26, purchases.rs:25, suppliers.rs:23), `require_any`/`require` (catalog.rs:23/37), `require_reports` (reports.rs:58), `require_restore_admin` (backup.rs:258) and direct inline `if ![...].has_permission(...)` checks (inventory.rs:103/204/273, printing.rs:43, sales.rs, returns.rs:208). Three commands also check permissions directly in the command layer (ipc.rs): printer_update_settings:2378 (ManagePrinterSettings), store_identity_save:2423 and printer_test_print:2456 (SystemSettingsManage). `auth_*`, `check_first_run_status`, `bootstrap_first_admin` and `app_health` are pre-auth (no session required); commands that read aggregate data with a valid session but no specific permission gate are marked NONE-SESSION-ONLY.
```text
# | command | handler ipc.rs: | use case / check file:line | Permission required | evidence file:line
1 | app_health | 181 | health_snapshot (no session) | NONE-PRE-AUTH | ipc.rs:182 no require_session
2 | auth_login | 555 | AuthenticateUserUseCase:559 | NONE-PRE-AUTH | ipc.rs:559 no require_session
3 | auth_logout | 579 | LogoutUseCase:583 | NONE-SESSION-ONLY | ipc.rs:583-585 (token) 
4 | check_first_run_status | 596 | first-run probe | NONE-PRE-AUTH | ipc.rs:596-614 no require_session
5 | bootstrap_first_admin | 615 | BootstrapFirstAdminUseCase:619 | NONE-PRE-AUTH | ipc.rs:619-624
6 | catalog_create_product | 641 | CreateProductUseCase catalog.rs:195 | StockAdjustment OR SystemSettingsManage (MANAGE) | catalog.rs:23,195
7 | catalog_update_product | 683 | UpdateProductUseCase catalog.rs:266 | StockAdjustment OR SystemSettingsManage (MANAGE) | catalog.rs:266
8 | catalog_get_product | 721 | GetProductWithUnitsUseCase catalog.rs:320 | StockAdjustment OR SystemSettingsManage (MANAGE) | catalog.rs:320
9 | catalog_search | 736 | SearchCatalogUseCase catalog.rs:406 | PosSell | catalog.rs:406
10 | catalog_define_kit | 766 | DefineKitUseCase catalog.rs:448 | PosSell | catalog.rs:448
11 | warehouses_create | 794 | CreateWarehouseUseCase inventory.rs:103 | ManageWarehouses | inventory.rs:103
12 | warehouses_list | 817 | ListWarehousesUseCase inventory.rs:148 | NONE-SESSION-ONLY | inventory.rs:148 (_session)
13 | stock_get | 840 | GetStockLevelUseCase inventory.rs:173 | NONE-SESSION-ONLY | inventory.rs:171 (_session)
14 | stock_adjust | 888 | AdjustStockUseCase inventory.rs:204 | AdjustStock (+ NegativeStockOverride when negative allowed) | inventory.rs:204,224
15 | stock_transfer | 923 | TransferStockUseCase inventory.rs:273 | TransferStock | inventory.rs:273
16 | sales_checkout | 976 | CheckoutSaleUseCase sales.rs:191 | CreateSale (+ PriceOverride/CreditLimitOverride/NegativeStockOverride when those paths are taken) | sales.rs:191,83,257,283
17 | sales_preview_totals | 1088 | PreviewSaleTotalsUseCase sales.rs:458 | CreateSale | sales.rs:458
18 | convert_devis_to_bl | 1142 | ConvertDevisToBlUseCase sales.rs:811 | CreateSale (+ PriceOverride when applicable) | sales.rs:811,83
19 | convert_bl_to_facture | 1179 | ConvertBlToFactureUseCase sales.rs:971 | CreateSale | sales.rs:971
20 | sales_create_draft | 1957 | CreateDraftSaleUseCase sales.rs:639 | CreateSale (+ PriceOverride) | sales.rs:639,659
21 | sales_get | 2023 | GetSaleUseCase sales.rs:1139 | NONE-SESSION-ONLY | sales.rs:1139 (_session)
22 | documents_search | 1226 | SearchDocumentsUseCase sales.rs:1192 | CreateSale OR ReportsView | sales.rs:1192-1193
23 | returns_get_source_summary | 1266 | inline command | NONE-SESSION-ONLY | ipc.rs:1271 require_session only
24 | returns_process | 1383 | ProcessReturnUseCase returns.rs:208 | ProcessReturn | returns.rs:208
25 | documents_get_return_context | 1485 | inline command | NONE-SESSION-ONLY | ipc.rs:1490 require_session only
26 | documents_process_return | 1601 | ProcessSaleReturnUseCase (=ProcessReturnUseCase) returns.rs:208 | ProcessReturn | ipc.rs:1654; returns.rs:208
27 | backups_create | 1723 | CreateBackupUseCase backup.rs:63 | NONE-SESSION-ONLY | backup.rs:63 (_session)
28 | backups_list | 1754 | ListBackupsUseCase backup.rs:162 | NONE-SESSION-ONLY | backup.rs:162 (_session)
29 | backups_verify | 1768 | VerifyBackupIntegrityUseCase backup.rs:218 | NONE-SESSION-ONLY | backup.rs:215 (_session)
30 | backups_restore_precheck | 1920 | PrecheckRestoreUseCase backup.rs:298 | SystemSettingsManage (restore admin) | backup.rs:258,298
31 | backups_restore | 1938 | RestoreBackupUseCase backup.rs:331 | SystemSettingsManage (restore admin) | backup.rs:258,331
32 | customers_create | 2087 | CreateCustomerUseCase customers.rs:87 | CreateCustomer | customers.rs:87
33 | customers_update | 2124 | UpdateCustomerUseCase customers.rs:163 | CreateCustomer (+ ManageCustomerCredit when policy/limit changes) | customers.rs:163,210
34 | customers_search | 2174 | SearchCustomersUseCase customers.rs:246 | ViewCustomers | customers.rs:246
35 | customers_get | 2220 | GetCustomerUseCase customers.rs:279 | ViewCustomers | customers.rs:279
36 | customers_list | 2233 | ListCustomersUseCase customers.rs:309 | ViewCustomers | customers.rs:309
37 | credit_record_payment | 2248 | RecordCreditPaymentUseCase customers.rs:342 | RecordCreditPayment (+ ManageCustomerCredit on overpayment) | customers.rs:342,352
38 | credit_get_statement | 2275 | GetCustomerStatementUseCase customers.rs:392 | ViewCustomers | customers.rs:392
39 | documents_get_print_status | 2303 | GetPrintStatusUseCase printing.rs:74 | NONE-SESSION-ONLY | printing.rs:71 (_session)
40 | documents_reprint | 2334 | ReprintDocumentUseCase printing.rs:43 | ReprintDocument | printing.rs:43
41 | printer_get_settings | 2353 | require_session | NONE-SESSION-ONLY | ipc.rs:2357
42 | printer_update_settings | 2371 | direct ipc.rs check | ManagePrinterSettings | ipc.rs:2378
43 | printer_test_print | 2449 | direct ipc.rs check | SystemSettingsManage | ipc.rs:2456
44 | store_identity_get | 2394 | GetStoreIdentityUseCase printing.rs:154 | NONE-SESSION-ONLY | printing.rs:154 (_session)
45 | store_identity_save | 2416 | direct ipc.rs check (also SaveStoreIdentityUseCase printing.rs:187) | SystemSettingsManage | ipc.rs:2423; printing.rs:187
46 | cash_open | 2564 | OpenCashSessionUseCase cash.rs:84 | CashSessionOpen | cash.rs:84
47 | cash_begin_closing | 2587 | BeginClosingUseCase cash.rs:139 | CashSessionClose | cash.rs:139
48 | cash_cancel_closing | 2600 | CancelClosingUseCase cash.rs:185 | CashSessionClose | cash.rs:185
49 | cash_confirm_closing | 2613 | ConfirmClosingUseCase cash.rs:234 | CashSessionClose | cash.rs:234
50 | cash_get_active_session | 2634 | GetActiveSessionUseCase cash.rs:292 | NONE-SESSION-ONLY | cash.rs:292 (let _ = session)
51 | cash_record_expense | 2647 | RecordExpenseUseCase cash.rs:328 | CashExpenseRecord | cash.rs:328
52 | cash_get_session_report | 2667 | GetSessionReportUseCase cash.rs:378 | CashReportView | cash.rs:378
53 | suppliers_create | 2767 | CreateSupplierUseCase suppliers.rs:71 | SupplierManage | suppliers.rs:71
54 | suppliers_update | 2791 | UpdateSupplierUseCase suppliers.rs:124 | SupplierManage | suppliers.rs:124
55 | suppliers_get | 2817 | GetSupplierUseCase suppliers.rs:168 | SupplierManage | suppliers.rs:168
56 | suppliers_search | 2831 | SearchSuppliersUseCase suppliers.rs:201 | SupplierManage | suppliers.rs:201
57 | suppliers_get_statement | 2847 | GetSupplierStatementUseCase suppliers.rs:231 | SupplierManage | suppliers.rs:231
58 | purchases_create_order | 2878 | CreatePurchaseOrderUseCase purchases.rs:134 | PurchaseOrderCreate | purchases.rs:134
59 | purchases_update_order | 2906 | UpdatePurchaseOrderUseCase purchases.rs:189 | PurchaseOrderCreate | purchases.rs:189
60 | purchases_receive_goods | 2938 | ReceiveGoodsUseCase purchases.rs:249 | PurchaseReceive (+ PriceOverride for over-receipt) | purchases.rs:249,252
61 | purchases_record_invoice | 2976 | RecordSupplierInvoiceUseCase purchases.rs:314 | PurchaseInvoiceRecord | purchases.rs:314
62 | purchases_record_payment | 3016 | RecordSupplierPaymentUseCase purchases.rs:376 | SupplierPaymentRecord | purchases.rs:376
63 | purchases_get_document | 3044 | GetPurchaseDocumentUseCase purchases.rs:448 | SupplierManage | purchases.rs:448
64 | reports_daily_sales | 3071 | GetDailySalesSummaryUseCase reports.rs:104 | ReportsView | reports.rs:104
65 | reports_sales_by_product | 3084 | GetSalesByProductUseCase reports.rs:134 | ReportsView | reports.rs:134
66 | reports_sales_by_category | 3107 | GetSalesByCategoryUseCase reports.rs:164 | ReportsView | reports.rs:164
67 | reports_gross_margin | 3129 | GetGrossMarginReportUseCase reports.rs:195 | ReportsView | reports.rs:195
68 | reports_credit_outstanding | 3169 | GetCreditOutstandingReportUseCase reports.rs:224 | ReportsView | reports.rs:224
69 | reports_stock_movement_history | 3205 | GetStockMovementHistoryUseCase reports.rs:259 | ReportsView | reports.rs:259
70 | reports_cash_sessions_history | 3240 | GetCashSessionsHistoryUseCase reports.rs:296 | ReportsView | reports.rs:296
71 | reports_low_stock | 3271 | GetLowStockReportUseCase reports.rs:327 | ReportsView | reports.rs:327
72 | reports_user_activity | 3294 | GetUserActivityLogUseCase reports.rs:366 | ReportsView | reports.rs:366
```
rows=72. No row left UNKNOWN; no NONE-PRE-AUTH except the 5 pre-auth commands noted. All traces completed from code.

### B3. Role x command matrix for the 20 selected commands (ALLOWED / DENIED / UNKNOWN)
Cells derive from B2 permissions combined with A3.
```text
# | command | Permission(s) | Vendeur | Caissier | Gerant | Admin | also needs
1 | sales_checkout | CreateSale(+overrides) | ALLOWED | ALLOWED | ALLOWED | ALLOWED | open cash session; override perms for price/negative/credit paths
2 | sales_create_draft | CreateSale(+PriceOverride) | ALLOWED | ALLOWED | ALLOWED | ALLOWED | PriceOverride only if unit price override needed
3 | convert_devis_to_bl | CreateSale(+PriceOverride) | ALLOWED | ALLOWED | ALLOWED | ALLOWED | PriceOverride if the source devis requires re-price
4 | convert_bl_to_facture | CreateSale | ALLOWED | ALLOWED | ALLOWED | ALLOWED | none
5 | documents_search | CreateSale OR ReportsView | ALLOWED | ALLOWED | ALLOWED | ALLOWED | none
6 | documents_reprint | ReprintDocument | DENIED | ALLOWED | ALLOWED | ALLOWED | none
7 | documents_process_return | ProcessReturn | DENIED | DENIED | ALLOWED | ALLOWED | manager login (no per-action override)
8 | returns_process | ProcessReturn | DENIED | DENIED | ALLOWED | ALLOWED | manager login
9 | stock_get | NONE-SESSION-ONLY | ALLOWED | ALLOWED | ALLOWED | ALLOWED | valid session
10 | stock_adjust | AdjustStock(+NegativeStockOverride) | DENIED | DENIED | ALLOWED | ALLOWED | NegativeStockOverride only if negative quantity requested
11 | stock_transfer | TransferStock | DENIED | DENIED | ALLOWED | ALLOWED | none
12 | warehouses_list | NONE-SESSION-ONLY | ALLOWED | ALLOWED | ALLOWED | ALLOWED | valid session
13 | warehouses_create | ManageWarehouses | DENIED | DENIED | DENIED | ALLOWED | none
14 | customers_list | ViewCustomers | ALLOWED | ALLOWED | ALLOWED | ALLOWED | none
15 | customers_create | CreateCustomer | DENIED | ALLOWED | ALLOWED | ALLOWED | none
16 | credit_get_statement | ViewCustomers | ALLOWED | ALLOWED | ALLOWED | ALLOWED | none
17 | credit_record_payment | RecordCreditPayment(+ManageCustomerCredit) | DENIED | ALLOWED | ALLOWED | ALLOWED | ManageCustomerCredit only when allow_overpayment=true
18 | backups_create | NONE-SESSION-ONLY | ALLOWED | ALLOWED | ALLOWED | ALLOWED | valid session
19 | store_identity_save | SystemSettingsManage | DENIED | DENIED | DENIED | ALLOWED | none
20 | printer_update_settings | ManagePrinterSettings | DENIED | DENIED | DENIED | ALLOWED | none
```

### B4. Verification of the suspected contradictions
```text
--- Contradiction 1: Vendeur reprint ---
Claim: docs/design 2.2 Documents R for Vendeur = "search, view, reprint only".
Code truth: Permission::ReprintDocument is granted to Caissier/Gerant/Admin only (role.rs:62-63 comment "Enqueue reprint jobs (Caissier, Gerant, Admin; NOT Vendeur)"); Vendeur set role.rs:121 lacks ReprintDocument; ReprintDocumentUseCase denies without it (printing.rs:43).
Evidence: role.rs:62-63, role.rs:121, printing.rs:43. CONFIRMED — contradiction (Vendeur cannot reprint).
--- Contradiction 2: Gérant Impression access ---
Claim: docs/design 2.2 Paramètres P for Gérant = "Impression and Langue & affichage".
Code truth: printer_update_settings requires ManagePrinterSettings (ipc.rs:2378), and ManagePrinterSettings is Admin-only (role.rs:64-65, Admin set role.rs:197). store_identity_save/test_print require SystemSettingsManage (Admin-only, role.rs:185-186). Gérant has no Impression permission.
Evidence: ipc.rs:2378, role.rs:64-65, role.rs:197, ipc.rs:2423/2456. CONFIRMED — contradiction (Gérant cannot change printer settings).
--- Contradiction 3: warehouse creation vs transfers ---
Claim: warehouse creation Admin-only (ManageWarehouses); transfers Gerant+Admin.
Code truth: warehouses_create -> CreateWarehouseUseCase requires ManageWarehouses (inventory.rs:103), Admin-only (role.rs:50-51, Admin set role.rs:190). stock_transfer -> TransferStockUseCase requires TransferStock (inventory.rs:273), granted to Gerant+Admin (role.rs:155). warehouses_list is session-only.
Evidence: inventory.rs:103, role.rs:50-51, role.rs:190, inventory.rs:273, role.rs:155. REFUTED (claim matches code; not a contradiction).
--- Row 4: can a Caissier complete a return without a manager? ---
Code truth: documents_process_return -> ProcessReturnUseCase requires ProcessReturn (returns.rs:208); Caissier has no ProcessReturn (Caissier set role.rs:122-136). Override mechanism: there is no per-action permission override; the only escalation is a user switch via the PIN fast-switch check (crates/application/src/use_cases/mod.rs:198, ports.rs:68 PIN hash slot) or a full login, i.e. a Gerant/Admin session. Evidence: returns.rs:208, role.rs:122-136, mod.rs:198, ports.rs:68. Answer: NO — a Caissier cannot return without a manager; mechanism = separate/fast user switch, no in-session override.
--- Row 5: who can call documents_search / customers_search / catalog_search ---
Code truth: documents_search -> SearchDocumentsUseCase requires CreateSale OR ReportsView (sales.rs:1192-1193) -> any role with CreateSale, i.e. all four (Vendeur has CreateSale, role.rs:121). customers_search -> ViewCustomers (customers.rs:246) -> all four (ViewCustomers in all sets). catalog_search -> PosSell (catalog.rs:406) -> all four (PosSell in all sets). Evidence: sales.rs:1192-1193, customers.rs:246, catalog.rs:406, role.rs:121. Vendeur real reach: all three searches ALLOWED.
```

## C. Frontend gating facts

### C1. Every role-string comparison / isManagerRole call in src/** (non-test)
```text
src/App.tsx:21 function isManagerRole(role:string){ 
src/App.tsx:22 return role === "GERANT" || role === "ADMIN";
src/App.tsx:99 const manager = session !== null && isManagerRole(session.role);
src/components/pos/PosShell.tsx:41 session.role === "GERANT" || session.role === "ADMIN" (manager)
src/components/pos/PosShell.tsx:42 session.role === "ADMIN" (admin)
src/components/pos/modals/BackupModal.tsx:399 session?.role (used to gate ADMIN / restore)
src/components/pos/modals/CheckoutModal.tsx:171 role: session?.role ?? "" (sent to backend)
src/components/pos/modals/CustomerSelectModal.tsx:27 canCreate = CAISSIER||GERANT||ADMIN
src/components/pos/modals/CustomerSelectModal.tsx:31 canManageCredit = GERANT||ADMIN
src/components/pos/modals/CustomerSelectModal.tsx:53 const role = session.role
src/components/pos/modals/RecordPaymentModal.tsx:57 isVendeur = role==="VENDEUR"
src/components/pos/modals/RecordPaymentModal.tsx:62 canUseOverpayment = GERANT||ADMIN
src/components/pos/modals/RecordPaymentModal.tsx:104 validatePaymentAmount(...,"CAISSIER")
src/components/pos/modals/RecordPaymentModal.tsx:155 const role = session.role
src/components/pos/modals/stock-gate.ts:15 ADMIN||GERANT
src/components/pos/modals/StoreIdentityModal.tsx:68 session?.role === "ADMIN"
src/components/pos/modals/WarehouseTransferModal.tsx:47 isManagerRole(role) (GERANT||ADMIN)
src/components/pos/ProductDiscovery.tsx:32 canEditCatalog = GERANT||ADMIN
src/features/catalog/CatalogModule.tsx:19-20 isManagerRole = GERANT||ADMIN
src/features/catalog/CatalogModule.tsx:38 !isManagerRole(session.role) -> deny
src/stores/backup-modal-store.ts:66 return role === "ADMIN"
src/stores/store-identity-store.ts:120 session?.role === "ADMIN"
src/stores/warehouse-transfer-store.ts:62 return role === "ADMIN" (create-warehouse tab)
src/stores/warehouse-transfer-store.ts:65-66 isManagerRole = GERANT||ADMIN (whole modal)
Count = 26 role-string comparison / isManagerRole sites in non-test source.
```
Mismatch statement (frontend grants MORE than the Rust permission allows): NONE FOUND among these 26 gates — each grants the same or a subset of the backend permission for the action it guards (canCreate == CreateCustomer holders; canUseOverpayment == ManageCustomerCredit holders; isManagerRole == ReportsView/Supplier/catalog/transfer holders; ADMIN == ManageWarehouses/SystemSettingsManage/ManagePrinterSettings holders). One interface-level exposure is noted, not a permission grant: the two ungated floating buttons in PosShell.tsx (:78-103, Convertir + Retour) render for every authenticated role and open DocumentConversionModal/ReturnModal, so a Vendeur/Caissier can reach the return/conversion screens even though returns_process/documents_process_return deny ProcessReturn (returns.rs:208). The backend still rejects the call; the frontend just does not pre-hide it. This is the same six-FAB finding as AUD-011-C1, restated here for the permission angle.

## D. Logo file inspection (read-only, Node built-ins only)
```text
File: D:/Dev/BarakaInbox/Baraka_Logo.png
exists=YES bytes=7177647
sha256 (Node)=9db26d2c61ab1c4f0baa51f5c5fa5a0a3b6228607e28f01ecf7db7a351e7b04d  sha256 (sha256sum)=9db26d2c61ab1c4f0baa51f5c5fa5a0a3b6228607e28f01ecf7db7a351e7b04d  match=YES
png-signature 89 50 4E 47 0D 0A 1A 0A = YES
chunks in order (115):
  IHDR len=13
  sRGB len=1
  gAMA len=4
  pHYs len=9
  IDAT len=65445
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=65524
  IDAT len=34195
  IEND len=0
IHDR: width=2833 height=2698 bitDepth=8 colorType=6 (6 RGBA) interlace=0
alpha channel (colorType 4/6)=YES; tRNS chunk=NO; transparency possible=YES
pHYs chunk=YES
sRGB chunk=YES
gAMA chunk=YES
iCCP chunk=NO

D3: NO pixel decoding performed. Background colour, transparency usage and visual content are UNKNOWN from this inspection; they need the owner or a later design task.
```

