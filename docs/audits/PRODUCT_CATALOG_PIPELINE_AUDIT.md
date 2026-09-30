# PRODUCT CATALOG PIPELINE AUDIT — Empty Catalog on Fresh Database (TASK-BRK-027)

> **Nature:** strict read-only static audit. Zero files modified. Every claim
> cites file path + line number / identifier from the current source tree.
>
> **Opening confirmation (mandated):** Product catalog table is confirmed
> empty via live smoke test (SESSION-BRK-007 screenshot context, reported by
> the project owner in the current 2026-09-27 session): searching "a" in the
> POS search box returns "Aucun produit trouvé" with zero results and the
> cart stays "Panier vide".

---

## Section 1 — Backend Command Inventory (`crates/commands/src/ipc.rs`)

Exactly **4** product-related IPC commands exist; all are registered in
`src-tauri/src/lib.rs:133-136` and all are fully implemented (no stubs):

| Command (snake_case) | Handler lines | Registration | Status |
|----------------------|---------------|--------------|--------|
| `catalog_create_product` | ipc.rs:544-581 | lib.rs:133 | Fully implemented (RBAC via use case, terminal_id threaded) |
| `catalog_get_product` | ipc.rs:583-597 | lib.rs:134 | Fully implemented |
| `catalog_search` | ipc.rs:598-627 | lib.rs:135 | Fully implemented (limit clamp, kit-item hydration) |
| `catalog_define_kit` | ipc.rs:628-654 (ends where `warehouses_create` begins at 656) | lib.rs:136 | Fully implemented |

**Explicitly absent as IPC:** no `catalog_update_product` (despite the use
case existing — §2), no `catalog_delete_product` / deactivate, no
`catalog_list_products`, and **no category command of any kind** (the only
`categor*` hits in ipc.rs are the `CategoryRepository` port import at line
11/106-107, the `category_id` passthrough at 440/553-568, and the
`reports_sales_by_category` read path at 2026-2040).

## Section 2 — Application Use Case Inventory (`crates/application/src/use_cases/catalog.rs`)

| Use case | Location | Status |
|----------|----------|--------|
| `CreateProductUseCase` (sg. `CreateProductInput`, `NewUnitInput`) | catalog.rs:165-236 | Exists (validates SKU/barcode uniqueness, unit-set invariants, category FK at 210-212) |
| `UpdateProductUseCase` (+ `UpdateProductInput`, `UnitUpsert`) | catalog.rs:238-295 | Exists (category re-validation at 273-275) — **but unreachable: no IPC handler** |
| `DefineKitUseCase` (+ `NewKitItemInput`) | catalog.rs:297-383 | Exists (cycle-checked) |
| `GetProductWithUnitsUseCase` (+ `ProductDetail`) | catalog.rs:384-424 | Exists |
| `SearchCatalogUseCase` | catalog.rs:425-455 | Exists — `require(session, Permission::PosSell)` (line 448); blank query → `Ok(vec![])` (lines 449-452) |
| CreateProductUnit (standalone) | — | Absent as a use case (units are created atomically inside `CreateProductInput.units` / `UnitUpsert`; no independent unit use case) |
| CreateBarcode (standalone) | — | Absent (barcode is a field on unit/product inputs, validated for duplicates; no independent use case) |
| DeleteProduct / DeactivateProduct | — | **Entirely absent** (no struct, no port method) |
| ListProducts | — | **Entirely absent** (only prefix `SearchCatalogUseCase`) |
| CreateCategory / any category use case | — | **Entirely absent** (`CategoryRepository` port exists with `create_category`, but zero use cases touch it outside test fakes at catalog.rs:645-670) |

## Section 3 — Domain Entity Implementation Status (`crates/domain/src/catalog/`)

Implemented per invariant, fully cited:

- `product.rs:43` `Sku`, `product.rs:66` `Barcode`, `product.rs:89` `Product`
  (sku/barcode/name_ar/name_fr/category/is_kit/is_active).
- `unit.rs:50` `ProductUnit` with `unit.rs:60` `factor_micro: FactorMicro`,
  `unit.rs:68` `is_base_unit`; base-unit invariant (exactly one base unit at
  factor 1, all factors positive) enforced at `unit.rs:80-103` raising
  `DomainError::BaseUnitMismatch` (unit tests at unit.rs:187-219).
- `kit.rs:17` `KitItem` (component/unit/quantity); BOM cycle detection at
  `kit.rs:51-68` raising `DomainError::KitCycleDetected` (tests at
  kit.rs:99-121).
- `category.rs:42` `Category` (id/code/name).

Verdict: domain layer is **complete** — entities, invariants, and error
variants all exist with unit tests. Nothing domain-side blocks catalog
population.

