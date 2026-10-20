# TASK-AUD-030 — Deep-Dive Audit: Feature-Level Role Checks (Customers, Stock, Catalog, Settings)

- Task: TASK-AUD-030 (read-only audit) · Workspace: D:/Dev/Baraka UltraPOS · Branch: master
- Baseline: worktree carries BRK-070 + DSN-071 + DSN-072 change sets (uncommitted); this audit reads that state.
- Output: `audit/AUD-030/REPORT.md` (this file — the ONLY file created).

## 0. Headline

1. The AUD-029 20-row inventory is reproduced verbatim below (§1), grouped by module.
   Post-DSN-072, row 1 (`App.tsx` manager gate) is **already migrated**; rows 3–18 remain.
2. **Task-text corrections (zero guessing, all verified):** the named files
   `StockAdjustModal.tsx`, `StockTransferModal.tsx`, `BackupPane.tsx`, `StoreIdentityPane.tsx`
   **do not exist** — stock adjust/transfer live in POS modals + `StockModule` panes; settings uses
   `BackupSettingsTab.tsx` / `StoreIdentityTab.tsx`. The named permissions `CUSTOMER_PAYMENT_RECORD`
   and `STOCK_TRANSFER` **do not exist** in `crates/domain/src/role.rs` — the real variants are
   `RecordCreditPayment` and `TransferStock`. `CREDIT_LIMIT_OVERRIDE` exists but governs POS
   checkout limits, **not** customer credit-limit editing (that is `ManageCustomerCredit`,
   `customers.rs:250,283,313`).
3. **Restore is NOT role-checked in Rust** — `backup.rs:257-265` (`require_restore_admin`) requires
   the `SystemSettingsManage` *permission*; it is Admin-only only because no other role holds that
   permission (`role.rs:121-167`). Batch 2 must migrate to the permission, not the outcome.
4. **Backup create/list have NO engine permission gate** (single `Permission::` in `backup.rs`, line 261)
   — yet the whole `BackupSettingsTab` is admin-hidden at UI. Widening candidate for Batch 2.
5. Dominant UX pattern for unauthorized actions is **hide** (`{flag && <button/>}`), not disable;
   the two exceptions use **disabled + inline error**: `FiscalSettingsTab` (`disabled={!isAdmin}`,
   `:53,86`) and credit-limit submit (`CustomerFormModal.tsx:61-64`,
   `CustomerSelectModal.tsx:168-171` — `UNAUTHORIZED` error on submit, limit input hidden at `:398`).
   No tooltips explain any hidden control anywhere inspected.

## 1. AUD-029 20-row inventory (verbatim, grouped by module)

### App shell
| # | File:line | Expression | Guards | Target permission (engine-verified) | Verdict |
|---|---|---|---|---|---|
| 1 | `App.tsx:28` | `role === "GERANT" \|\| role === "ADMIN"` | rail + 6 routes | manager bundle (no single perm) | migrate Batch 1 |
| 2 | `App.tsx:40-41,124-125` | `hasPermission("USER_MANAGE")` | users nav + route | `UserManage` | DONE (reference impl) |

### Customers + POS credit flows
| # | File:line | Expression | Guards | Target permission (engine-verified) | Verdict |
|---|---|---|---|---|---|
| 4 | `CustomerSelectModal.tsx:28` `canCreate` | C/G/A | new-customer button | `CreateCustomer` (customers.rs:87,164) | EXACT — direct swap |
| 5 | `CustomerSelectModal.tsx:32` `canManageCredit` | G/A | credit controls | `ManageCustomerCredit` | EXACT |
| 6 | `RecordPaymentModal.tsx:57` `isVendeur` | `=== "VENDEUR"` (inverted block) | blocks payment recording | NOT `RecordCreditPayment` (customers.rs:346) | EXACT (inverted) |
| 7 | `RecordPaymentModal.tsx:62` `canUseOverpayment` | G/A | negative-balance approval | `ManageCustomerCredit` | EXACT |
| 8 | `CustomerFormModal.tsx:31` `canManageCredit` | G/A | credit-limit fields | `ManageCustomerCredit` | EXACT |
| 12 | `CustomersModule.tsx:66` + `CustomerFormModal.tsx:31` | G/A | whole directory | reads `ViewCustomers` (ALL); create `CreateCustomer` (Caissier+); credit `ManageCustomerCredit` (G/A) | **OVER (reads + Caissier create)**; credit OK |

### Stock + checkout gate
| # | File:line | Expression | Guards | Target permission (engine-verified) | Verdict |
| 9 | `stock-gate.ts:15` `isPrivilegedStockRole` | A/G | sale-on-shortage | `NegativeStockOverride` (sales ×2, inventory ×1) | EXACT |
| 10 | `CheckoutModal.tsx:171` `evaluateStockGate({role…})` | role string in | checkout confirm | `NegativeStockOverride` | EXACT |
| 14 | `StockModule.tsx:58` + `:60` `isAdmin` | G/A ; A | module ; warehouse create | `ViewStock` etc. ; `ManageWarehouses` (inventory ×1, deletion ×2) | **OVER (reads)**; admin OK |

