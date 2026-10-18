# TASK-BRK-057 REPORT — `suppliers_list` IPC Capability

## STEP 0 FINDINGS (verbatim, pre-implementation)

1. **Domain trait method** — `crates/application/src/ports.rs:1092`
   (`pub trait CustomerRepository: Send + Sync`, trait at :1048):
   `fn list_customers(&self, active_only: bool) -> Result<Vec<CustomerSummary>, ApplicationError>;`
   (doc: "List customers ordered by name.")
2. **SQLite impl** — `crates/infrastructure/src/adapters/sqlite_customer_repository.rs:387`
   (`impl CustomerRepository for SqliteCustomerRepository`). SQL verbatim:
   `"SELECT id, name, phone, credit_policy, customer_tier FROM customers WHERE (?1 = 0 OR is_active = 1) ORDER BY name"`
   (param `[i64::from(active_only)]`; no LIMIT/OFFSET/pagination).
3. **Use case** — `crates/application/src/use_cases/customers.rs:298-316`
   (`ListCustomersUseCase::execute(&self, session: &SessionContext, active_only: bool) -> Result<Vec<CustomerSummary>, ApplicationError>`,
   `require(session, Permission::ViewCustomers)?` then `list_customers(active_only)`).
4. **Tauri command** — `crates/commands/src/ipc.rs:2684-2697`
   (`pub fn customers_list(state: tauri::State<'_, AppState>, session_token: String) -> Result<SearchCustomersResponse, String>`,
   calls `ListCustomersUseCase::new(state.customers.as_ref()).execute(&session, true)`).
   Registration: `src-tauri/src/lib.rs:255` `baraka_commands::ipc::customers_list,`
   (inside `invoke_handler![...]`, customers block :250-255, suppliers block :274-279).
5. **Frontend binding** — `src/lib/ipc.ts:842-850`:
   `export async function ipcListCustomers(sessionToken: string): Promise<CustomerSummary[]>`
   calling `call<ListWire>("customers_list", { session_token: sessionToken })`;
   errors surface as thrown exceptions from `call()` (`src/lib/ipc.ts:41-46`:
   throws `bridgeError()` off-Tauri, else rejects with the backend `error_code` string).
6. **Callers** — `src/features/customers/CustomersModule.tsx:15` (import),
   `:79` (`trimmed === "" ? ipcListCustomers(token) : ipcSearchCustomers(...)` —
   empty-query debounce branch on mount/query-clear).
7. **suppliers.rs:1-200** — read in full (236 lines total): module doc states
   "all five operations need `SupplierManage`"; structs Create/Update/Search
   (SearchSuppliersUseCase::execute at :162-175 with the `169-172` empty-query
   `Ok(Vec::new())` guard); GetSupplierUseCase at :179-200.
8. **Supplier DTO** — exists, structurally comparable: `SupplierView`
   (`crates/application/src/ports.rs:1603-1628`): id, name, phone?,
   address?, nif?, rc?, notes?, is_active: bool, current_balance_centimes: i64
   (centimes, untouched), origin_device_id, created_at, updated_at.
   Schema read-only from `crates/infrastructure/migrations/0009_supplier_purchase_engine_schema.sql:29-42`
   (`suppliers_new`: same 13 columns incl. `is_active INTEGER NOT NULL DEFAULT 1`);
   customers table (`0001_baseline_schema.sql:113`) likewise has `is_active`.

## BLOCKED — ASYMMETRY FOUND
none found. Mirrorable exactly: no pagination in customers SQL; both tables
have `is_active`; SupplierView exists. One deliberate domain-consistent choice
(not an asymmetry): the new use case requires `Permission::SupplierManage`
(all supplier ops do) rather than `ViewCustomers` — same roles that may search
suppliers may list them.

## FAIL-then-PASS (verbatim)
FAIL (pre-change):
`error[E0432]: unresolved import 'baraka_application::use_cases::suppliers::ListSuppliersUseCase'`
`warning: unused import: 'SupplierRepository'`
`error[E0599]: no method named 'list_suppliers' found for struct 'SqliteSupplierRepository' in the current scope` (×2)
PASS (post-change):
`test list_suppliers_returns_all_in_name_order_respecting_active_flag ... ok`
(`1 passed; 0 failed`; inserts 3 suppliers via `CreateSupplierUseCase`, asserts
repo returns all 3 in name order, active_only excludes the deactivated one,
use case returns the populated list, vendeur gets `Unauthorized`.)

## Per-layer diffs (final signatures)
- Trait `crates/application/src/ports.rs:1724`: `fn list_suppliers(&self, active_only: bool) -> Result<Vec<SupplierView>, ApplicationError>;`
- Repo `sqlite_supplier_repository.rs:242`: same signature; SQL verbatim:
  `"SELECT {SUPPLIER_COLUMNS} FROM suppliers WHERE (?1 = 0 OR is_active = 1) ORDER BY name"`
  (via `format!`, same pattern as `search_suppliers`; `map_supplier` reused).
- Use case `suppliers.rs:179-205`: `ListSuppliersUseCase::execute(&self, session: &SessionContext, active_only: bool) -> Result<Vec<SupplierView>, ApplicationError>` (requires `SupplierManage`); module doc "five" → "six".
- Command `crates/commands/src/ipc.rs:3347`: `pub fn suppliers_list(state: tauri::State<'_, AppState>, session_token: String) -> Result<SearchSuppliersResponseDto, String>` (reuses `SearchSuppliersResponseDto` + `to_supplier_dto`, same as search); registered `src-tauri/src/lib.rs:279` after `suppliers_search`.
- Binding `src/lib/ipc.ts:1927`: `export async function ipcListSuppliers(sessionToken: string): Promise<SupplierView[]>` (maps `SupplierWire[]` via `toSupplier`, same as search). Zero call sites: `grep -rn ipcListSuppliers src/ | grep -v "export async"` → only the definition (verified below).

## Counts
Rust: 462 + **1** = **463 passed, 0 failed** (N=1; single focused test at the
same integration level customers_list is tested — `crates/infrastructure/tests/`,
same TempDir/migrate harness as `customer_management.rs`; no new harness).
Vitest: no new test file — the binding is a pure typed passthrough over the
already-tested `call()`/mock shapes; typecheck gate covers it. Total stays 597/0.

## Scope note — one test assertion updated (declared, not hidden)
`src/features/suppliers/supplier-empty-states.test.ts:92-101` (added by
TASK-DSN-054, still untracked) asserted the ABSENCE of `pub fn suppliers_list`
in `crates/commands/src/ipc.rs` — a pinned assertion of the very gap this task
closes. It failed (1 failed / 596 passed) as soon as the command landed, as
expected. Updated minimally to assert presence instead (same describe block,
same intent: documenting the capability state), title now
"(TASK-DSN-054; gap closed by TASK-BRK-057)". No UI/store file touched.

## CRLF/LF discipline (before → after, `file`)
- ports.rs: UTF-8 → UTF-8 · suppliers.rs: ASCII → ASCII · sqlite_supplier_repository.rs: ASCII → ASCII
- commands ipc.rs: UTF-8 → UTF-8 · src-tauri lib.rs: UTF-8 → UTF-8
- src/lib/ipc.ts: **CRLF preserved** (before and after: "with CRLF line terminators"; diff +11/-0)
- supplier_purchase_lifecycle_tests.rs: unchanged convention (LF heredoc append on LF file)
