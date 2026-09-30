# WAREHOUSE LIFECYCLE AUDIT — Empty "Dépôt" Selector on Fresh Database (TASK-BRK-024)

> **Nature:** strict read-only static audit. Zero files modified. All claims cite
> file path + line number from the current source tree.

---

## Q1 — Schema & Constraints: does `warehouses` exist and what does it enforce?

**Yes.** `crates/infrastructure/migrations/0001_baseline_schema.sql:82-88`:

```sql
CREATE TABLE IF NOT EXISTS warehouses (
    id TEXT PRIMARY KEY,
    code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1
) STRICT;
```

Extended by `crates/infrastructure/migrations/0003_stock_engine_schema.sql:16-19`:

```sql
ALTER TABLE warehouses ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE warehouses ADD COLUMN origin_device_id TEXT;
ALTER TABLE warehouses ADD COLUMN created_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z';
ALTER TABLE warehouses ADD COLUMN updated_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z';
```

Constraints summary: `id` PK, `code` UNIQUE NOT NULL, `name` NOT NULL, no
CHECK constraints, no FKs *into* warehouses; outbound FKs reference it
(`stock_levels.warehouse_id`, `documents.warehouse_id`,
`cash_sessions.warehouse_id` — e.g. `0001:92`
`warehouse_id TEXT NOT NULL REFERENCES warehouses(id)`).

**Notable asymmetry:** the `is_default` column (0001:86) is **write-only at
the DB layer** — the domain entity
(`crates/domain/src/inventory/warehouse.rs:63-74`) exposes only
`{id, code, name, is_active}`, and the repository row mapper
(`crates/infrastructure/src/adapters/sqlite_warehouse_repository.rs:40-47`)
reads columns `(id, code, name, is_active)` by positional index, never
`is_default`. No code anywhere sets or reads a default-warehouse flag. The
flag is dead schema.

## Q2 — Initial Data & Seeding: is a default warehouse ever inserted?

**No.** Exhaustive grep for `INSERT INTO warehouses|users|settings` across
all 11 files in `crates/infrastructure/migrations/` returns **zero matches**.
No migration seeds anything; there is no seed function anywhere outside
`#[cfg(test)]` fixtures.

Warehouse creation was **omitted from the First-Run Setup Wizard**:
`src/features/auth/FirstRunSetupScreen.tsx` (full file, ~100 lines, verified
in TASK-BRK-019) contains exactly three fields — username, password, password
confirmation — and calls only `bootstrap_first_admin` + `login`. No warehouse
step, no post-setup seeding call. So on a fresh database: zero users → wizard
creates one Admin → still **zero warehouses**, by construction.

## Q3 — Application & IPC Surface

**Use cases** (`crates/application/src/use_cases/inventory.rs`) — exactly two,
no update/deactivate path (exhaustive `pub struct *Warehouse*` grep):

- `CreateWarehouseUseCase` (inventory.rs:78-128): ADMIN-only
  (`Permission::ManageWarehouses`, line 103), validates code 1–20 chars via
  `WarehouseCode::new` (`crates/domain/src/inventory/warehouse.rs:46-54`),
  name 1–100 chars (`warehouse.rs:78-86`), rejects duplicate code with
  `Conflict("WAREHOUSE_CODE_EXISTS")` (lines 113-117), persists with
  `terminal_id` (lines 124-125).
- `ListWarehousesUseCase` (inventory.rs:131-151): any authenticated session,
  delegates to `list_active()`.

**Repository** (`sqlite_warehouse_repository.rs:124-135`): `list_active`
runs `SELECT ... WHERE is_active = 1 ORDER BY code` and returns
`Ok(warehouses)` — on an empty table this is **`Ok(vec![])`, never an error**.

**IPC** (`crates/commands/src/ipc.rs`): `warehouses_create` (lines 669-690,
ADMIN-gated via the use case) and `warehouses_list` (lines 692-713, maps the
vec 1:1 into `ListWarehousesResponseDto`). Both registered in
`src-tauri/src/lib.rs:141-142`. On empty: `Ok({warehouses: []})` — a
**successful empty response**, so the frontend receives no error signal, only
absence.

