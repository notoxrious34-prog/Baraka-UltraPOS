# TASK-BRK-050 — Suppliers, Purchases, Reporting & Backup Audit

**Status:** read-only investigation — zero production/test files modified.
**Method:** targeted reads across `crates/domain`, `crates/application`,
`crates/infrastructure` (adapters + migrations 0001/0009), `crates/commands`,
`src-tauri`, and `src/`.

---

## 1. Supplier master data & ledger — DONE end to end

### Schema
- `suppliers(id, code UNIQUE, name, phone?, address?, current_balance_centimes DEFAULT 0, is_active DEFAULT 1, created_at, updated_at)` (`0001:144-154`); migration 0009 rebuilds it authoritatively adding `nif?, rc?, notes?, origin_device_id` (`0009:29-42`).
- `supplier_ledger(id, sequence_no, supplier_id→suppliers, entry_type, amount_centimes, running_balance_centimes, document_type, document_id, created_by, created_at)` (`0001:157-168`; 0009 shape adds `reference_type CHECK(PURCHASE_INVOICE|PAYMENT)`, `reference_id`, `description`, `amount > 0` CHECK at `0009:45-55`).
- **Append-only triggers mirror customer_ledger exactly:**
  `prevent_supplier_ledger_update/delete` (`0001:357-367`, re-created
  identically in `0009:133-143`). Index
  `idx_supplier_ledger_supplier(supplier_id, sequence_no)` (`0001:335-336`).

### Domain
- `Supplier{id, name(2..=100), phone?, address?, nif?, rc?, notes?, is_active, current_balance: Money}` (`suppliers/supplier.rs:86-107`); `PurchaseOrderStatus` (`:43+`, Draft/PartiallyReceived/FullyReceived rollup).
- `LedgerEntryType::{Debit,Credit}` with `DEBIT increases what we owe; CREDIT reduces it` (`supplier_ledger.rs:9-32`); `LedgerReferenceType::{PurchaseInvoice→PURCHASE_INVOICE, Payment→PAYMENT}` (`:51-77`); `SupplierLedgerEntry` with positive-only `amount: Money` (`:82-101`); module invariant `SUM(DEBIT)−SUM(CREDIT)` (`suppliers/mod.rs:6`).

### Application
- `suppliers.rs`: `CreateSupplierUseCase` (`:47-98`), `UpdateSupplierUseCase` sparse (`:101-142`), `SearchSuppliersUseCase` (`:145-176`), `GetSupplierUseCase` (`:179-206`), `GetSupplierStatementUseCase` (`:209-236`) — all gated `SupplierManage` (Gerant/Admin; `role.rs:162,198`; denied-matrix asserted `:348`).
- Ports `ports.rs:1376+`: `SupplierView, NewSupplier, SupplierUpdate, SupplierStatement`; `SupplierRepository` port consumed by all five use cases.

### Commands (all registered `lib.rs:175-179`)
- `suppliers_create/update/get/search/get_statement` (`ipc.rs:2390-2470+`).
- TS wrappers `ipcCreateSupplier … ipcGetSupplierStatement` (`ipc.ts:1365-1432+`).
- UI: `SupplierListScreen.tsx` (directory + statement) with embedded
  `SupplierFormModal.tsx`; mounted as manager-gated `SuppliersModule`
  (`App.tsx:15,113`).

## 2. Purchases lifecycle (BC → BR → FF) — DONE end to end

- Use cases (`purchases.rs`, all Gerant/Admin per header `:1-8`):
  `CreatePurchaseOrderUseCase` BC, pure intent, zero stock/ledger effect
  (`:98-148`); `UpdatePurchaseOrderUseCase` DRAFT-only line replace
  (`:151-207`); `ReceiveGoodsUseCase` BR + `PURCHASE_IN` effect, Gerant
  override for over-receipt (`:210-272`); `RecordSupplierInvoiceUseCase`
  FF = single ledger DEBIT, structurally stock-free per ADR-029
  (`:275-339`); `RecordSupplierPaymentUseCase` CREDIT + optional drawer
  outflow per ADR-030, methods CASH|OTHER only (`:342-422`);
  `GetPurchaseDocumentUseCase` (`:425-460`).
- Infra `sqlite_purchase_repository.rs`: `PURCHASE_IN` legs at `:713,907`;
  FF ledger `DEBIT/'PURCHASE_INVOICE'` at `:918-924`; payment
  `CREDIT/'PAYMENT'` at `:1021-1023`. **Cost price is stored per purchase
  line (`unit_cost_centimes`) only — no `UPDATE products/product_units`
  exists in the purchase adapter (grep: zero matches), so receiving goods
  does NOT refresh the product reference cost.** Record as design gap
  (margin reports keep using the catalog snapshot).
