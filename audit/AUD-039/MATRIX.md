# MATRIX.md — deletion paths per area (TASK-AUD-039)

Conventions: engine = `crates/application/src/use_cases/deletion.rs` unless noted.
Dual-track = hard purge when unreferenced, deactivation otherwise (reason + audit
still open — see gap G1). `SafeDeleteModal` = `src/components/common/SafeDeleteModal.tsx`
(simple confirm, NO typed gate). Audit: **no delete use case writes an audit event**
(`deletion.rs` has zero audit references; audit port exists in `ports.rs:80` but is
unwired here).

## 1. Products (+units/kits/barcodes) — SAFE (minor gaps)
- Engine: `DeleteProductUseCase::execute` (`deletion.rs:68-93`), perm
  `StockAdjustment‖SystemSettingsManage` (`catalog.rs:18-21` MANAGE_PERMISSIONS).
  Refs>0 → `set_product_active(false)` ("deactivated"); else `hard_delete_product`
  (`sqlite_catalog_repository.rs:970-983`: tx deletes stock_levels + product_units + product).
- UI: `CatalogModule.tsx:710,1020-1036` → `SafeDeleteModal` (`del.productTitle`),
  mode-aware note `del.deleted/del.archived` (`:921-927`, `role=status`, no undo — correct,
  hard delete is irreversible).
- Gaps: (a) no typed gate for irreversible hard delete (ADR-074); (b) no audit event.

## 2. Warehouses/depots (+levels/movements) — INCONSISTENT
- Engine: `DeleteWarehouseUseCase::execute` (`deletion.rs:205-237`), perm `ManageWarehouses`.
  Guards: DEP-01 → `DEPOT_PRINCIPAL_IMMUTABLE`; nonzero levels → `WAREHOUSE_HAS_STOCK`
  (hard refuse); refs>0 → deactivate; else `hard_delete_warehouse`
  (`sqlite_warehouse_repository.rs:206-218`: tx deletes zero-levels + row; movements
  history preserved via refs-gate).
- UI: `ManageDepotsTab` (`WarehouseTransferModal.tsx:46-234`, `canManage` gate `:61`):
  DEP-01 shows lock badge (`:105`) **but the delete button stays clickable** (`:176-190`)
  and the modal is fed `isBlocked={false}` (`:213`) — engine rejects only post-confirm
  with `translateBackendError`. Stocked depots likewise offered-then-rejected.
- Gaps: UI offers what engine forbids (pre-confirm `isBlocked` unwired); no typed gate; no audit.

## 3. Customers (+ledger) — SAFE (by engine guard)
- Engine: `DeleteCustomerUseCase::execute` (`deletion.rs:112-141`), perm `ManageCustomerCredit`.
  **Balance ≠ 0 → hard refuse `CUSTOMER_BALANCE_NONZERO`** (ledger-safe); refs>0 →
  deactivate; else `hard_delete_customer` (`sqlite_customer_repository.rs:682-694`).
- UI: `CustomersModule.tsx:59-60,136,208-222` → `SafeDeleteModal`; pre-computes
  `blocked = hasOpenBalance(...)` (`:136`) with `blockedBalanceReason` guidance —
  correct disabled-with-guidance pattern.
- Gaps: no typed gate; no audit event.

## 4. Documents/invoices (+items/payments) — FORBIDDEN-BY-DESIGN
- Engine: **no delete use case, no `hard_delete_sale`, no IPC delete command**
  (ripgrep `DeleteDocument|delete_document|hard_delete_sale` → zero production hits).
  Corrections flow via AVOIR/returns (`documents_process_return`).
- UI: `DocumentsModule.tsx` has zero delete affordances.
- No fix proposed — immutability + append-only ledger (§5) rule.

## 5. Suppliers (+ledger) — SAFE (minor gaps)
- Engine: `DeleteSupplierUseCase::execute` (`deletion.rs:160-186`), perm `SupplierManage`.
  Refs>0 → deactivate; else `hard_delete_supplier`
  (`sqlite_supplier_repository.rs:350-359`). NOTE: no balance guard — verify supplier
  ledger exposure (purchase debts) before assuming SAFE (see proposal P3).
- UI: `SupplierListScreen.tsx:388-410` → `SafeDeleteModal` (`del.supplierTitle`),
  mode-aware note. Gaps: typed gate; audit.

## 6. Users — SAFE (deactivation-only + last-admin)
- Engine: **no delete use case** — only `users_update` role/active patch
  (`users.rs:158-202`, `ipc.rs:750`); demotion/deactivation of last active Admin →
  `LAST_ADMIN_PROTECTED` (`users.rs:202`).
- UI: `UsersModule.tsx:61,121-122` computes `activeAdminTotal`, passes
  `isProtected` into `UserFormModal:114-122` (DSN-071 pattern). No delete affordance.
- No hard delete exists anywhere — nothing to purge. SAFE as designed.

## Cross-cutting gaps
- G1 (engine, M): **no audit event on any delete/deactivate** — dual-track ADR-092
  demands reason + audit; reason is not even collected (no `reason` param in any execute).
- G2 (frontend, S): **no typed confirmation gate** for irreversible hard deletes
  (ADR-074) — `SafeDeleteModalProps` (`:16-27`) has no `requireText`; all 4 areas affected.
- G3 (frontend, S): depot modal offers forbidden deletes (DEP-01, stocked) — wire
  `isBlocked` pre-confirm from cheap pre-checks.
- G4 (engine, S): confirm supplier purchase-debt exposure — if suppliers carry ledger
  balances like customers, add the same nonzero-balance refuse.