## Q4 — Frontend Consumption & Navigation

**Single fetch point:** `loadWarehouses()` in
`src/stores/session-store.ts:78-82` calls `ipcListWarehouses()` once per
login and stores `warehouses: []`, `warehouse: active[0] ?? null` → `null`
(lines 84-90). No retry, no empty-state warning.

**Consumers of the empty state** (all read-only selectors, none can create):

- `src/components/pos/Topbar.tsx:38-40,66-81` — the "Dépôt" `<select>`: with
  zero rows renders only `<option value="">Aucun dépôt</option>` (line 73).
- `src/components/pos/ProductDiscovery.tsx:25,91-93` — blocks search with
  `search.noWarehouse` ("Sélectionnez d'abord un dépôt") when
  `warehouse === null`.
- Purchase screens (`GoodsReceiptFormScreen.tsx:24-43`,
  `PurchaseOrderFormScreen.tsx:165-183`,
  `SupplierInvoiceFormScreen.tsx:29-48`) — re-call `loadWarehouses()` when
  empty (still `[]`), then fail submissions with `WAREHOUSE_NOT_FOUND`.
- Reports (`LowStockReport.tsx:18`, `StockMovementReport.tsx:24`, …) render
  empty warehouse filter dropdowns.

**Management UI: does not exist.** Grep for `warehouses_create |
ipcCreateWarehouse | CreateWarehouse | WarehouseForm | WarehouseManage` across
all of `src/` returns **zero matches** (only type-level `WarehouseInfo` /
`warehouseId` filter fields). `src/features/` contains only
`auth/health/purchases/reports/suppliers` — no warehouse module, no route, no
navigation button. The backend `warehouses_create` command is **registered but
unreachable**: it exists, is ADMIN-gated, and has exactly zero frontend
callers — an orphaned capability.

## Q5 — Root Architectural Diagnosis

This is the **structural sibling of the ADR-048 Admin-bootstrap gap**: the
first-run wizard provisions identity (one Admin) but not operating context
(at least one warehouse), no seed covers it, and no UI can create it afterward.
A fresh database is therefore warehouse-less by construction, and every
downstream flow degrades silently-but-correctly to empty (never an error):

`warehouses_list → Ok([])` → `warehouse: null` → Topbar "Aucun dépôt" →
POS search blocked → cash session cannot bind a warehouse
(`OpenCashSessionUseCase` requires an existing active warehouse:
`crates/application/src/use_cases/cash.rs:92-95` rejects unknown/inactive
with `NotFound("warehouse")` / `WAREHOUSE_INACTIVE`) → checkout impossible.

**Fix-option analysis:**

- **(A) Automated seed in migration/bootstrap — DISCOURAGED.** Data-seeding
  inside schema migrations conflates versioning concerns; it also cannot
  reach already-initialized databases without a dedicated data migration,
  and a hardcoded code/name cannot satisfy per-site reality (multi-depot
  shops). The `code UNIQUE` constraint would make it idempotent, but that
  does not cure the layering smell.
- **(B) Warehouse step in the First-Run Setup Wizard — RECOMMENDED as the
  primary fix.** The wizard is the established per-database provisioning
  point (TASK-BRK-016 pattern); adding "create Dépôt Principal (code + name,
  Admin-only, one-time, skippable only if a warehouse already exists)" closes
  the fresh-DB hole with no migration, reusing the existing
  `warehouses_create` backend verbatim.
- **(C) Admin warehouse-management UI — REQUIRED as the follow-up regardless.**
  Real shops need 2nd-depot creation, renaming, and deactivation; the backend
  already supports create+list (update/deactivate use cases would need a small
  additive task). (C) alone does not fix first-run (chicken-and-egg: an Admin
  with no warehouse still needs the first one from somewhere), so (C)
  complements (B), it does not replace it.

**Recommended sequencing: (B) now to unblock first-run, (C) next for
operational completeness; (A) rejected.**
