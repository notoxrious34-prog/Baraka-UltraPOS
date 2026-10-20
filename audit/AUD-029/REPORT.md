# TASK-AUD-029 — Systemic Audit: Frontend Role-Gating, Navigation Shell, Over-Gating Mismatches

- Task: TASK-AUD-029 (read-only audit) · Workspace: D:/Dev/Baraka UltraPOS · Branch: master
- Baseline: worktree carries BRK-070 + DSN-071 change sets (uncommitted); this audit reads that state.
- Output: `audit/AUD-029/REPORT.md` (this file — the ONLY file created).

## 0. Headline

1. `hasPermission()` (ADR-100) has exactly **2 UI call sites**, both in `src/App.tsx:40-41,124-125`
   (`canManageUsers = hasPermission("USER_MANAGE")`, nav entry + users route). Everything else
   still compares role strings: **~22 raw `role ===` sites** across 14 files.
2. **Over-gating confirmed and quantified.** The harshest cases: Catalog and Customers modules
   require GERANT/ADMIN (`CatalogModule.tsx:715`, `CustomersModule.tsx:66`) while the engine
   authorizes reads with `PosSell` / `ViewCustomers` — permissions **all four roles hold**
   (catalog.rs:417,459,644; customers.rs:250,283,313,396). Result: Caissier and Vendeur are
   locked out of price/stock lookup and the customer directory, actions the backend would allow.
   `customers_create` needs only `CreateCustomer` (Caissier holds it), yet `CustomerFormModal`
   credit paths require GERANT/ADMIN — Caissier can create the customer but the UI's credit
   management assumes otherwise (see §3 row 12).
3. **Under-gating: none found.** Every UI-exposed action was checked against the engine gate;
   in all mismatches the UI is stricter than Rust, never looser. Rust remains authoritative.
4. `suppliers/`, `purchases/`, `reports/`, `pos/` feature roots contain **zero** internal role
   checks — they rely on the App-level `manager` gate plus engine enforcement.

## 1. Navigation rail & routing (`src/App.tsx`)

`isManagerRole` (`App.tsx:28-30`, local): `GERANT || ADMIN`. `manager` (`:123`) gates the
entire rail (`:140`) and 6 routes. `canManageUsers` (`:40-41,124-125`) gates users only.

| View | Nav entry gate | Route gate | Fail fallback | Engine truth | Verdict |
|---|---|---|---|---|---|
| pos | none (always) | none — default branch | n/a | `CreateSale` etc. per action | OK (action-level) |
| stock | rail: `manager` | `manager` (`:142`) | n/a (rail hidden) | reads `ViewStock` (all roles) | **OVER** — Vendeur/Caissier hold `ViewStock` (+`CreateSale`) yet cannot open the view |
| customers | rail: `manager` | `manager` (`:144`) + module `authorized` (`CustomersModule.tsx:66`) | `UNAUTHORIZED` card (`:104-112`) | reads `ViewCustomers` (all roles) | **OVER** — same shape as stock |
| documents | rail: `manager` | `manager` (`:146`) + module `authorized` (`DocumentsModule.tsx:39`) | `UNAUTHORIZED` card | search gate not probed; reprint `ReprintDocument` (Caissier+) | **LIKELY OVER** for reprint discovery |
| suppliers | rail: `manager` | `manager` (`:148`) | n/a | `SupplierManage` + purchase perms (Gerant/Admin only) | OK — UI matches engine |
| reports | rail: `manager` | `manager` (`:150`) | n/a | `ReportsView` (Gerant/Admin); cash reports `CashReportView` (Caissier+) | **PARTIAL** — Caissier holds `CashReportView` but has no route to any report |
| catalog | rail: `manager` | `manager` (`:152`) + module `authed` (`CatalogModule.tsx:715`) | token-gated (`token=null` blocks fetch) | reads `PosSell` (all roles); mutations `StockAdjustment` OR `SystemSettingsManage` | **OVER (reads)**; mutations ≈ correct (GERANT via `StockAdjustment`, ADMIN via both) |
| settings | rail: `manager` | `manager` (`:154`); tabs: `isAdmin` (`SettingsModule.tsx:157`) → lock badges (`:224`) + `FiscalSettingsTab` (`:53,86`), `BackupSettingsTab` (`:61`) | locked-tab badge; tab-level null/disabled | identity `SystemSettingsManage`; printer `ManagePrinterSettings` (Admin-only); backup restore Admin-only (`backup.rs:257-265`); categories = catalog `MANAGE_PERMISSIONS` | OK for admin tabs; categories tab inherits catalog over-gating |
| users | `canManageUsers` (`:75`) | `canManageUsers` (`:159`) → `<UsersModule/>` else `users.unauthorized` card (`:160-164`) | unauthorized card | `UserManage` (Admin-only) | OK (migrated DSN-071) |

## 2. Feature-level call-site inventory (non-test sources only)