### Catalog
| # | File:line | Expression | Guards | Target permission (engine-verified) | Verdict |
| 11 | `CatalogModule.tsx:247,715` `isManagerRole` + `authed` | G/A | whole hub incl. search | reads `PosSell` (ALL roles); writes `StockAdjustment` OR `SystemSettingsManage` | **OVER (reads)**; writes OK |

### Settings / identity / backup / transfer
| # | File:line | Expression | Guards | Target permission (engine-verified) | Verdict |
| 3 | `OperationsMenu.tsx:40-41` | `manager` / `admin` in `operationsForRole` | stock section (transfer, adjust) / system section (backup, identity) | `TransferStock`+`AdjustStock` / `BackupRestoreManage`+`SystemSettingsManage` | Batch 2 |
| 15 | `SettingsModule.tsx:157` `isAdmin` | A | fiscal/printer/backup tabs + lock badges | `SystemSettingsManage` / `ManagePrinterSettings` (Admin-only) / restore Admin-only | OK |
| 16 | `StoreIdentityModal.tsx:68,74` + `store-identity-store.ts:119-120` `canManageIdentity` | A | identity load + render null | `SystemSettingsManage` (printing.rs:159) | EXACT |
| 17 | `backup-modal-store.ts:65-66` `canRestoreForRole` | A | restore button (`BackupModal.tsx:399-402`) | Admin-only restore gate (`backup.rs:257-265`) | EXACT |
| 18 | `warehouse-transfer-store.ts:63-68` `isAdminRole`/`isManagerRole` + `WarehouseTransferModal.tsx:292-293` | A / G/A | warehouse pickers vs transfer submit | `ManageWarehouses` / `TransferStock` (inventory ×1) | EXACT |

### Non-gates (no migration)
| # | File:line | Expression | Guards | Target permission (engine-verified) | Verdict |
|---|---|---|---|---|---|
| 19 | `user-helpers.ts:101-104` metric filters | `=== "ADMIN"` / `"CAISSIER" \|\| "VENDEUR"` in `filter` predicates | HUD counts only (not gates) | N/A | not gates — no migration |
| 20 | `Topbar.tsx:121` | displays `session.role` | informational badge | N/A | not a gate |

POS/F7 overrides, discount modals, cash closing, return authorization: all *** through
rows 6, 9, 10 (`RecordPaymentModal`, `stock-gate`, `CheckoutModal`) — no other POS role
checks exist. `suppliers/`, `purchases/`, `reports/` roots: zero internal checks (App gate + engine).

## 2. Customers deep-dive

- **Directory access** (`CustomersModule.tsx:24-26,66-67,104-112`): local `isManagerRole`
  (GERANT/ADMIN); `token` nulled for others so `CustomerListPane` never fetches; `!authorized`
  renders a red `UNAUTHORIZED` panel (hidden-pattern at module scale, no guidance).
  Engine reads need only `ViewCustomers` (ALL roles) → over-gated (Batch 2: split read/create/credit).
- **"Nouveau Client"** (`CustomerListPane.tsx:31,72-78` → `onNew` → `CustomersModule.tsx:139-141`
  `openCreate` → `CustomerFormModal` create mode): button itself **unprotected** — covered only by
  the module gate. Engine create needs `CreateCustomer` (Caissier+). Post-migration the button must
  stay visible for Caissier (they hold it) — a pure read/create split, no new hide.
- **Credit-limit editing** (`CustomerFormModal.tsx:30-33,61-64,398`): limit *input* hidden unless
  `canManageCredit(role)` (`:398`); submit re-checks and surfaces `UNAUTHORIZED` inline (`:61-64`).
  `CustomerDossierPane.tsx:116-123` edit entry is unprotected (form gates at save). Target is
  `ManageCustomerCredit`, NOT `CREDIT_LIMIT_OVERRIDE` (POS checkout limit). Pattern: hide + submit guard.
- **Payment recording** (`RecordPaymentModal.tsx:57,62,95-97,194-195`): `isVendeur` → validation
  `{ ok: false, key: "payment.vendeurRestricted" }` (disabled-at-submit with message — the only
  role-gated control with user-facing text); `canUseOverpayment` gates negative-balance approval.
  Targets: NOT `RecordCreditPayment` (inverted) / `ManageCustomerCredit`. Real variant names confirmed.

## 3. Stock & catalog deep-dive

- **Stock module** (`StockModule.tsx:34-36,58-60,116-124,205-222`): module `authorized` G/A with red
  `UNAUTHORIZED` fallback; transfer button (`:205-211`) unprotected inside (module gate covers);
  warehouse-create hidden via `{isAdmin && ...}` (`:212-221`, no tooltip). Engine: reads `ViewStock`
  (Vendeur+), create `ManageWarehouses` (Admin). Batch 2: read surface to `VIEW_STOCK`, keep create hidden.
