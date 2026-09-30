# TASK-BRK-048 — Catalog, Stock & Warehouse Capabilities Audit

**Status:** read-only investigation — zero production/test files modified.
**Method:** targeted reads across `crates/domain`, `crates/application`,
`crates/infrastructure`, `crates/commands`, `src-tauri`, and `src/`.

---

## 1. Product editing & price updates

### 1.A Application layer — COMPLETE

- `UpdateProductUseCase` at
  `crates/application/src/use_cases/catalog.rs:238-294` (input
  `UpdateProductInput`, `catalog.rs:90-105`).
- Modifiable: `name_ar` + `name_fr` (both required via
  `Product::validate_names`), `category_id` (existence-checked, `None`
  clears), `is_active` (deactivation = flag flip in the same call, no
  dedicated use case), and the full unit set via `UnitUpsert`
  (`catalog.rs:81-88`; known id merged, unknown id allocates).
- Per-unit mutable (via `build_unit`, `catalog.rs:129-162`): unit names,
  `factor_micro` (conversion factors **are** updatable),
  per-unit `barcode`, `sale_price_centimes`, `purchase_price_centimes`
  (must be ≥ 0), base/default flags — full set re-validated
  (`validate_unit_set`, `:286`).
- NOT modifiable: `sku` (struct doc `:90` "SKU stays immutable"; no code
  path writes it), product-level `barcode` (no field on the input — only
  per-unit barcodes), kit BOM (separate `DefineKitUseCase`, `:296+`),
  `is_kit` flag.
- Price revision tracking: **in-place overwrite, no history/audit.**
  Infra `update_product`
  (`sqlite_catalog_repository.rs:280-334+`) bumps `products.revision + 1`
  (`:291`) and overwrites `product_units` rows (`:307-326`), INSERTing
  unknown unit ids (`:327-333+`). No `price_history` table exists
  (grep: zero matches in `crates/`); old prices are unrecoverable.
- Draft interplay: **no validation against pending drafts** — grep for
  `draft|Draft|DRAFT` in `catalog.rs` returns zero matches. Price edits
  apply immediately to all future pricing; parked drafts carry their own
  snapshots.
- RBAC: mutations require `StockAdjustment` **or** `SystemSettingsManage`
  (`catalog.rs:17-35`, enforced `:266`). Holders — Gerant:
  `StockAdjustment` (`role.rs:150`, no `SystemSettingsManage` in Gerant
  list `:137-167`); Admin: both (`role.rs:181,185` region);
  Vendeur/Caissier: neither → `Unauthorized`. There is NO
  `ManageCatalog`/`ManageProducts` permission (grep: zero matches).

### 1.B Commands & IPC — COMPLETE

- `catalog_update_product` (`ipc.rs:657-695`): parses ids/units/category,
  executes the use case, echoes the fresh detail view
  (`GetProductWithUnitsUseCase` + `to_response`, `:691-694`).
- DTO `UpdateProductDto` (`dto.rs:321-338`): `session_token, product_id,
  name_ar, name_fr, category_id?, is_active, units: Vec<UpdateProductUnitDto>`;
  unit DTO (`dto.rs:298-319`) carries names, `factor_micro`,
  `barcode?`, both prices (canonical strings), base/default flags.
  Response `ProductResponseDto` (`dto.rs:242-265`) includes `is_active`.
- Registered `src-tauri/src/lib.rs:136`.

### 1.C Frontend & TypeScript — COMPLETE (post-BR K-050 work present)

- Wrapper `updateProduct(payload: UpdateProductPayload): Promise<void>`
  (`src/lib/ipc.ts:193-212`, types `:180-201`) mirrors the DTO exactly.
- UI: per-card ✎ edit button in `ProductDiscovery.tsx:144-155`, gated by
  `canEditCatalog` (GERANT/ADMIN, `:32`); `ProductEditModal.tsx` +
  `product-edit-store.ts` mounted in `PosShell.tsx:116` on
  `"edit-product"` (ActiveModal member present).
- Admin display outside POS grid: `CatalogModule.tsx` (creation flow +
  search/list panel reusing `catalog_search`; header comment states
  update/edit lives in the POS discovery trigger, categories/kits/labels
  deferred).

## 2. Stock adjustments & physical inventory

- Use case: `AdjustStockUseCase` (`use_cases/inventory.rs:182-245`).
  `AdjustStock` permission = Gerant/Admin (`role.rs:154,188`;
  Vendeur/Caissier lack it). Explicit `StockDirection`
  (`inventory.rs:18-25`); quantity must be positive (`:207-208`); reason
  mandatory ≥ 3 chars (`ADJUSTMENT_REASON_REQUIRED`, `:210-212`);
  negative outcomes need `NegativeStockOverride` via
  `resolve_adjust_authorization` (`:220-226`).
- Movements emitted: `ADJUSTMENT_POSITIVE` / `ADJUSTMENT_NEGATIVE`
  (`:213-216`); free-text `reason` covers breakage/shrinkage/theft/expired
  (single string field, no reason enum).
- Infra `apply_stock_movement` (`sqlite_stock_repository.rs:425-454`)
  runs one `run_transaction` (`:431`): kit rejection, factor conversion,
  optimistic-concurrency level write, movement row with reason +
  override flag (`:289-314`).
- Command `stock_adjust` (`ipc.rs:864-897`, DTOs `dto.rs:409-437`),
  registered `lib.rs:143`, with `adjust_error_code` mapping.