| # | File:line | Expression | Guards | Target permission (engine-verified) | Verdict |
|---|---|---|---|---|---|
| 1 | `App.tsx:28` | `role === "GERANT" \|\| role === "ADMIN"` | rail + 6 routes | manager bundle (no single perm) | migrate Batch 1 |
| 2 | `App.tsx:40-41,124-125` | `hasPermission("USER_MANAGE")` | users nav + route | `UserManage` | DONE (reference impl) |
| 3 | `OperationsMenu.tsx:40-41` | `manager` / `admin` in `operationsForRole` | stock section (transfer, adjust) / system section (backup, identity) | `TransferStock`+`AdjustStock` / `BackupRestoreManage`+`SystemSettingsManage` | Batch 2 |
| 4 | `CustomerSelectModal.tsx:28` `canCreate` | C/G/A | new-customer button | `CreateCustomer` (customers.rs:87,164) | EXACT — direct swap |
| 5 | `CustomerSelectModal.tsx:32` `canManageCredit` | G/A | credit controls | `ManageCustomerCredit` | EXACT |
| 6 | `RecordPaymentModal.tsx:57` `isVendeur` | `=== "VENDEUR"` (inverted block) | blocks payment recording | NOT `RecordCreditPayment` (customers.rs:346) | EXACT (inverted) |
| 7 | `RecordPaymentModal.tsx:62` `canUseOverpayment` | G/A | negative-balance approval | `ManageCustomerCredit` | EXACT |
| 8 | `CustomerFormModal.tsx:31` `canManageCredit` | G/A | credit-limit fields | `ManageCustomerCredit` | EXACT |
| 9 | `stock-gate.ts:15` `isPrivilegedStockRole` | A/G | sale-on-shortage | `NegativeStockOverride` (sales ×2, inventory ×1) | EXACT |
| 10 | `CheckoutModal.tsx:171` `evaluateStockGate({role…})` | role string in | checkout confirm | `NegativeStockOverride` | EXACT |
| 11 | `CatalogModule.tsx:247,715` `isManagerRole` + `authed` | G/A | whole hub incl. search | reads `PosSell` (ALL roles); writes `StockAdjustment` OR `SystemSettingsManage` | **OVER (reads)**; writes OK |
| 12 | `CustomersModule.tsx:66` + `CustomerFormModal.tsx:31` | G/A | whole directory | reads `ViewCustomers` (ALL); create `CreateCustomer` (Caissier+); credit `ManageCustomerCredit` (G/A) | **OVER (reads + Caissier create)**; credit OK |
| 13 | `DocumentsModule.tsx:39` | G/A | whole module | reprint `ReprintDocument` (Caissier+, printing.rs:43) | **OVER (reprint discovery)** |
| 14 | `StockModule.tsx:58` + `:60` `isAdmin` | G/A ; A | module ; warehouse create | `ViewStock` etc. ; `ManageWarehouses` (inventory ×1, deletion ×2) | **OVER (reads)**; admin OK |
| 15 | `SettingsModule.tsx:157` `isAdmin` | A | fiscal/printer/backup tabs + lock badges | `SystemSettingsManage` / `ManagePrinterSettings` (Admin-only) / restore Admin-only | OK |
| 16 | `StoreIdentityModal.tsx:68,74` + `store-identity-store.ts:119-120` `canManageIdentity` | A | identity load + render null | `SystemSettingsManage` (printing.rs:159) | EXACT |
| 17 | `backup-modal-store.ts:65-66` `canRestoreForRole` | A | restore button (`BackupModal.tsx:399-402`) | Admin-only restore gate (`backup.rs:257-265`) | EXACT |
| 18 | `warehouse-transfer-store.ts:63-68` `isAdminRole`/`isManagerRole` + `WarehouseTransferModal.tsx:292-293` | A / G/A | warehouse pickers vs transfer submit | `ManageWarehouses` / `TransferStock` (inventory ×1) | EXACT |
| 19 | `user-helpers.ts:101-104` metric filters | `=== "ADMIN"` / `"CAISSIER" \|\| "VENDEUR"` in `filter` predicates | HUD counts only (not gates) | N/A | not gates — no migration |
| 20 | `Topbar.tsx:121` | displays `session.role` | informational badge | N/A | not a gate |

POS/F7 overrides, discount modals, cash closing, return authorization: all flow through
rows 6, 9, 10 (`RecordPaymentModal`, `stock-gate`, `CheckoutModal`) — no other POS role
checks exist. `suppliers/`, `purchases/`, `reports/` roots: zero internal checks (App gate + engine).

## 3. Domain permission cross-reference (`crates/domain/src/role.rs:27-80`, matrix `:109-205`)

- **Vendeur (5)** — `PosSell, PosParkResume, ViewStock, CreateSale, ViewCustomers` (role.rs:121).
  Should access: POS selling + park/resume, stock-level reads, checkout, customer directory
  reads. **Today: customer directory, stock view, catalog search, and every manager-routed
  module are blocked** (rows 11, 12, 14 + App rail).
