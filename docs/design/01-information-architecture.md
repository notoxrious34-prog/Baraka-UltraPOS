# 01 - Information Architecture (v0.1, PROPOSED)

Status: PROPOSED. Structure follows ADR-078. The visibility matrix (2.2), the key bindings (3.3, 7) and the frequencies (6) are provisional until the static prototypes are accepted by the owner. Nothing here is a compliance or hardware-verification claim.

Scope: navigation, roles, jobs-to-be-done, module map, relocation of the six floating functions, engine dependencies. Out of scope: tokens, components, screen layouts, copy (later documents in docs/design/).

## 1. Principles

1. Every job has a path of at most 3 clicks or 4 key presses from the start screen (Vente), and the path is documented in section 6.
2. No feature is reachable only through a floating element. There are no floating action buttons. `position: fixed` is allowed only for modal overlays, toasts and the command palette (ADR-067).
3. A function lives in the screen that owns its data. Document actions live in Documents, stock actions in Stock, backups in Paramètres.
4. Every screen is reachable from the navigation rail or from a documented contextual action, and also from the command palette (Ctrl+K), which lists every screen and every action allowed to the current role.
5. Hick's law: at most 5 tabs per module; rare actions are behind a row menu or a detail panel.
6. Muscle-memory stability: a module keeps its key (Ctrl+N) forever; a hidden module leaves its key unused and the other keys do not renumber.
7. Rust is the authority on permissions. The UI hides what a role cannot use as a convenience and shows a plain "Accès réservé" state when a hidden screen is reached by palette or restored state.
8. Digits are Latin in every locale (ADR-069). Money, codes, barcodes, phone numbers are LTR-isolated inside RTL text.

## 2. Roles and visibility

### 2.1 Roles
The code has four roles: Vendeur, Caissier, Gerant, Admin (Appendix A has the code facts). Today the shop has one owner and one employee on one PC (ADR-071).

| Role | Typical person | Main jobs |
|---|---|---|
| Admin | the owner | everything, including users, backups, store identity |
| Gérant | a trusted manager | operations: catalog, stock, suppliers, reports, overrides |
| Caissier | the employee at the counter | selling, collecting payments, cash session, reprints |
| Vendeur | a seller without cash handling | selling, quotes, product and customer lookup |

### 2.2 Visibility (PROPOSED)
Legend: Y = full, R = read-only, P = partial (see note), N = hidden.

| Module | Admin | Gérant | Caissier | Vendeur |
|---|---|---|---|---|
| Vente | Y | Y | Y | Y |
| Documents | Y | Y | Y | R |
| Clients & Crédit | Y | Y | Y | R |
| Produits & Codes-barres | Y | Y | R | R |
| Stock | Y | Y | R | R |
| Fournisseurs & Achats | Y | Y | N | N |
| Caisse & Clôture | Y | Y | Y | N |
| Rapports | Y | Y | N | N |
| Utilisateurs | Y | N | N | N |
| Paramètres & Sauvegarde | Y | P | P | P |

Notes: Documents R for Vendeur = search, view, reprint only. Clients R for Vendeur = search and select a customer only (no ledger, no payments). Paramètres P for Gérant = Impression and Langue & affichage; P for Caissier and Vendeur = Langue & affichage only. Return/Avoir and document conversion for Caissier are subject to the engine permission found in Appendix A; if the engine requires a manager permission, the screen shows the action disabled with the reason, never hidden without explanation.

## 3. Navigation model

### 3.1 Shell
One application shell: a single top bar (height 48 px at all tiers) plus a navigation rail at the inline-start edge (right side in Arabic) plus the content region. The content region is the only region that scrolls; inner panels scroll, never the page.