- **UI gap: no adjust modal exists** and **no TS wrapper exists** —
  grep for `ipcAdjustStock|adjustStock` in `src/lib/ipc.ts` returns zero
  matches; no component/store/test references adjustments. The command is
  reachable only via raw `invoke` today.

## 3. Multi-warehouse management

- Commands (all registered `lib.rs:140-144`): `warehouses_create`
  (`ipc.rs:767-791`, ADMIN-only `ManageWarehouses` — Admin-only per
  `role.rs:342/344` test asserts; duplicate code →
  `WAREHOUSE_CODE_EXISTS`); `warehouses_list` (`:792-814`, bare
  `session_token` scalar, any authenticated session, active ordered by
  code); `stock_get` (`:816-849`, level or 0 + base unit resolution);
  `stock_transfer` (`:899-941`, OUT+IN ids + both new levels echoed).
- DTOs: `CreateWarehouseRequestDto`/`WarehouseDto`/
  `ListWarehousesResponseDto` (`dto.rs:354-383`),
  `TransferStockRequestDto/ResponseDto` (`dto.rs:439-471`).
- Table `warehouses(id, code UNIQUE, name, is_default DEFAULT 0,
  is_active DEFAULT 1)` (`0001:82-88`). Domain `Warehouse`
  (`inventory/warehouse.rs:63-74`) carries **no `is_default`** — default
  is a SQL-seed convention (migration 0012 seeds DEP-01); `list_active`
  ignores it.
- Warehouse resolution: **client-selected, never defaulted.** Checkout
  takes `warehouse_id` from input and validates active
  (`sales.rs:209`, same pattern `:467,645`); frontend
  `session-store.ts` holds `warehouses[]` + `warehouse` (`:14-15`),
  auto-selecting first active on login (`:84-90`) with a Topbar dropdown
  (`selectWarehouse`, `:93-95`).
- Transfer use case (`inventory.rs:248-305`): `TransferStock` perm
  (Gerant/Admin; Caissier explicitly lacks it, `role.rs:340`);
  same-warehouse rejected (`:276-278`); both endpoints must exist+active
  (`:282-290`); shared `transfer_batch_id` (`:299`).
- Atomicity proven in infra (`sqlite_stock_repository.rs:456-502`): one
  `run_transaction` applying `TransferOut` then `TransferIn` (`:463-492`)
  with `authorize_negative_stock: false` (`:473`); engine line 271 makes
  `TransferOut` unconditionally non-negative-capable, so source underflow
  aborts the whole pair — no partial transfer constructible.
- Frontend: wrappers `listWarehouses` (`ipc.ts:253`),
  `createWarehouse` (`:268`), `transferStock` (`:279`) exist; legacy
  `ipcListWarehouses` (`:286`) feeds login; `WarehouseTransferModal.tsx`
  + store mounted in `PosShell.tsx:117` (manager-gated trigger). **No
  adjust wrapper/modal** (see §2).

## 4. POS shell & navigation

- `App.tsx:47-52` NavRail with four views (`pos|suppliers|reports|
  catalog`, `app-view-store.ts:11`), rendered manager-only
  (`isManagerRole` GERANT/ADMIN, `App.tsx:21-22,99,111-117`); non-managers
  see the POS cockpit only. No router; modals mount in `PosShell`
  (`checkout/customer/conversion/return/edit-product/warehouse-transfer/
  payment/discount/quantity/cash-*`, `PosShell.tsx:113-119+`) with FAB
  triggers (conversion, return, warehouse-transfer gated by `manager`,
  `:98`; edit trigger lives per-card in discovery).
- RBAC is view-level (manager rail) + command-level (backend
  permissions); modal triggers re-check roles locally
  (`CatalogModule:19`, `ProductDiscovery:32`, `PosShell:35-36`).

## Gap analysis & roadmap

| Capability | Backend | IPC | TS wrapper | UI | Verdict |
|---|---|---|---|---|---|
| Product edit (names/category/active/units+prices) | ✅ tested | ✅ registered | ✅ `updateProduct` | ✅ modal+trigger | DONE |
| Price history/audit | ❌ overwrite-only | — | — | — | Out of scope unless ordered |
| Draft-vs-price guard | ❌ none (by design) | — | — | — | Documented, no action |
| Stock adjust (corrections) | ✅ perm-gated+tested | ✅ registered | ❌ missing | ❌ missing | Needs wrapper + modal |
| Warehouse create/list | ✅ ADMIN-gated+tested | ✅ registered | ✅ both | ✅ transfer modal | DONE |
| Inter-warehouse transfer | ✅ atomic+tested | ✅ registered | ✅ | ✅ modal+trigger | DONE |
| Default-warehouse concept | ⚠️ SQL-only, domain-blind | — | — | — | No action (works as-is) |

Recommended sequence (all Rust green; frontend-only except noted):
1. `ipcAdjustStock` wrapper + wire test (mirrors `transferStock` pattern).
2. Stock-adjust modal (product picker, warehouse selector, INCREASE/
   DECREASE toggle, reason field ≥ 3 chars, override checkbox for
   Gerant/Admin) mounted in PosShell behind the manager gate.
3. (Optional) categories/kit/label-printing module extensions — the only
   remaining CatalogModule deferred items; no backend gaps found.

No production or test file was modified (only this new audit file was
created).