## Section 4 — Database Migration Seed Audit (0001 → 0012)

Enumerated files in `crates/infrastructure/migrations/`: 0001_baseline_schema,
0002_auth_lockout, 0003_stock_engine_schema, 0004_sales_engine_schema,
0005_customer_module_indexes, 0006_print_worker_columns, 0007_document_language,
0008_cash_session_engine_schema, 0009_supplier_purchase_engine_schema,
0010_reporting_indexes, 0011_add_converted_document_status,
0012_seed_default_warehouse.

Grep for `INSERT INTO (products|categories|product_units|product_kit_items|product_barcode)`
across all 12 files returns **zero matches**. The sole data-seeding migration
is 0012, which inserts exclusively into `warehouses` (verified by authorship
in TASK-BRK-025). **Confirmed with certainty: zero product/category/unit/kit
seed rows exist across the entire migration history** — and per commercial
correctness (context statement, undisputed) none must ever be added.

## Section 5 — Frontend Component Inventory (`src/`)

Grep for `ProductForm|ProductManagement|CatalogAdmin|NewProduct|AddProduct|EditProduct|ipcCreateProduct|catalog_create|ProductModal`
returns **zero product-management hits**. The only `addProduct` matches are
cart-line additions (`cart-store.ts:61,77`, `ProductDiscovery.tsx:26,118`,
`use-catalog-search.ts:122`) — putting a found item into the cart, unrelated
to catalog administration.

Wiring status: **does not exist at all** (not even orphaned). This is a
stronger absence than the TASK-BRK-024 `warehouses_create` pattern (backend
present + unreachable); here there is no screen, form, modal, or draft code
to wire up.

## Section 6 — Navigation/Routing Audit (ADMIN view)

`src/App.tsx` NavRail (lines 24-51, rendered for managers at line 109)
contains exactly three entries — `entry("pos"…)`, `entry("suppliers"…)`,
`entry("reports"…)` (lines 47-49) — with view switching at lines 111-113.
**No catalog/product menu item, button, or route exists for any role,
including ADMIN.** A product management screen would have nowhere to mount
without a NavRail addition.

## Section 7 — Search Pipeline Verification (F1 input → SQLite)

Traced hop by hop, all verified wired to the **real** `products` table:

1. `src/components/pos/ProductDiscovery.tsx:50` — `<input id="pos-search-input"
   placeholder={t(locale,"search.placeholder")} /* "Recherche produit… (F1)" */>`.
2. `src/hooks/use-catalog-search.ts:40-72` — 150 ms debounce → session check
   → `ipcSearchCatalog(session.token, query, 20)` (line 56) → results/error
   state; empty results render `t(locale,"search.noResults")` at
   ProductDiscovery.tsx:106 ("Aucun produit trouvé").
3. `src/lib/ipc.ts` `ipcSearchCatalog` → `catalog_search` IPC → §1 handler →
   §2 use case (PosSell gate) → `SqliteCatalogRepository::search`
   (`crates/infrastructure/src/adapters/sqlite_catalog_repository.rs:553-591`):
   escaped LIKE-prefix on `p.sku / p.name_fr / p.name_ar` plus exact
   `u.barcode = ?2 OR p.barcode = ?2`, `ORDER BY p.sku LIMIT ?3`.
4. **No FTS5 anywhere**: grep for `fts5|FTS|VIRTUAL TABLE|trigram` across all
   migrations returns zero; the query above hits base tables directly.

Conclusion: the path is **correctly wired, no broken query, no wrong table,
no FTS index mismatch**. "Aucun produit trouvé" for "a" is the truthful
output of a correct query over zero rows. (Note: LIKE-prefix means "a" matches
only names/SKUs *starting with* "a" — correct behavior on real data, not a
defect.)

## Section 8 — Gap Classification and Recommendation

**Classification: (b) — Backend exists but Frontend missing/orphaned.**
Backend create/get/search/kit-define are implemented, registered, RBAC-gated,
and reachable; domain and schema are complete; the search path is sound. What
is missing is 100% frontend: no management screen, no nav entry, and (noted
for completeness) no IPC exposure of the existing update use case and no
delete/deactivate or category-management capability at any layer. It is
*not* (a) (backend substantially exists), *not* (c) (there is nothing built
to connect), and *not* (d) (creation is not blocked — it is uninvokable).

**ONE recommendation:** authorize a single "Product Catalog Management"
task delivering an ADMIN-gated catalog screen (create product with unit set +
barcodes, edit via a newly exposed `catalog_update_product` IPC reusing the
existing `UpdateProductUseCase`, barcode-exact lookup reuse) mounted as a
fourth NavRail entry — the smallest build that lets the owner populate real
store-specific data through the already-proven backend, with category
management and product deactivation explicitly deferred to a follow-up.