### 3.2 Top bar (single, ADR-067)
Order from inline-start to inline-end: rail toggle, logo and name, current screen title, flexible space, cash-session chip, printer-status chip, language switch (FR | AR), theme switch, user menu. The user menu holds: user name and role, Langue, Thème, Déconnexion. Collapse by priority when the width is short: first theme and language move into the user menu, then the printer chip becomes icon-only, then the cash chip becomes icon-only. The user menu and Déconnexion are never clipped at 1024 px wide. Status chips carry icon plus text (never color alone).

### 3.3 Navigation rail
- Width 200 px expanded (icon + label), 56 px collapsed (icon + tooltip). Default: expanded at 1280 px and wider, collapsed below 1280 px. The user toggle overrides the default; persistence of the choice is an engine/store item (section 10).
- Item height 40 px. If the available height is too small the rail scrolls inside itself.
- Order, label FR | label AR | key:

| # | Module | Arabic label | Key |
|---|---|---|---|
| 1 | Vente | البيع | Ctrl+1 |
| 2 | Documents | المستندات | Ctrl+2 |
| 3 | Clients & Crédit | الزبائن والكريدي | Ctrl+3 |
| 4 | Produits & Codes-barres | المنتجات والباركود | Ctrl+4 |
| 5 | Stock | المخزون | Ctrl+5 |
| 6 | Fournisseurs & Achats | الموردون والمشتريات | Ctrl+6 |
| 7 | Caisse & Clôture | الصندوق والإغلاق | Ctrl+7 |
| 8 | Rapports | التقارير | Ctrl+8 |
| 9 | Utilisateurs | المستخدمون | Ctrl+9 |
| 10 | Paramètres & Sauvegarde | الإعدادات والنسخ الاحتياطي | Ctrl+0 |

A separator sits between item 8 and item 9. The Arabic labels are proposals; the single terminology source will be docs/design/glossary-ar-fr.md.
- The start screen after login is Vente for every role.

## 4. Module map
Engine status: EXISTS = engine command exists; UI-ONLY = engine exists and only the screen is missing; NEEDS-BRK = engine capability missing (section 10).

### 4.1 Vente
One cockpit, no tabs: search/scan field, results, cart, TOTAL, customer chip with credit HUD, parked-sales list (F6), quick products (Alt+1..8), last-sale reprint, key-hint footer. The keyboard contract is the one in the charter section 7.3. Screen spec comes in a later document. Engine: EXISTS for sales; NEEDS-BRK for draft list/resume and FTS search.

### 4.2 Documents
| Tab or panel | Content | Existing UI to reuse | Engine |
|---|---|---|---|
| List (single screen) | search and filters (type, date, customer, status), table of documents | none | UI-ONLY (documents_search) |
| Detail panel | items, payments, print status, actions: Réimprimer / Duplicata, Convertir, Retour / Avoir | DocumentConversionModal, ReturnModal logic | EXISTS (convert_*, documents_*, returns_*) |

### 4.3 Clients & Crédit
| Tab or panel | Content | Existing UI to reuse | Engine |
|---|---|---|---|
| Clients | list, search, create, edit | CustomerSelectModal logic | customers_list/search/create EXISTS; customers_update UI-ONLY; fiscal identifiers NEEDS-BRK |
| À relancer | customers with overdue balance, days since last payment | CreditOutstandingReport | NEEDS-BRK |
| Detail panel | identity, balance, statement, button Encaisser | RecordPaymentModal | credit_get_statement UI-ONLY; credit_record_payment EXISTS |

### 4.4 Produits & Codes-barres
| Tab | Content | Existing UI to reuse | Engine |
|---|---|---|---|
| Liste | browse, search, filter, open editor | CatalogModule | browse/pagination NEEDS-BRK |
| Catégories | create, edit, reorder, deactivate | none | NEEDS-BRK |
| Import / Export | file import with validation report, export (ADR-077) | none | NEEDS-BRK |
| Étiquettes | barcode label printing | none | NEEDS-BRK |
Product editor with the fast keyboard-only entry flow (ADR-077 D1) is a full-height form reached from Liste ("Nouveau produit", Ctrl+N inside the module). Kit definition appears inside the editor (catalog_define_kit exists, no UI today).

