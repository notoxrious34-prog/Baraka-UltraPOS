# TASK-AUD-058 REPORT — Permission consistency: `suppliers_list` vs existing supplier search

READ-ONLY audit. Zero files modified, added, or deleted by this task except this report (the sole deliverable).

## 1. Existing supplier search path — permission check
- Use case: `SearchSuppliersUseCase::execute` — `crates/application/src/use_cases/suppliers.rs:162-175`
- Exact permission check line — `crates/application/src/use_cases/suppliers.rs:168`:
  `require(session, Permission::SupplierManage)?;`
- Variant checked: `Permission::SupplierManage`
- Backing Tauri command (delegates auth to the use case): `suppliers_search` —
  `crates/commands/src/ipc.rs` (session resolved via `require_session`, no separate
  `Permission::` check at command level).
- Frontend caller today: `SupplierListScreen.tsx` non-empty-query debounced branch
  calls `ipcSearchSuppliers` (unchanged by TASK-BRK-057).

## 2. Permission enum — Supplier* and Customer* variants (verbatim, file:line)
Declared once: `crates/domain/src/role.rs:27` (`pub enum Permission {`).
Grep `Supplier|Customer` (case-insensitive) over `crates/domain/src/role.rs`:
- `crates/domain/src/role.rs:55`: `ViewCustomers,`
- `crates/domain/src/role.rs:57`: `CreateCustomer,`
- `crates/domain/src/role.rs:59`: `ManageCustomerCredit,`
- `crates/domain/src/role.rs:70-71`: `/// Supplier master-data management (Gerant, Admin only).` /
  `SupplierManage,`
- `crates/domain/src/role.rs:79`: `SupplierPaymentRecord,`
(No other Supplier*/Customer* variants exist; remaining matches at :112-118,
:121, :132-133, :157-159, :162, :166, :192-194, :198, :202 are uses of the same
variants in the `use Permission::{...}` import and role mappings.)

## 3. Is `SupplierManage` the ONLY supplier-related permission?
**No.** Two supplier-related variants exist: `SupplierManage` (:71) and
`SupplierPaymentRecord` (:79). (`SupplierPaymentRecord` gates supplier-payment
recording — `crates/application/src/use_cases/purchases.rs:376`:
`require(session, Permission::SupplierPaymentRecord)?;` — a different operation,
not search or list.)

## 4. Permission checked by the EXISTING supplier search
`Permission::SupplierManage` — exact line `crates/application/src/use_cases/suppliers.rs:168`:
`require(session, Permission::SupplierManage)?;`
This is the SAME variant `suppliers_list` (TASK-BRK-057,
`ListSuppliersUseCase::execute`, `crates/application/src/use_cases/suppliers.rs:201`)
requires. The search path was not narrowed; there is no narrower supplier-search permission.

## 5. Role-to-permission mapping
Found: `Role::permissions()` — `crates/domain/src/role.rs:109-205`.
Roles holding ANY supplier-related permission:
- `Gerant` (:137-167): holds `SupplierManage` (:162) AND `SupplierPaymentRecord` (:166).
- `Admin` (:168-203): holds `SupplierManage` (:198) AND `SupplierPaymentRecord` (:202).
- `Vendeur` (:121) and `Caissier` (:122-136): hold NO supplier-related permission.
Roles holding a supplier permission OTHER than `SupplierManage`: Gerant and Admin
hold `SupplierPaymentRecord` in addition — but NO role holds `SupplierPaymentRecord`
(or any supplier permission) WITHOUT also holding `SupplierManage`. No role can
pass the search gate while failing the list gate.

## CONCLUSION
NO ASYMMETRY — `suppliers_list`'s `SupplierManage` requirement exactly matches the permission already required by the existing supplier search path; every role that can search suppliers today can already list them. Safe to wire in TASK-DSN-054-C2 without further decision.

## 7. `git status --porcelain` (this task added/modified nothing)
Verbosity note: the tree is NOT empty — it carries the already-accepted,
uncommitted deliverables of prior tasks (TASK-DSN-054/-C1, AUD-015, DSN-055,
DSN-056, BRK-057). This audit created zero source modifications; the only
`audit/` entry attributable to this task is its own required deliverable,
`audit/AUD-058/REPORT.md`. Verbatim output at task end:
```
M crates/application/src/ports.rs
M crates/application/src/use_cases/suppliers.rs
M crates/commands/src/ipc.rs
M crates/infrastructure/src/adapters/sqlite_supplier_repository.rs
M crates/infrastructure/tests/supplier_purchase_lifecycle_tests.rs
M docs/design/glossary-ar-fr.md
M src-tauri/src/lib.rs
M src/features/customers/CustomerDossierPane.tsx
M src/features/customers/CustomersModule.tsx
M src/features/reports/ReportsHomeScreen.tsx
M src/features/suppliers/SupplierListScreen.tsx
M src/index.css
M src/lib/i18n.ts
M src/lib/ipc.ts
M tailwind.config.js
?? audit/AUD-015/
?? audit/AUD-058/
?? audit/BRK-057/
?? audit/DSN-054/
?? audit/DSN-055/
?? audit/DSN-056/
?? docs/adr/094-dark-mode-semantic-design-tokens.md
?? docs/adr/095-ink-muted-token-tier-refinement.md
?? docs/design/design-tokens.md
?? docs/design/evidence/
?? src/features/customers/customer-empty-panel.test.ts
?? src/features/suppliers/supplier-empty-states.test.ts
```
No line of any tracked file was touched by this task (`git diff` for all
tracked paths is attributable entirely to the prior tasks listed above).
