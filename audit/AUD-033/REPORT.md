# TASK-AUD-033 REPORT — Full read-only inventory at HEAD (2026-10-20)

HEAD: `0104e4b81ad1207b477778500bd4ae69681b7530` — "feat(permissions): align
Customers/Stock/Catalog root gates to granular hasPermission and gate
sensitive actions with bilingual guidance (TASK-DSN-077)". Tree clean
(`git status --short` empty). Branch `master`. Tags include
`baseline-engine-pre-redesign` and `v1.0.0-rc.1` … `v1.0.0-rc.10`.
Toolchain: node v26.7.0, rustc 1.92.1. App version 1.0.0
(package.json:4, src-tauri/tauri.conf.json:4).

## 1. Git

- HEAD hash + message: see above (`git rev-parse HEAD`, `git log --oneline -30`
  stored in COMMITS.txt).
- `git status --short`: empty (only untracked `audit/AUD-033/` created by this task).
- `git diff --stat HEAD~5..HEAD`: 100 files, +8617/−1639 (spans BRK-074, DSN-075,
  DSN-076, DSN-077: inactive-leak fix, metrics IPC, table layout, quick-products,
  permission realignment).

## 2. Engine

- Migrations: 15 files `0001…0015` (crates/infrastructure/migrations/),
  457/5/34/30/16/18/10/144/159/22/39/32/69/33/3 lines, 215–15797 bytes.
- Checksum mechanism: `schema_migrations(version, name, checksum)` verified by
  SHA-256 over embedded SQL — crates/infrastructure/src/persistence/migration.rs:24
  (`MIGRATIONS`), :103 (`checksum_of`), :120 (verification loop); backup path
  re-checks at sqlite_backup_manager.rs:569.
- Tables (32 `CREATE TABLE` statements, incl. `_new`/`_tmp` rebuild artifacts):
  schema_migrations, settings, users, categories, products, product_units,
  product_kit_items, warehouses, stock_levels, stock_movements, customers,
  customer_ledger, suppliers, supplier_ledger, documents, document_items,
  payments, cash_sessions, cash_movements, document_sequences, print_jobs,
  command_log, audit_log, backups_log, tax_rules (+ rebuild twins).
- IPC: **92** Tauri commands registered (src-tauri/src/lib.rs; deduped list +
  count in COMMANDS.txt; handlers in crates/commands/src/ipc.rs, e.g.
  `catalog_search` :930, `catalog_get_metrics` :962, `app_health` :205,
  last handler `reports_user_activity` :3856).
- Permission enum: **34** values (crates/domain/src/role.rs:27…) incl. PosSell:28,
  StockAdjustment:38, SystemSettingsManage:42, ViewStock:45, AdjustStock:47,
  TransferStock:49, ManageWarehouses:51, ViewCustomers:55. Role matrices at
  role.rs:111-203 (Vendeur 5 perms … Admin full set).
- Money/quantity: `Money(i64)` newtype (crates/domain/src/primitives.rs:49);
  only `f32` in domain is print-layout `size_px`
  (crates/domain/src/printing/receipt_model.rs:116, non-financial). No floats
  in financial paths.
- SQLite: rusqlite 0.32 `bundled, backup, modern_sqlite, hooks` (Cargo.toml:27);
  no SQLCipher; no FTS5 in migrations.

## 3. Frontend

- Feature modules (12 under src/features/): auth, catalog, customers,
  documents, health, pos, purchases, reports, settings, stock, suppliers,
  users. Gating: nav shell decided exclusively by `hasPermission(...)`
  (src/App.tsx:8,90-100); module roots — customers `VIEW_CUSTOMERS`
  (CustomersModule.tsx:67), stock `VIEW_STOCK` (StockModule.tsx:~115),
  catalog `POS_SELL` + `canManageCatalog` (CatalogModule.tsx:~720);
  **residual legacy**: `isManagerRole` still present in
  src/features/documents/DocumentsModule.tsx (only legacy root gate left).
  Action buttons in stock/catalog render disabled + bilingual `title`
  guidance (DSN-077); label-print stays enabled.