### 4.5 Stock
| Tab | Content | Existing UI to reuse | Engine |
|---|---|---|---|
| Vue d'ensemble | quantities, valuation, low-stock filter, row action Régulariser | LowStockReport | listing NEEDS-BRK; stock_adjust EXISTS |
| Mouvements | movement history per product | StockMovementReport | EXISTS |
| Dépôts & transferts | warehouses, transfers | WarehouseTransferModal | EXISTS (warehouses_*, stock_transfer) |
| Inventaire physique | counting session and reconciliation | none | engine exists (ADR-060 reconcile), command surface to be verified |

### 4.6 Fournisseurs & Achats
Tabs: Fournisseurs, Commandes, Réceptions, Factures fournisseur. Reuse SupplierListScreen, SupplierFormModal, SupplierStatementView, PurchaseOrderFormScreen, GoodsReceiptFormScreen, SupplierInvoiceFormScreen, RecordSupplierPaymentModal. Engine: EXISTS.

### 4.7 Caisse & Clôture
Tabs: Session en cours (state, movements, expense, closing flow, Z report), Historique des sessions. Reuse OpenCashSessionModal, CloseCashSessionModal, RecordExpenseModal, CashSessionIndicator, CashSessionsHistoryReport. Engine: EXISTS. F11 in Vente opens the same closing flow.

### 4.8 Rapports
A home grid of the nine reports (DailySales, SalesByProduct, SalesByCategory, GrossMargin, LowStock, CreditOutstanding, StockMovement, CashSessionsHistory, UserActivity). Rows link to the owning screen (section 8). Engine: EXISTS.

### 4.9 Utilisateurs
List, create, edit, deactivate, reset password, role; protection against deactivating the last active Admin. Engine: NEEDS-BRK.

### 4.10 Paramètres & Sauvegarde
Sections: Identité du magasin (StoreIdentityModal content), Impression (58 mm, 80 mm, A4, test print), Fiscalité (read-only statement of the IFU regime per ADR-068; no compliance claim), Sauvegardes (BackupModal content, typed RESTORE confirmation kept per ADR-074), Langue & affichage. Engine: EXISTS for identity, printer and backups; the tax regime key and language/theme persistence are NEEDS-BRK.

## 5. Relocation of the six floating functions

| Today (fixed button, PosShell.tsx) | Opens | Gate today | New home |
|---|---|---|---|
| Convertir | DocumentConversionModal | all roles | Documents > detail panel > Convertir |
| Retour & Facture d'Avoir | ReturnModal | all roles | Documents > detail panel > Retour / Avoir |
| Dépôts & Transferts | WarehouseTransferModal | manager | Stock > Dépôts & transferts |
| Régularisation de Stock | StockAdjustModal | manager | Stock > Vue d'ensemble > row action Régulariser |
| Sauvegardes & Sécurité | BackupModal | ADMIN | Paramètres & Sauvegarde > Sauvegardes |
| Identité & Impression | StoreIdentityModal | ADMIN | Paramètres & Sauvegarde > Identité du magasin and Impression |

The six buttons are deleted when the new shell lands. Modals that remain modals (checkout, quantity, discount, customer select, payment, cash open/close/expense) are opened from their screens, never from fixed elements.

## 6. Jobs and paths (frequencies are estimates to validate with the owner)