- Commands `purchases_create_order/update_order/receive_goods/
  record_invoice/record_payment/get_document` (`ipc.rs:2501-2667`),
  registered `lib.rs:180-185`; TS wrappers `ipcCreatePurchaseOrder …`
  (`ipc.ts:1446-1560+`); UI screens `PurchaseOrderFormScreen`,
  `GoodsReceiptFormScreen`, `SupplierInvoiceFormScreen`,
  `RecordSupplierPaymentModal` embedded in the suppliers module
  (`SupplierListScreen.tsx:16-18,204-206`).

## 3. Reporting engine — DONE end to end

- 9 use cases (`reports.rs:81-338+`): daily sales, sales-by-product,
  sales-by-category, gross margin, credit outstanding, stock movement
  history, low stock (depleted `quantity_base_micro <= 0`, `:290-298`),
  cash sessions history, user activity. All require `ReportsView`
  (Gerant/Admin, `:58-64`); ISO day validation + `InvalidDateRange`
  (`:27-56`); pagination default 50, hard reject > 200 (`:17-23,66-78`).
- No dedicated reporting tables — pure SELECT views over transactional
  tables via `ReportingRepository` port.
- Commands `reports_daily_sales … reports_user_activity` (9,
  `ipc.rs:2694-2917+`), registered `lib.rs:186-194`; TS wrappers (9,
  `ipc.ts:1694-1854+`).
- UI: `ReportsHomeScreen.tsx` nine-tile dashboard (`:2,10-18,59-67`)
  mounted manager-gated (`App.tsx:116`); all nine report components exist
  under `src/features/reports/`.

## 4. Backup, encryption & maintenance — GAP (largest in this audit)

- `backups_log(id, backup_id UNIQUE, app_version, schema_version,
  created_at, source_device_id, database_hash, cipher_version, status)`
  (`0001:314-324`) is **write-only schema**: grep over all of
  `infrastructure/src` for `backups_log|INSERT INTO backups|backup_id`
  returns **zero matches** — nothing ever writes it.
- `rusqlite 0.32` enables `backup` cargo feature (`Cargo.toml:24`), but
  **no backup code exists** (no `sqlite3_backup`, no `VACUUM INTO`,
  no snapshot routine anywhere in `crates/`).
- **No SQLCipher**: zero matches for sqlcipher/cipher/pragma-key across
  manifests and sources; `cipher_version` is a dead schema column. The
  database file is unencrypted SQLite.
- **No backup/restore IPC commands** (grep `backup|restore` in
  `commands/src/ipc.rs`: zero matches) and **no Settings/Backup UI**
  (no settings screen/modal anywhere in `src/`; only printer-settings
  IPC exists).

## Gap table

| Module | Backend Domain | Commands/IPC | Frontend Store | Frontend UI | Status |
|---|---|---|---|---|---|
| Suppliers master + statement | ✅ tested | ✅ 5 cmds | ✅ via wrappers | ✅ list+form+statement | DONE |
| Purchases BC/BR/FF + payments | ✅ tested (ADR-029/030) | ✅ 6 cmds | ✅ via wrappers | ✅ 4 screens | DONE |
| Cost-price refresh on receipt | ❌ lines-only storage | — | — | — | GAP (design decision needed) |
| Reporting (9 reports) | ✅ tested | ✅ 9 cmds | ✅ via wrappers | ✅ 9-tile dashboard | DONE |
| Backup/restore routines | ❌ no code (feature flag only) | ❌ none | ❌ | ❌ | GAP (greenfield) |
| SQLCipher encryption | ❌ absent | — | — | — | GAP (greenfield, migration-sensitive) |
| Settings/Backup UI | — | ❌ | ❌ | ❌ printer-settings only | GAP |

## Recommended roadmap

1. **Backup hardening (greenfield, no migration):** backup service
   (`rusqlite::backup` step API already compiled in) → `backups_create/
   backups_restore` commands + `backups_log` writer (hash =
   existing `checksum_of`) → manager-only Settings/Backup screen.
   Zero schema change; ship first (data-loss risk dominates).
2. **Cost-price policy (decision + micro-task):** choose last-cost vs
   weighted-average on BR; one UPDATE path + margin-report note.
3. **SQLCipher (last):** requires rekey migration + startup unlock flow;
   sequence AFTER backup is proven (else unrecoverable-brick risk).

No production or test file was modified (only this new audit file was
created).