- Stores (src/stores/): app-store, app-view-store, backup-modal-store,
  cart-store, cash-session-store, conversion-modal-store, payment-modal-store,
  pos-ui-store, product-edit-store, quick-products-store (enabled flag
  `baraka-quick-products-enabled`, active-only assign/dispatch),
  return-modal-store, session-store (`hasPermission` reads live permissions,
  session-store.ts:104), stock-adjust-modal-store, store-identity-store,
  supplier-store, theme-store, warehouse-transfer-store.
- i18n (src/lib/i18n.ts): ~2070 key lines; locales `fr-DZ`, `ar-DZ`
  (i18n.ts:2110-2111); includes DSN-077 `stock.*ReservedGuidance` ×3 +
  `cat.manageReservedGuidance` (FR+AR).
- Fonts: none bundled (no .woff2/.ttf/.woff under src); Tailwind references
  Inter/Cairo/JetBrains Mono by name only (tailwind.config.js:49-52).
  Design tokens: primary `#0F3D5C`, accent `#F59E0B` (tailwind.config.js:9-12).
- `position: fixed`: zero matches in src/** (prohibition holds).

## 4. Quality/docs history

- audit/ dirs with REPORT.md first headings (sample): BRK-057 "suppliers_list
  IPC Capability"; DSN-069 "Quick-Products Palette (Alt+1..8)"; DSN-068
  "Final 5 Family-2 Contrast Remediation"; DSN-071 "Users Module Token
  Harmonization…"; AUD-031 "Live End-to-End Systemic Verification";
  AUD-026 "Historical CTP Consolidation". **DSN-075 and DSN-076 have no
  REPORT.md** — evidence is `capture.mjs`/`drill.mjs` + results JSON + PNGs
  (DSN-075: 12/12 CDP steps; DSN-076: 21/21).
- ADR: max **ADR-100** (INDEX.md:107, ACCEPTED); next free **ADR-101**
  (INDEX.md:109). CTP-SESSION-ARC-023.md + BOOT-SESSION-ARC-023.md present.
- Last-known gates: GATES.txt (NOT RUN here; G1–G7 all green 2026-10-20,
  G7 = 72 files / 701 tests).

## 5. Deltas vs CTP-023 claims

| Claim | CTP-023 value | Observed HEAD | Verdict |
|---|---|---|---|
| Baseline commit | `cf9d39c` (CTP-023:26) | `0104e4b` (+2: 5dd891f, 0104e4b) | MISMATCH (work landed since) |
| Rust tests (G3) | 464 passed / 5 ignored (CTP-023:69) | 469 / 0 / 0 | MISMATCH (new suites added) |
| Vitest (G7) | 675 / 71 files (CTP-023:73) | 701 / 72 files | MISMATCH (growth) |
| IPC commands | 72 (BOOT-023:122) | 92 registered | MISMATCH (additive growth) |
| Migrations | 13, 0001…0013 (BOOT-023:119) | 15 files, 0001…0015 | MISMATCH (0014, 0015 added) |
| FTS5 search upgrade | planned (CTP-023:101) | absent from migrations | MISMATCH (not implemented) |
| Fonts bundled | NOT bundled, name refs only (BOOT-023:112) | no font files; Cairo/Inter/JetBrains refs only | MATCH |
| `position: fixed` | zero except overlays (CTP-023:13) | zero matches in src | MATCH |
| SQLCipher | absent | rusqlite bundled, no sqlcipher | MATCH |
| `.gitattributes` | `* -text` (BOOT-023:107) | `* -text` | MATCH |
| Session permission set | frontend gates by role name, Rust real authority (BOOT-023:122 NOTE) | `permissions: Vec<String>` on wire; UI via `hasPermission` (ADR-100) | MISMATCH (superseded by design) |