| ID | Job | Role | Frequency | Criticality | Path | Engine |
|---|---|---|---|---|---|---|
| J01 | Scan or type, add to cart, pay | Vendeur, Caissier, Gérant | 200 per day | critical | start on Vente; scan or F1 + type; Enter; F12 | EXISTS, search NEEDS-BRK |
| J02 | Sell on credit | Caissier, Gérant | 30 per day | critical | F2 choose customer; F12; method Crédit | EXISTS |
| J03 | Collect a debt payment | Caissier, Gérant | 20 per day | critical | Ctrl+3; select customer; Encaisser; or customer chip in Vente > Encaisser | EXISTS |
| J04 | Park and resume a sale | all selling roles | 10 per day | high | F8 park; F6 list; Enter resume | NEEDS-BRK |
| J05 | Reprint the last receipt | Caissier, Gérant | 5 per day | medium | Vente > Dernier ticket; or palette Réimprimer | NEEDS-BRK |
| J06 | Return goods / Avoir | Caissier (override), Gérant | 3 per week | high | Ctrl+2; F1 find document; open; Retour / Avoir | EXISTS, list UI-ONLY |
| J07 | Quote to BL to Facture | Gérant, Caissier | 10 per week | medium | Ctrl+2; filter Devis; open; Convertir | EXISTS, list UI-ONLY |
| J08 | Open and close the cash session | Caissier, Gérant | 2 per day | critical | F11 in Vente; or Ctrl+7 | EXISTS |
| J09 | Enter a new product (fast loop) | Gérant, Admin | hundreds at start | high | Ctrl+4; Ctrl+N; fill; save-and-new key decided in prototype | NEEDS-BRK |
| J10 | Import products from a file | Gérant, Admin | rare | medium | Ctrl+4; tab Import / Export | NEEDS-BRK |
| J11 | Receive supplier goods | Gérant | 3 per week | high | Ctrl+6; tab Réceptions; Nouvelle | EXISTS |
| J12 | Adjust stock or count inventory | Gérant, Admin | weekly | high | Ctrl+5; Vue d'ensemble; Régulariser; or Inventaire physique | listing NEEDS-BRK |
| J13 | See who owes and for how long | Gérant, Admin | daily | critical | Ctrl+3; tab À relancer | NEEDS-BRK |
| J14 | Make a backup | Admin | daily | critical | Ctrl+0; Sauvegardes | EXISTS |
| J15 | Manage users | Admin | rare | high | Ctrl+9 | NEEDS-BRK |
| J16 | Daily sales report | Gérant, Admin | daily | medium | Ctrl+8; Ventes du jour | EXISTS |
| J17 | Edit store identity and printer | Admin | rare | medium | Ctrl+0; Identité or Impression | EXISTS |
| J18 | Transfer stock between warehouses | Gérant | rare | low | Ctrl+5; Dépôts & transferts | EXISTS |

## 7. Global keyboard (PROPOSED)
- Ctrl+K: command palette (screens and actions allowed to the role).
- Ctrl+1..Ctrl+9, Ctrl+0: modules per 3.3 (do not type characters, so they never hijack text inputs).
- F1: focus the primary search field of the current screen (in Vente it is the product search).
- Esc: close the innermost modal, panel or palette; clear the focused search; never leaves the screen.
- Inside Vente the F-key contract of the charter section 7.3 applies unchanged; outside Vente F2..F12 are inactive unless a screen specification defines them.
- Ctrl+Digit conflicts with WebView2 or Windows are verified in the prototype before the keys are frozen.

## 8. Contextual links between modules
- Vente customer chip > Clients & Crédit detail of that customer.
- Product row in Vente or in a report > Produits editor of that product (role permitting).
- Document number anywhere > Documents detail panel.
- LowStock report row > Stock Vue d'ensemble filtered on that product.
- CreditOutstanding report row > Clients & Crédit detail with the Encaisser button.
- Every link that the role cannot follow is shown disabled with the reason.

## 9. Required states for navigation
- Forbidden: "Accès réservé" with the roles that can open it and a back action.
- Empty module: a teaching empty state with the next action (for example Produits with no product: "Ajouter un produit" and "Importer un fichier").
- Loading: skeleton under 400 ms perception budget; no spinner-only screens.
- Error: plain-language message in the active language with a retry action.
- Offline is normal: no screen shows a connectivity error because the product never needs a network.
- Unsaved edits: leaving a form with edits asks to save or discard (undo toast for reversible actions, typed or long-press confirmation for irreversible ones per ADR-074).