- **Catalog hub** (`CatalogModule.tsx:245-247,715` + `:702,943-962`): whole hub behind `authed` G/A;
  create/edit/delete/toggle/label buttons (`:955-962`, `:636-658`) unprotected inside. Engine reads
  `PosSell` (all roles); writes `StockAdjustment` OR `SystemSettingsManage`. Batch 2: browse to
  `POS_SELL`, mutate surfaces to the OR-pair (mirrors DSN-072 settings precedent).
- Named `StockAdjustModal`/`StockTransferModal` do not exist; adjust/transfer modals live under
  `src/components/pos/modals/` (rows 9, 10, 18 — unchanged from AUD-029).

## 4. Settings, backup, identity deep-dive

- **Tab shell** (`SettingsModule.tsx:157,183-185,224`): `isAdmin` from `session?.role`; admin-only tabs
  get lock badges; fiscal/backup tabs receive `isAdmin` prop.
- **Backup** (`BackupSettingsTab.tsx:16-22,44-53,61-70`): non-admin early-returns a
  `settings.backupRestricted` notice (whole tab incl. *create* hidden). Engine: create/list have **no**
  permission gate (sole `Permission::` in `backup.rs` is restore, `:261`); restore requires
  `SystemSettingsManage` (`require_restore_admin`, `backup.rs:257-265`) — effectively Admin-only since
  Gerant lacks it, but the check is permission-based and Batch 2 must mirror the permission.
- **Identity** (`StoreIdentityTab.tsx:72,212-213`): no role gate; save `disabled={!canSave}` (form
  validity only); engine save/load `SystemSettingsManage` (`printing.rs:159`). Same for
  `PrinterSettingsTab` (engine `ManagePrinterSettings`, Admin-only; UI ungated, backend authoritative).
- Named `BackupPane`/`StoreIdentityPane` do not exist (tabs, see above).

## 5. Test inventory & coupling (Vitest files asserting roles in these 4 modules)

| File | Role-coupled sites (must change in Batch 2) |
|---|---|
| `src/features/stock/stock-module.test.ts:96` | asserts `MODULE_SRC` contains `session.role === "ADMIN"` |
| `src/features/settings/settings-module.test.ts:47,76` | asserts no ADMIN-gated settings entry; asserts `session?.role === "ADMIN"` |
| `src/features/stock/stock-adjust-row-prefill.test.ts:46` | seeds `role: "GERANT"` SessionInfo (`permissions: []`) |
| `src/features/catalog/__tests__/CategoryManageModal.test.tsx:346` | seeds `role: "GERANT"` (`permissions: []`) |
| `src/features/catalog/__tests__/ProductImportModal.test.tsx:100` | seeds `role: "GERANT"` (`permissions: []`) |
| `src/features/customers/customers-module.test.ts` | APP_SRC pins already migrated (DSN-072); SessionInfo seeds (if any) carry `permissions: []` |
| `src/features/settings/fiscal-settings-tab.test.ts`, `settings-command-center.test.ts` | no role strings found (verify at Batch 2 start) |

Migration rule per seed: replace `role: "X"` seeds with real permission arrays (DSN-072 `PERMS` table in
`src/navigation-permissions.test.tsx` is the canonical mirror of `role.rs:121-203`) and assert
`hasPermission(...)` call sites, not role strings.

## 6. UX recommendation (quincaillerie-friendly)

Keep hide for genuinely unreachable surfaces (module fallbacks, warehouse-create, limit input) but
attach the DSN-072 `AccessDenied` semantics at every *denied click*: the two submit-guard sites
(`CustomerFormModal`, `CustomerSelectModal` credit paths) already surface `UNAUTHORIZED` inline —
extend that exact pattern to transfer-submit and settings-save, e.g. disabled save + inline
`common.accessDenied` hint naming the missing permission code. Never expose a button whose engine
call is guaranteed `Unauthorized` without telling the operator which habilitation to request from
the Gérant — the current silent hides cost counter time (AUD-029 §3 impact stands).

## 7. Line endings

Byte-counted (`tr -cd '\r' | wc -c`): `src/App.tsx` 303/303 CRLF, `src/lib/i18n.ts` 2314/2314 CRLF
(HEAD conventions); feature sources pure LF; this report LF. Zero files modified by this task, so no
normalization was needed.

## 8. Quality gates (verbatim, run after writing this report — docs-only change)

### G1 `cargo fmt --check`
```
(empty output — clean)
G1_EXIT:0
```
### G2 `cargo clippy --all-targets --all-features -- -D warnings`
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 31.28s
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
dist/assets/index-BY5p3b2H.js              727.32 kB │ gzip: 187.35 kB
✓ built in 34.18s
```
(slower than usual — cargo test ran concurrently; same output hash names as DSN-072 build)
### G7 `npx vitest run`
```
Tests  662 passed (662)
   Start at  20:51:18
   Duration  49.78s (transform 14.03s, setup 0ms, collect 21.02s, tests 2.75s, environment 27ms, prepare 37.48s)
```
