# TASK-BRK-049 — Catalog Product Editing & Multi-Warehouse Capabilities Audit

**Status:** read-only investigation — zero production/test files modified.
**Method:** targeted reads across `crates/domain`, `crates/application`,
`crates/infrastructure`, `crates/commands`, `src-tauri`, and `src/`.

---

## 1. Catalog product update & status management

### 1.A Application layer — READY (complete, tested)

- `UpdateProductUseCase` exists at
  `crates/application/src/use_cases/catalog.rs:238-294` (input
  `UpdateProductInput` at `:92-105`).
- Modifiable fields: `name_ar` + `name_fr` (validated via
  `Product::validate_names`, `:271-272`), `category_id` (existence-checked,
  `:273-277`; `None` clears it), `is_active` (`:290`, applied verbatim),
  and the full unit set via `UnitUpsert { id: Option<UnitId>, unit }`
  (`:81-88`; known id merged, unknown id allocates `UnitId::new()`,
  `:279-285`). Per-unit mutable: unit names, `factor_micro`, barcode,
  `sale_price_centimes`, `purchase_price_centimes`, base/default flags
  (`build_unit`, `:129-162`; prices must be ≥ 0, `:139-143`).
- NOT modifiable: `sku` (struct doc `:90` "SKU stays immutable"; no code
  path writes it), product-level `barcode` (no field on the input —
  only per-unit barcodes), kit BOM (separate `DefineKitUseCase`,
  `:297+`), `is_kit` flag.
- Price revision tracking: **in-place UPDATE, no history/audit table.**
  Infra `update_product`
  (`crates/infrastructure/src/adapters/sqlite_catalog_repository.rs:280-334+`)
  bumps `products.revision + 1` (`:291`) and overwrites
  `product_units` rows in place (`:308-326`), INSERTing unknown unit ids
  (`:327-333+`). No `price_history` table exists (grep: no matches in
  `crates/`); old prices are unrecoverable after update.
- Deactivation: **no dedicated use case** — handled via the `is_active`
  boolean inside the same full-state update (`catalog.rs:290`). Note the
  payload is full-state, not sparse: callers must echo unchanged fields.
- RBAC: mutations require `StockAdjustment` **or** `SystemSettingsManage`
  (`catalog.rs:17-35`, `require_any` at `:266`). Holders: Gerant holds
  `StockAdjustment` (`role.rs:150`) but NOT `SystemSettingsManage`
  (absent from Gerant list `:137-167`); Admin holds both (`role.rs:181,185`
  region). Vendeur/Caissier hold neither → `Unauthorized`. There is NO
  `ManageCatalog`/`ManageProducts` permission (grep: no matches) —
  spec's hypothesized names do not exist.
- Unit-tested in-file (`catalog.rs:842-915`: update paths incl. category
  reassignment).

### 1.B Commands & IPC — READY

- `catalog_update_product` at `crates/commands/src/ipc.rs:653-692`
  (RBAC comment `:654-656` confirms Gerant/Admin parity with creation).
- DTO `UpdateProductDto` (`dto.rs:321-338`): `session_token, product_id,
  name_ar, name_fr, category_id: Option<String>, is_active: bool,
  units: Vec<UpdateProductUnitDto>` — full-state mirror of the use case.
  Response re-reads the fresh detail (`ProductResponseDto`, `dto.rs:242-265`
  with `is_active` at `:260`).
- Registered `src-tauri/src/lib.rs:136`.

### 1.C Frontend & TypeScript — GAP (wrapper + UI missing)

- `src/lib/ipc.ts` exposes `ipcSearchCatalog` (`:138`),
  `ipcCreateProduct` (`:170`), `ipcListWarehouses` (`:180`),
  `ipcGetStock` (`:194`) — but **zero hits** for `ipcUpdateProduct`,
  `catalog_update_product`, or `UpdateProduct` (explicit grep: no matches).