## 10. Engine dependency register (priority is decided in the roadmap, not here)
| Capability | Needed by | Status |
|---|---|---|
| SessionInfo carries the effective permission set | rail visibility, disabled-with-reason | NEEDS-BRK |
| Indexed FTS search with normalization (ADR-077 D3) and product browse with pagination | Vente, Produits | NEEDS-BRK |
| Categories management | Produits > Catégories | NEEDS-BRK |
| Users management | Utilisateurs | NEEDS-BRK |
| Customer fiscal identifiers; overdue-credit list | Clients & Crédit | NEEDS-BRK |
| Parked drafts: list, resume, discard; last-sale lookup | Vente | NEEDS-BRK |
| Stock overview listing with valuation | Stock | NEEDS-BRK |
| Product import/export (ADR-077) and barcode labels | Produits | NEEDS-BRK |
| Language, theme and rail-state persistence; tax.regime key | shell, Paramètres | NEEDS-BRK |
| Document list screen, customer edit screen, kit UI, product detail | Documents, Clients, Produits | UI-ONLY |

## 11. Open points (to settle through the prototypes and the owner)
1. Rail default (collapsed vs expanded) at 1024 px wide for a non-technical user.
2. Exact Caissier vs Vendeur permissions (compare with Appendix A before freezing 2.2).
3. Whether Ctrl+Digit keys are free of conflicts in the packaged app.
4. Which of the 19 named modals in AUD-011 is the nineteenth (UNKNOWN: AUD-011 enumerates 18).
5. Dialogs today have no focus trap and no focus restoration (AUD-011-C1 section G); the modal component must add both.

## Appendix A - Role and permission facts (generated from code, verbatim)
Single mapping: exactly one Role-to-permission table exists, `Role::permissions()` in crates/domain/src/role.rs (below); the frontend holds no permission mapping and compares role-name strings only.

Source: crates/domain/src/role.rs:13-22

    pub enum Role {
        /// Counter sales only.
        Vendeur,
        /// Vendeur + cash collection and cash sessions.
        Caissier,
        /// Caissier + overrides, returns, stock, purchases, reports.
        Gerant,
        /// Gerant + users, settings, backup/restore.
        Admin,
    }

Source: crates/domain/src/role.rs:27-80

    pub enum Permission {
        PosSell,
        PosParkResume,
        CashierCollectPayment,
        CashSessionOpen,
        CashSessionClose,
        PriceOverride,
        DiscountOverride,
        NegativeStockOverride,
        CreditLimitOverride,
        ProcessReturn,
        StockAdjustment,
        PurchaseManage,
        ReportsView,
        UserManage,
        SystemSettingsManage,
        BackupRestoreManage,
        /// Read warehouses and stock levels (all roles).
        ViewStock,
        /// Manual stock corrections (Gerant, Admin).
        AdjustStock,
        /// Warehouse-to-warehouse transfers (Gerant, Admin).
        TransferStock,
        /// Warehouse master-data management (Admin only).
        ManageWarehouses,
        /// Checkout and draft sales (all roles).
        CreateSale,
        /// Read customer directory (all roles).
        ViewCustomers,
        /// Create customers (Caissier, Gerant, Admin).
        CreateCustomer,
        /// Credit policy/limit management + overpayment approval (Gerant, Admin).
        ManageCustomerCredit,
        /// Record customer credit payments (Caissier, Gerant, Admin).
        RecordCreditPayment,
        /// Enqueue reprint jobs (Caissier, Gerant, Admin; NOT Vendeur).
        ReprintDocument,
        /// Printer transport configuration (Admin only).
        ManagePrinterSettings,
        /// Record drawer expenses (Caissier, Gerant, Admin).
        CashExpenseRecord,
        /// Read cash session reports / Z-reports (Caissier, Gerant, Admin).
        CashReportView,
        /// Supplier master-data management (Gerant, Admin only).
        SupplierManage,
        /// Create/edit purchase orders BC (Gerant, Admin only).
        PurchaseOrderCreate,
        /// Receive goods BR (Gerant, Admin only).
        PurchaseReceive,
        /// Record supplier invoices FF (Gerant, Admin only).
        PurchaseInvoiceRecord,
        /// Record supplier payments CR (Gerant, Admin only).
        SupplierPaymentRecord,
    }