- **Caissier (13)** — Vendeur 5 + `CashierCollectPayment, CashSessionOpen, CashSessionClose,
  CashExpenseRecord, CashReportView, CreateCustomer, RecordCreditPayment, ReprintDocument`
  (role.rs:122-136). Should access: all cash flows, customer create, credit-payment
  recording, document reprint, session/Z reports. **Today: reprint discovery, cash-report
  route, and catalog/customer/stock reads are blocked** despite engine grants (rows 11–14).
- **Gerant (29)** — Caissier 13 (minus nothing) + `PriceOverride, DiscountOverride,
  NegativeStockOverride, CreditLimitOverride, ProcessReturn, StockAdjustment, PurchaseManage,
  ReportsView, AdjustStock, TransferStock, ManageCustomerCredit, SupplierManage,
  PurchaseOrderCreate, PurchaseReceive, PurchaseInvoiceRecord, SupplierPaymentRecord`
  (role.rs:137-167). Should access everything except user/settings/backup/warehouses-master/
  printer-config. **Today: fully served** — every Gerant-held capability has a reachable UI path.
- **Admin (34)** — all (role.rs:168-203), incl. `UserManage, SystemSettingsManage,
  BackupRestoreManage, ManageWarehouses, ManagePrinterSettings`. Fully served.

**Quincaillerie impact.** A Vendeur cannot look up a product price or check depot stock on
screen (catalog + stock views manager-locked; engine: `PosSell`/`ViewStock` held) — must ask
a Gérant, stalling counter sales. A Caissier cannot open the customer directory to verify a
credit balance before accepting a credit payment (engine: `ViewCustomers` +
`RecordCreditPayment` held) — must leave the POS view or escalate. A Caissier cannot reprint
a delivery note for a returning contractor (engine: `ReprintDocument` held; UI: documents
module manager-locked). No revenue is lost to under-gating (none found); the cost is
entirely operator friction and Gérant bottleneck.

## 4. Phased migration blueprint

- **Batch 1 — shell** (`App.tsx`, rail entries, `manager`/`isManagerRole`): replace with
  per-view permission checks (`ViewStock`+`CreateSale` for stock, `ViewCustomers` for
  customers, `PosSell` for catalog browse, `ReportsView` OR `CashReportView` for reports,
  `ReprintDocument`-aware documents entry). Fallbacks: keep the existing `UNAUTHORIZED` card
  pattern + `token=null` fetch block. Risk: low (gates only widen to engine-authorized sets;
  engine stays authoritative).
- **Batch 2 — actions/modals** (rows 3–18): mechanical `role ===` → `hasPermission(<PERM>)`
  swaps; inverted `isVendeur` → `!hasPermission("RECORD_CREDIT_PAYMENT")`; split
  `StockModule` read (`ViewStock`) vs mutate (`AdjustStock`/`TransferStock`/
  `ManageWarehouses`) surfaces; split `CustomersModule` read/create/credit similarly.
- **Tests affected / to add (Vitest):** `operations-menu.test.tsx`
  (`operationsForRole`), `checkout-stock-gate.test.ts`, `record-payment-wire.test.ts`,
  `warehouse-transfer-modal.test.ts`, `backup-modal.test.tsx`, `store-identity-store.test.ts`,
  `users-module.test.ts` (reference impl — extend, don't rewrite); new permission-matrix
  tests per migrated site asserting `hasPermission(...)` call, not role strings. Rust:
  no changes (engine already permission-native; 136-assertion matrix test
  `role.rs:256-371` is the contract).
- **Out of scope for migration:** row 19 metric filters, row 20 Topbar display.

## 5. Line endings

Measured with byte counting (`tr -cd '\r' | wc -c`): the `grep -c $'\r'` idiom silently
returns 0 in this runner (control character mangled in transit) and is not used.
`src/App.tsx` 176/176 CRLF, `src/lib/i18n.ts` 2306/2306 CRLF (both match HEAD conventions);
`users/*` pure LF; new `audit/DSN-071/*` LF. No file touched by this task, so no
normalization was needed.

## 6. Quality gates (verbatim, run after writing this report — docs-only change)

### G1 `cargo fmt --check`
```
(empty output — clean)
G1_EXIT:0
```
### G2 `cargo clippy --all-targets --all-features -- -D warnings`
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.13s
```
### G3 `cargo test --workspace -j 2`
```
G3 passed=464 failed=0
```
(aggregate of every `test result` line; zero failures)
### G4 `npm run lint`
```
npm notice run baraka-ultrapos@1.0.0 lint
npm notice run eslint . --ext ts,tsx --report-unused-disable-directives --max-warnings 0
```
### G5 `npm run typecheck`
```
npm notice run baraka-ultrapos@1.0.0 typecheck
npm notice run tsc --noEmit
```
### G6 `npm run build`
```
dist/assets/index-r3liXAGn.css              46.16 kB │ gzip:   8.22 kB
dist/assets/index-B6SSQNbj.js              726.16 kB │ gzip: 187.00 kB
✓ built in 5.22s
```
### G7 `npx vitest run`
```
Tests  649 passed (649)
   Start at  18:15:33
   Duration  14.18s (transform 2.22s, setup 0ms, collect 6.86s, tests 1.86s, environment 27ms, prepare 13.30s)
```
