# ADR-092: Dual-Track Safe Deletion Strategy and Configurable Fiscal Stamp (Timbre)

Status: ACCEPTED
Date: 2026-10-15
Session: TASK-BRK-074 (engine dual-track safe deletion, warehouse updates, configurable timbre)
Deciders: Master Principal Architect & Design Lead, Owner

## Context

Owner Customization Specifications (Mod 2, Mod 4, Mod 5) and
CTP-SESSION-ARC-018 Section 7 require safe removal of obsolete products,
customers, suppliers, and warehouses without violating append-only ledger
invariants or foreign-key immutability, plus warehouse update support and
an operator-toggleable Algerian fiscal stamp (Droit de Timbre) on cash
sales. Hard-deleting a row referenced by `document_items`,
`stock_movements`, `product_kit_items`, `documents`, `customer_ledger`, or
`supplier_ledger` would orphan history and trip append-only triggers; never
deleting anything leaves dead catalog rows in daily operations.

## Decision

1. **Dual-track deletion.** Every deletable entity exposes one guarded
   delete use case returning `DeleteEntityOutcome { id, mode, message }`
   (`mode = "hard_deleted" | "deactivated"`, stable wire DTO
   `DeleteEntityResultDto`):
   - Zero foreign references: atomic hard purge in an immediate SQLite
     transaction (product: `stock_levels` → `product_units` → `products`;
     customer/supplier/warehouse: single-row delete after reference
     counting). Missing rows surface `NotFound`, never silent success.
   - Non-zero references: soft deactivation (`is_active = 0`), hiding the
     row from POS and active lists while preserving history.
2. **Guard invariants (checked before either track).**
   - Primary depot `DEP-01` is strictly immutable: deletion and
     deactivation both reject (`warehouses_delete`, `warehouses_update`
     with `is_active = false`).
   - Customers with `current_balance_centimes != 0` reject deletion and
     deactivation alike (debt must settle first).
   - Warehouses with any non-zero `stock_levels` row reject deletion
     (stock must transfer out first).
3. **Sparse `warehouses_update`.** Optional rename and/or `is_active`
   flip, ADMIN-gated (`ManageWarehouses`), DEP-01 deactivation rejected
   upstream of persistence.
4. **Configurable timbre.** `fiscal.timbre_enabled` lives in the existing
   `settings` table (zero schema changes; absent key reads as `true` via
   `FiscalSettingsPort`). `price_document` takes an explicit
   `timbre_enabled: bool`; `false` forces `compute_timbre` to 0 centimes
   across preview (`PreviewTotalsInput`) and checkout (`CheckoutInput`),
   both fed from `AppState.fiscal` at the IPC edge. Get/update IPC pair
   (`fiscal_settings_get` any session, `fiscal_settings_update` ADMIN via
   `SystemSettingsManage`); TypeScript bridge in `src/lib/ipc.ts`
   (`ipcGetFiscalSettings`, `ipcUpdateFiscalSettings`,
   `ipcDeleteProduct/Customer/Supplier/Warehouse`, `ipcUpdateWarehouse`).

## Consequences

Referential integrity preserved (zero orphaned documents, zero trigger
violations); clean catalog hygiene for unreferenced rows; flexible
taxation configuration for Algerian commerce. Follow-up UI surfacing
belongs to TASK-DSN-051 (engine ships the IPC contract only).