Source: crates/domain/src/role.rs:107-211

        /// Exact permission set for this role (additive hierarchy).
        #[must_use]
        pub const fn permissions(self) -> &'static [Permission] {
            use Permission::{
                AdjustStock, BackupRestoreManage, CashExpenseRecord, CashReportView, CashSessionClose,
                CashSessionOpen, CashierCollectPayment, CreateCustomer, CreateSale,
                CreditLimitOverride, DiscountOverride, ManageCustomerCredit, ManagePrinterSettings,
                ManageWarehouses, NegativeStockOverride, PosParkResume, PosSell, PriceOverride,
                ProcessReturn, PurchaseInvoiceRecord, PurchaseManage, PurchaseOrderCreate,
                PurchaseReceive, RecordCreditPayment, ReportsView, ReprintDocument, StockAdjustment,
                SupplierManage, SupplierPaymentRecord, SystemSettingsManage, TransferStock, UserManage,
                ViewCustomers, ViewStock,
            };
            match self {
                Self::Vendeur => &[PosSell, PosParkResume, ViewStock, CreateSale, ViewCustomers],
                Self::Caissier => &[
                    PosSell,
                    PosParkResume,
                    CashierCollectPayment,
                    CashSessionOpen,
                    CashSessionClose,
                    CashExpenseRecord,
                    CashReportView,
                    ViewStock,
                    CreateSale,
                    ViewCustomers,
                    CreateCustomer,
                    RecordCreditPayment,
                    ReprintDocument,
                ],
                Self::Gerant => &[
                    PosSell,
                    PosParkResume,
                    CashierCollectPayment,
                    CashSessionOpen,
                    CashSessionClose,
                    CashExpenseRecord,
                    CashReportView,
                    PriceOverride,
                    DiscountOverride,
                    NegativeStockOverride,
                    CreditLimitOverride,
                    ProcessReturn,
                    StockAdjustment,
                    PurchaseManage,
                    ReportsView,
                    ViewStock,
                    AdjustStock,
                    TransferStock,
                    CreateSale,
                    ViewCustomers,
                    CreateCustomer,
                    ManageCustomerCredit,
                    RecordCreditPayment,
                    ReprintDocument,
                    SupplierManage,
                    PurchaseOrderCreate,
                    PurchaseReceive,
                    PurchaseInvoiceRecord,
                    SupplierPaymentRecord,
                ],
                Self::Admin => &[
                    PosSell,
                    PosParkResume,
                    CashierCollectPayment,
                    CashSessionOpen,
                    CashSessionClose,
                    CashExpenseRecord,
                    CashReportView,
                    PriceOverride,
                    DiscountOverride,
                    NegativeStockOverride,
                    CreditLimitOverride,
                    ProcessReturn,
                    StockAdjustment,
                    PurchaseManage,
                    ReportsView,
                    UserManage,
                    SystemSettingsManage,
                    BackupRestoreManage,
                    ViewStock,
                    AdjustStock,
                    TransferStock,
                    ManageWarehouses,
                    CreateSale,
                    ViewCustomers,
                    CreateCustomer,
                    ManageCustomerCredit,
                    RecordCreditPayment,
                    ReprintDocument,
                    ManagePrinterSettings,
                    SupplierManage,
                    PurchaseOrderCreate,
                    PurchaseReceive,
                    PurchaseInvoiceRecord,
                    SupplierPaymentRecord,
                ],
            }
        }
    
        /// Whether this role grants `permission`.
        #[must_use]
        pub fn has_permission(self, permission: Permission) -> bool {
            self.permissions().contains(&permission)
        }