- UI: `CatalogModule.tsx` is creation-flow-only by its own header
  (`src/features/catalog/CatalogModule.tsx:1-7`: "Update/edit, categories,
  kits, and label printing are deferred to future tasks"); only
  `ProductCreateForm.tsx` exists beside it. `ProductDiscovery.tsx` is
  POS-side search display only.
- **Verdict: backend READY, needs IPC wiring (one wrapper) + edit modal.**

## 2. Multi-warehouse management & stock transfers

### 2.A Domain & schema

- `Warehouse { id: WarehouseId, code: WarehouseCode (1..=20 chars),
  name (1..=100), is_active }`
  (`crates/domain/src/inventory/warehouse.rs:63-74`; validation `:44-86`).
  **No `is_default` on the domain model.**
- SQL `warehouses` HAS `is_default INTEGER DEFAULT 0`
  (`0001_baseline_schema.sql:82-88`); migration 0012 seeds one
  `DEP-01 / Dépôt Principal / is_default=1` row. Default is therefore a
  **SQL-seed convention invisible to the domain**: `list_active()` ignores
  it, and login auto-selects the first active warehouse
  (session-store `loadWarehouses`, TASK-BRK-036 evidence). A store operates
  fine without any `is_default=1` row (selection degrades to first-active).
- **No `min_stock` / reorder threshold anywhere** in `crates/` (grep zero
  matches). `LowStockReport` means `quantity_base_micro <= 0`, most-depleted
  first (`use_cases/reports.rs:290-298`). Per-product alert thresholds would
  need a schema addition (new column + migration) — the only item in this
  audit requiring a migration.

### 2.B Application & infrastructure — READY

- `CreateWarehouseUseCase` (`use_cases/inventory.rs:78-128`): ADMIN-only
  (`ManageWarehouses`, `:103`), code/name validation, duplicate-code
  `Conflict("WAREHOUSE_CODE_EXISTS")` (`:113-117`). `ManageWarehouses` is
  Admin-only (`role.rs:342` Gerant lacks / `:344` Admin holds — asserted in
  tests).
- `ListWarehousesUseCase` (`inventory.rs:131-151`): any authenticated
  session, active ordered by code.
- `GetStockLevelUseCase` (`:154-179`), `AdjustStockUseCase` (`:182-245`,
  `AdjustStock` perm = Gerant/Admin at `role.rs:154,188`; explicit
  INCREASE/DECREASE, reason ≥ 3 chars, negative needs override).
- `TransferStockUseCase` (`inventory.rs:248-305`): `TransferStock` perm
  (Gerant/Admin; Caissier explicitly lacks it, `role.rs:340`); same-warehouse
  rejected (`TRANSFER_SAME_WAREHOUSE`, `:276-278`); both endpoints must
  exist and be active (`WAREHOUSE_INACTIVE` / `NotFound("warehouse")`,
  `:282-290`); quantity must be positive (`InvalidQuantity`, `:279-281`).
- Atomicity + negativity: infra `transfer_stock`
  (`sqlite_stock_repository.rs:456-502`) runs **one `run_transaction`**
  applying `TransferOut` (source) then `TransferIn` (destination) with a
  shared `transfer_batch_id` (`:468-492`); source leg passes
  `authorize_negative_stock: false` (`:473`), and the engine hard-blocks
  any negative outcome without authorization (`apply_one` negative branch),
  so **source underflow aborts the whole pair** — no partial transfer is
  constructible. Pre-transaction endpoint checks happen before the txn
  opens (`inventory.rs:282-290` + comment `:292`).

### 2.C Commands & IPC — READY

- `warehouses_create` (`ipc.rs:767-788`, ADMIN-only), `warehouses_list`
  (`:790-811`, bare `session_token` scalar), `stock_get` (`:813-846`),
  `stock_adjust` (`:861-894`, `adjust_error_code` mapping), `stock_transfer`
  (`:896-938`, OUT+IN ids + both new levels echoed as strings).
- DTOs: `CreateWarehouseRequestDto` / `WarehouseDto` /
  `ListWarehousesResponseDto` (`dto.rs:354-383`),
  `AdjustStockRequestDto/ResponseDto` (`:411-437`),
  `TransferStockRequestDto/ResponseDto` (`:439-470+`, batch + both
  movement ids + both levels).
- All registered `src-tauri/src/lib.rs:140-144`.

### 2.D Frontend — PARTIAL GAP

- Wrappers present: `ipcListWarehouses` (`ipc.ts:180`, consumed at login by
  `session-store.ts`), `ipcGetStock` (`ipc.ts:194`).
- Wrappers absent (explicit grep, zero hits): `warehouses_create`,
  `stock_adjust`, `stock_transfer` — no `ipcCreateWarehouse`,
  `ipcAdjustStock`, `ipcTransferStock`.
- UI absent: no warehouse CRUD screen, no transfer modal, no destination
  selector; warehouse selection exists only as the Topbar active-warehouse
  dropdown.
- **Verdict: backend READY; needs 3 thin wrappers + warehouse/transfer UI.**

## 3. Gap checklist & roadmap

| Capability | Backend | IPC | TS wrapper | UI | Verdict |
|---|---|---|---|---|---|
| Product edit (names/category/active/units+prices) | ✅ complete+tested | ✅ command+DTO+registered | ❌ missing | ❌ creation-only module | Needs IPC wiring + edit modal |
| Price history / audit trail | ❌ in-place overwrite | — | — | — | Out of scope (no table; add only if required) |
| Stock alert thresholds (min_stock) | ❌ no column anywhere | — | — | — | Needs migration + backend + UI (only migration-requiring item) |
| Product deactivation | ✅ via `is_active` in update | ✅ | ❌ (same wrapper) | ❌ | Covered by edit modal (toggle) |
| Warehouse create/list | ✅ ADMIN-gated + tested | ✅ both registered | ⚠️ list only | ❌ | Needs 1 wrapper + admin screen |
| Stock adjust (corrections) | ✅ perm-gated + tested | ✅ registered | ❌ missing | ❌ | Needs wrapper + modal |
| Inter-warehouse transfer | ✅ atomic OUT/IN + tested | ✅ registered | ❌ missing | ❌ | Needs wrapper + modal |
| Default-warehouse concept | ⚠️ SQL-seed only, domain-blind | — | — | — | No action (works as-is) |

Proposed zero-friction sequence (all backend/IPC green, frontend-only
except item 4):
1. **TS bridge batch** — `ipcUpdateProduct`, `ipcCreateWarehouse`,
   `ipcAdjustStock`, `ipcTransferStock` in `src/lib/ipc.ts` + wire tests
   (mirrors `ipc-conversion.test.ts` pattern). No Rust.
2. **Product edit modal** — full-state form (names, category, active
   toggle, unit price/barcode grid) inside `CatalogModule`
   (manager-only, like creation). No Rust.
3. **Warehouse admin screen + transfer modal** — create form (ADMIN),
   list reuse, transfer form (source/dest selectors, product, qty) with
   `TRANSFER_SAME_WAREHOUSE` / `WAREHOUSE_INACTIVE` surfacing. No Rust.
4. **(Optional, only if ordered) min_stock thresholds** — new nullable
   column + migration, port/use-case/DTO extension, LowStock report
   redefinition, UI field. The sole task touching Rust + schema.

No production or test file was modified (only this new audit file was
created).