Source: src/App.tsx:21-23

    function isManagerRole(role: string): boolean {
      return role === "GERANT" || role === "ADMIN";
    }

Source: src/App.tsx:99-121

      const manager = session !== null && isManagerRole(session.role);
    
      return (
        <div
          dir={locale === "ar-DZ" ? "rtl" : "ltr"}
          lang={locale === "ar-DZ" ? "ar" : "fr"}
          className={`h-full ${locale === "ar-DZ" ? "font-arabic" : "font-sans"}`}
        >
          {session === null ? (
            <UnauthenticatedGate />
          ) : (
            <div className="flex h-full flex-col">
              {manager && <NavRail view={view} onSelect={setView} />}
              <div className="min-h-0 flex-1">
                {view === "suppliers" && manager ? (
                  <SuppliersModule />
                ) : view === "reports" && manager ? (
                  <ReportsHomeScreen />
                ) : view === "catalog" && manager ? (
                  <CatalogModule />
                ) : (
                  <PosShell />
                )}

Observations (each one sentence, tied to a row of table 2.2):

1. Vente: today App.tsx:119-121 falls every non-manager session through to PosShell with no Vendeur/Caissier distinction, while the engine gives Caissier 13 permissions and Vendeur only 5.
2. Documents: no list screen exists although documents_search and the convert/return commands exist, and conversion plus return open only from the two ungated FABs.
3. Clients & Crédit: no customer list or edit screen exists and customers_update has no UI caller, against the PROPOSED Clients tab with create/edit.
4. Produits & Codes-barres: the catalog module covers the creation flow only with manager-only entry (CatalogModule.tsx:38), while the PROPOSED Liste/Catégories/Import-Export/Étiquettes tabs do not exist.
5. Stock: no overview screen exists and Caissier/Vendeur hold only ViewStock with no read-only UI surface, against the PROPOSED R cells.
6. Fournisseurs & Achats: the existing screens are manager-gated by the App view switch, matching the PROPOSED N cells for Caissier and Vendeur.
7. Caisse & Clôture: the cash modals open from CashSessionIndicator for any authenticated role although Vendeur holds no Cash* permission, against the PROPOSED N cell for Vendeur.
8. Rapports: the reports view is manager-gated, matching the PROPOSED N cells, but Caissier holds CashReportView with no reachable report surface.
9. Utilisateurs: no screen of any kind exists although UserManage is Admin-only in the engine, against the PROPOSED Admin-only row.
10. Paramètres & Sauvegarde: backup and identity open only from ADMIN-gated FABs with no Impression or Langue & affichage sections, against the PROPOSED P cells for Gérant, Caissier and Vendeur.
11. The frontend authorizes by role-name string compares (isManagerRole at App.tsx:21-23 with copies at CatalogModule.tsx:19 and warehouse-transfer-store.ts:65, plus inline ADMIN checks) and SessionInfo (src/types/pos.ts:111-117) carries no permission set, so the PROPOSED permission-derived visibility needs the section-10 engine item.
12. Caissier lacks the engine ProcessReturn permission yet the return FAB is ungated for every role, so the PROPOSED disabled-with-reason treatment is future work with the backend as decider.

## Appendix B - Floating functions evidence (from audit/AUD-011/INVENTORY-C1.md section C2)
Six `<button type="button">` elements with `fixed ... end-4 z-40 ... rounded-full` classes in src/components/pos/PosShell.tsx (fixed-class lines 86, 99, 113, 128, 149, 164 per AUD-011). Openers verified exhaustively with `grep -rn 'openModal("'`: conversion, return, warehouse, stock-adjust, backup and identity modals are opened ONLY from these six elements. Cashier (non-manager) does not render the warehouse, stock-adjust, backup and identity buttons (PosShell.tsx role checks at lines 104, 119, 140, 156 per AUD-011-C1).
