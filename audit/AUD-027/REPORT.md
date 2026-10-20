# TASK-AUD-027 — Exploratory Audit: User Management, Authentication Commands, Role Gating

- Task: TASK-AUD-027 (read-only audit)
- Date: 2026-10-19
- Workspace: D:/Dev/Baraka UltraPOS, branch master
- Output: `audit/AUD-027/REPORT.md` (this file — the ONLY file created)

## 0. Headline finding

The task CONTEXT premise is **stale**. It states that "full user management (creating users,
editing roles, deactivation, password/PIN reset, self-protection of last admin) is unmapped" and
Charter v2.0 §3.4 likewise records "user management (NO engine command exists)".

**Reality: the full engine + IPC + UI stack for user management already exists** (apparently built
as BRK-064 / DSN-030, which post-date the Charter text):

- 4 RBAC-gated use cases in `crates/application/src/use_cases/users.rs` (List, Create, Update,
  ResetCredentials), all requiring `Permission::UserManage`, with last-active-Admin protection.
- 4 registered Tauri commands (`users_list`, `users_create`, `users_update`,
  `users_reset_credentials`) + 4 typed IPC wrappers + a complete Admin-only UI module
  (`src/features/users/`, 5 files) mounted behind an ADMIN nav gate in `src/App.tsx`.
- Dedicated integration tests (`crates/infrastructure/tests/users_management.rs`, 8 tests) and
  Vitest coverage (`src/features/users/users-module.test.ts`).

Gaps that genuinely remain (Charter §11 items still open):
1. `users.role` is free TEXT with **no SQL CHECK** and **no trigger** — enforcement is
   application-level only (Charter §11 explicitly leaves this as "migration adds CHECK or
   application-level enforcement decision via ADR" — the ADR decision is not on record).
2. `SessionInfo` / `LoginResponseDto` carries **only the role code**, not the effective permission
   set (Charter §11 item 2 still open; Charter §3.3 engineering debt #3 confirms).
3. No self-service password change or self-demotion guard: an Admin can demote/deactivate their
   *own* account as long as ≥1 other active Admin exists (only the *last*-admin case is blocked);
   no "cannot deactivate yourself while logged in" rule exists.

---

## 1. Database schema & persistence

### 1.1 `users` table — migration 0001, lines 21–30

`crates/infrastructure/migrations/0001_baseline_schema.sql:21-30`:

```sql
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    pin_hash TEXT,
    role TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;
```

### 1.2 Migration 0002 — lockout columns (full file, 5 lines)

`crates/infrastructure/migrations/0002_auth_lockout.sql:4-5`:

```sql
ALTER TABLE users ADD COLUMN failed_login_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN locked_until TEXT;
```

### 1.3 No other migration touches `users`

- Migrations present: 0001–0015 (`crates/infrastructure/migrations/`, 15 files; 0003 stock engine,
  0004 sales engine, 0005 customer indexes, 0006 print worker, 0007 document language, 0008 cash
  session, 0009 supplier/purchase, 0010 reporting indexes, 0011 converted-document status,
  0012 default-warehouse seed, 0013 refund-out movement, 0014 print_jobs nullable, 0015 pricing
  tiers). None of 0003–0015 alters `users`.
- `users` is referenced as FK target (`REFERENCES users(id)`) from: `stock_movements.created_by`
  (0001:108), `customers.created_by` (0001:139), `customers` audit columns, `documents.created_by`,
  `cash_sessions.opened_by/closed_by` (0008:23,26), `cash_movements.created_by` (0008:56),
  supplier/purchase tables (0009:54,75), `stock_movements` refund-out (0013:37). FKs are
  unenforced-by-default unless `PRAGMA foreign_keys=ON` (set at runtime per AGENTS.md §3).

### 1.4 Answers to the technical questions

| Question | Answer |
|---|---|
| `users.role` CHECK constraint? | **No.** Free TEXT. No `CHECK` on the column, no users trigger in 0001 (triggers present: `prevent_*_update/delete` on ledgers, movements, payments, audit, command log, finalized documents — 0001:345-449 — none on `users`), no users index beyond `UNIQUE(username)` (implicit) + PRIMARY KEY. No partial or extra index on `users` in any migration. |
| Password storage | Argon2id PHC string in `password_hash TEXT NOT NULL`. Hashing: `crates/infrastructure/src/adapters/argon2_hasher.rs` (34 lines, `Argon2::default()` = OWASP params, fresh `SaltString` per hash, `verify` via `PasswordHash::new`). Same hasher reused for PINs (independent secret slot). |
| PIN field | Yes: `pin_hash TEXT` (nullable = no PIN). Verified via `get_pin_hash` returning `Option<String>` (`sqlite_user_repository.rs:170-184`); managed view exposes only `has_pin: bool` (`ports.rs:124`), never the hash. |
| Active status | `is_active INTEGER NOT NULL DEFAULT 1`; 0 = deactivated. No `deleted_at`, no soft-delete tombstone — deactivation is a flag update (`sqlite_user_repository.rs:316-339`, `UPDATE users SET role/is_active/updated_at`). Inactive accounts cannot authenticate (`mod.rs:158-160` returns `AccountInactive`). |
| Triggers on `users` | **None.** Confirmed by grep of 0001 trigger section (0001:345+) and all migrations. |

---

## 2. Domain & application models (Rust)

### 2.1 `crates/domain/src/user.rs` (157 lines)

- `UserId(Uuid)` newtype: `user.rs:13-47` (`new()`, `as_uuid()`, `Display`, `From<Uuid>`, `Default`).
- `Username` validated newtype (trimmed, non-empty): `user.rs:48-70`.
- `User` entity = domain view of `users` (no hashes — hashes travel only through the repository
  port): `user.rs:71-86` — fields `id`, `username`, `role: Role`, `is_active: bool`,
  `failed_login_attempts: u8`, `locked_until: Option<DateTime<Utc>>`.
- `LockoutState` enum (`Clear` / `TrackingFailures(u8)` / `Locked{until}`) + `from_parts` /
  `is_locked`: `user.rs:88-118`.
- Unit tests: `username_trims_and_rejects_empty`, `lockout_state_transitions` (`user.rs:136-156`).

### 2.2 `crates/domain/src/role.rs` (380 lines)

- `Role` enum, serde `SCREAMING_SNAKE_CASE`: `role.rs:11-22` — exactly 4 variants:
  `Vendeur` (counter sales) → `Caissier` (+ cash/sessions) → `Gerant` (+ overrides/returns/stock/
  purchases/reports) → `Admin` (+ users/settings/backup). Strictly hierarchical, additive.
- `Permission` enum, 34 variants: `role.rs:24-80` (`PosSell`, `PosParkResume`,
  `CashierCollectPayment`, `CashSessionOpen/Close`, `PriceOverride`, `DiscountOverride`,
  `NegativeStockOverride`, `CreditLimitOverride`, `ProcessReturn`, `StockAdjustment`,
  `PurchaseManage`, `ReportsView`, `UserManage`, `SystemSettingsManage`, `BackupRestoreManage`,
  `ViewStock`, `AdjustStock`, `TransferStock`, `ManageWarehouses`, `CreateSale`, `ViewCustomers`,
  `CreateCustomer`, `ManageCustomerCredit`, `RecordCreditPayment`, `ReprintDocument`,
  `ManagePrinterSettings`, `CashExpenseRecord`, `CashReportView`, `SupplierManage`,
  `PurchaseOrderCreate`, `PurchaseReceive`, `PurchaseInvoiceRecord`, `SupplierPaymentRecord`).
- `Role::code()` / `Role::from_code()` wire mapping VENDEUR/CAISSIER/GERANT/ADMIN: `role.rs:82-105`.
- `Role::permissions()` additive matrix: `role.rs:107-205` — Vendeur 5, Caissier 13, Gerant 29,
  Admin 34 (all). Only Admin holds `UserManage`, `SystemSettingsManage`, `BackupRestoreManage`,
  `ManageWarehouses`, `ManagePrinterSettings`.
- `has_permission()`: `role.rs:207-211`.
- Tests: `matrix_matches_constitution_for_all_roles_and_permissions` (4×34=136 assertions),
  `role_codes_round_trip` (`role.rs:255-380`).

### 2.3 Session context — `crates/application/src/ports.rs`

- `SessionContext { token: SessionToken, user_id: UserId, role: Role, expires_at: DateTime<Utc> }`:
  `ports.rs:194-206`. **Role snapshot at login time** (no live re-read, no permission set).
- `SessionStore` trait (volatile only): `ports.rs:207-217` — `create_session`,
  `validate_session` (None on unknown/expired), `invalidate_session` (silent no-op).
  Implementation: `InMemorySessionStore` (`crates/infrastructure/src/adapters/in_memory_session_store.rs`,
  71 lines; `Mutex<HashMap>`, TTL enforced on validate, expired entries evicted on sight).
- `SESSION_TTL_SECONDS = 43_200` (12 h, fixed, no sliding refresh): `use_cases/mod.rs:33-34`.
- `MAX_LOGIN_ATTEMPTS = 5`, `LOCKOUT_SECONDS = 300` (5 min): `use_cases/mod.rs:36-40`.
- `UserRecord` (managed view, never carries hashes): `ports.rs:112-129`
  (`id`, `username`, `role`, `is_active`, `has_pin`, `created_at`, `updated_at`).
- `UserRepository` trait (11 methods): `ports.rs:50-110` — `find_by_username`,
  `record_login_failure`, `reset_login_failures`, `get_password_hash`, `get_pin_hash`,
  `count_users` (first-run probe), `bootstrap_first_admin` (BEGIN IMMEDIATE + in-txn emptiness
  re-check, audit row in same txn), `list_all_users`, `find_user_by_id`, `create_user`,
  `update_user`, `update_credentials`, `count_active_admins`.

### 2.4 Password hashing

- Port `PasswordHasher { hash, verify }`: `ports.rs:41-48` (comment: "Argon2id in infrastructure;
  fakes in tests").
- Production: `Argon2PasswordHasher` (`argon2_hasher.rs`, 34 lines) — `Argon2::default()` OWASP
  params, `SaltString::generate(OsRng)`, PHC string storage; failures map to static
  non-leaking `ApplicationError::Port` strings.
- SQLite repository maps every storage failure to static `"user store failure"` (no SQL text leaks
  to auth callers): `sqlite_user_repository.rs:29-33`.

---

## 3. Application use cases & engine capabilities

All in `crates/application/src/use_cases/` — auth in `mod.rs`, management in `users.rs`.

| Operation | Status | Location |
|---|---|---|
| Login (password, lockout, inactive-check) | ✅ EXISTS | `AuthenticateUserUseCase`, `mod.rs:101-198` |
| PIN verify (fast user-switch, no lockout) | ✅ EXISTS | `VerifyPinUseCase`, `mod.rs:200-234` |
| Logout (idempotent revoke) | ✅ EXISTS | `LogoutUseCase`, `mod.rs:236-258` |
| Bootstrap first Admin (empty-table atomic) | ✅ EXISTS | `BootstrapFirstAdminUseCase`, `mod.rs:289-343` |
| Create user (validate+hash+insert) | ✅ EXISTS | `CreateUserUseCase`, `users.rs:100-156` |
| Update user role / active flag | ✅ EXISTS | `UpdateUserUseCase`, `users.rs:158-211` |
| Deactivate / activate user | ✅ EXISTS (via `is_active` patch) | same as above |
| Change / reset password+PIN | ✅ EXISTS | `ResetUserCredentialsUseCase`, `users.rs:213-272` |
| List all users | ✅ EXISTS | `ListUsersUseCase`, `users.rs:76-98` |
| Last-active-Admin protection | ✅ EXISTS | `users.rs:195-203` (`count_active_admins() <= 1` → `Conflict(LAST_ADMIN_PROTECTED)` on demote *or* deactivate of an active Admin) |

Validation rules (`users.rs:42-74`): username 3–30 chars ASCII alnum+`_`; password ≥ 6 chars;
PIN 4–6 ASCII digits. Wire codes: `INVALID_USERNAME`, `WEAK_PASSWORD`, `INVALID_PIN`,
`USERNAME_TAKEN`, `LAST_ADMIN_PROTECTED`, `NOTHING_TO_RESET` (`users.rs:18-29`). All four
management use cases gate on `Permission::UserManage` (`require_user_manage`, `users.rs:35-40`;
Admin-only in practice). Login algorithm: unknown-user ≡ wrong-password (`InvalidCredentials`,
`mod.rs:150-157,171-184`); locked accounts short-circuit without touching the verifier
(`mod.rs:162-169`); threshold-reaching attempt locks (`mod.rs:172-180`); success resets counter
(`mod.rs:186`).

**Missing / partial (engine):**
- No username *rename* path (update covers role/active only).
- No self-deactivation guard (an Admin may deactivate their own session's account if another active
  Admin exists; the session token itself stays valid until expiry — `SessionContext` is a login-time
  snapshot, role changes do not revoke live sessions).
- No credential *clear* (PIN removal): `update_credentials` uses `COALESCE(?2, pin_hash)` —
  `None` keeps the slot (`sqlite_user_repository.rs:341-370`); there is no path to NULL out a PIN.
- `VerifyPinUseCase` has **no IPC command** (dead engine surface — no caller in `ipc.rs`).

---

## 4. IPC contract & Tauri registration

### 4.1 Backend — `crates/commands/src/ipc.rs` (3967 lines), registered in `src-tauri/src/lib.rs:206-214`

| Command | Signature (ipc.rs) | Notes |
|---|---|---|
| `auth_login` | `auth_login(state, request: LoginRequestDto) -> Result<LoginResponseDto, String>` (`ipc.rs:602-625`) | username+password; lockout/inactive mapping |
| `auth_logout` | `auth_logout(state, request: LogoutRequestDto) -> Result<(), String>` (`ipc.rs:626-638`) | idempotent |
| `check_first_run_status` | `check_first_run_status(state) -> Result<CheckFirstRunStatusResponse, String>` (`ipc.rs:640-654`) | unauthenticated, side-effect-free `COUNT(*)` |
| `bootstrap_first_admin` | `bootstrap_first_admin(state, request: BootstrapFirstAdminRequest) -> Result<BootstrapFirstAdminResponse, String>` (`ipc.rs:656-688`) | unauthenticated; `terminal_id` from TerminalContext state (ADR-045), never client input; no session minted |
| `users_list` | `users_list(state, session_token: String) -> Result<Vec<UserSummaryDto>, String>` (`ipc.rs:690-701`) | UserManage-gated |
| `users_create` | `users_create(state, request: CreateUserRequestDto) -> Result<UserSummaryDto, String>` (`ipc.rs:703-727`) | role parsed via `Role::from_code` at edge |
| `users_update` | `users_update(state, request: UpdateUserRequestDto) -> Result<UserSummaryDto, String>` (`ipc.rs:728-750`) | UUID-parsed `user_id`; optional role/active |
| `users_reset_credentials` | `users_reset_credentials(state, request: ResetUserCredentialsRequestDto) -> Result<(), String>` (`ipc.rs:752-773`) | returns unit |

- Session enforcement: `require_session` → `validate_session`, `None` ⇒ `"UNAUTHORIZED"`
  (`ipc.rs:419-427`).
- Error-code mapping (`error_code`, `ipc.rs:213-240+`): `INVALID_CREDENTIALS`, `ACCOUNT_INACTIVE`,
  `ACCOUNT_LOCKED:{remaining_seconds}`, `PIN_NOT_SET`, `BOOTSTRAP_ALREADY_COMPLETED`,
  `UNAUTHORIZED` (`ipc.rs:378`), plus pass-through of `USERNAME_TAKEN`, `LAST_ADMIN_PROTECTED`,
  `INVALID_USERNAME`, `WEAK_PASSWORD`, `INVALID_PIN`, `NOTHING_TO_RESET` via `Validation(...)`
  display. `LoginRequestDto`/`LoginResponseDto` use redacting `Debug` impls (`dto.rs:60-96`).

### 4.2 DTOs — `crates/commands/src/dto.rs` (2507 lines)

- `LoginRequestDto { username, password }` (`dto.rs:48-70`, redacted Debug).
- `LoginResponseDto { session_token, user_id, username, role, expires_at }` — **all plain strings**
  (`dto.rs:73-96`). **Carries the role code only — NOT the permission set.** Charter §11 item
  "SessionInfo carries the effective permission set" is **not implemented** at any layer.
- `LogoutRequestDto { session_token }` (`dto.rs:98+`); `CheckFirstRunStatusResponse
  { requires_setup }` (`dto.rs:121-129`); `BootstrapFirstAdminRequest { command_id, username,
  password, ... }` (`dto.rs:131-152`); `BootstrapFirstAdminResponse { user_id, username }`
  (`dto.rs:153+`).
- `UserSummaryDto { id, username, role, is_active, has_pin, created_at, updated_at }`
  (`dto.rs:2336-2353`) — mirrors `UserRecord`, never carries hashes.
- `CreateUserRequestDto { session_token, username, password, pin: Option, role }`
  (`dto.rs:2355-2368`); `UpdateUserRequestDto { session_token, user_id, role: Option,
  is_active: Option }` (`dto.rs:2370-2381`); `ResetUserCredentialsRequestDto { session_token,
  user_id, password: Option, pin: Option }` (`dto.rs:2383-2395`).

### 4.3 Frontend bridge — `src/lib/ipc.ts` (2698 lines)

- `PRE_AUTH_COMMANDS = app_health, auth_login, check_first_run_status, bootstrap_first_admin`
  (`ipc.ts:58-63`); every other command gets the store-owned token auto-injected at top level AND
  inside nested `request` objects (`withSessionToken`, `ipc.ts:76-97`); missing session ⇒
  `IPC_NO_ACTIVE_SESSION` fail-fast.
- `ipcLogin → SessionInfo` mapping (`ipc.ts:100-111`); `ipcLogout` (`ipc.ts:114-116`);
  `ipcCheckFirstRunStatus` (`ipc.ts:119-122`); `ipcBootstrapFirstAdmin`
  (`ipc.ts:128-139`, caller logs in afterwards — no session minted).
- `UserInfo { id, username, role: "VENDEUR"|"CAISSIER"|"GERANT"|"ADMIN", is_active, has_pin,
  created_at, updated_at }` (`ipc.ts:490-500`); `ipcListUsers` / `ipcCreateUser` /
  `ipcUpdateUser` / `ipcResetUserCredentials` (`ipc.ts:502-549`). Note: these four take an
  explicit `sessionToken` parameter (pre-date the auto-injection rule; the wrapper also injects,
  so both paths coincide).
- `SessionInfo { token, userId, username, role: string, expiresAt }` (`src/types/pos.ts:121-127`):
  **role name only — no `permissions: Permission[]` field.** Confirmed gap (matches Charter §3.3
  debt #3).

---

## 5. Frontend authentication & role gating

### 5.1 Session store — `src/stores/session-store.ts` (98 lines)

- Zustand store: `session: SessionInfo | null`, warehouses, `authBusy`, `authError`.
- `login()` trims username, stores session, then loads warehouses + cash refresh
  (`session-store.ts:34-55`); failures clear session and surface wire code in `authError`.
- `bootstrap()` creates the first Admin then performs a standard `login()`
  (`session-store.ts:56-69`).
- `logout()` calls `ipcLogout()` (idempotent; errors swallowed) then clears session + warehouse +
  cash (`session-store.ts:70-81`). "Cleared on logout" = in-memory Zustand reset; **no
  localStorage/sessionStorage persistence** — a reload always returns to the login gate.
- `getSessionToken()` is the single source of truth for IPC injection
  (`session-store.ts:96-97`).

### 5.2 Auth screens — `src/features/auth/`

- `LoginScreen.tsx` (91 lines): username+password form → `session-store.login()`; backend code
  surfaced via `translateBackendError` (`LoginScreen.tsx:28-34`).
- `FirstRunSetupScreen.tsx` (109 lines): first-Admin creation → `session-store.bootstrap()`.
- Unauthenticated gate: `UnauthenticatedGate` in `src/App.tsx:86-113` probes
  `ipcCheckFirstRunStatus()` once; empty `users` table ⇒ setup screen, else login; probe failure
  (e.g. browser preview) falls back to login.

### 5.3 Role gating (UI = convenience only; Rust use cases are the authority)

- Nav: `role === "ADMIN" && entry("users", …)` (`src/App.tsx:74`); `isManagerRole =
  GERANT||ADMIN` (`App.tsx:29-31`); `isAdmin` (`App.tsx:123`).
- All gating is **string comparison on `session.role`** — no frontend permission table exists
  (consistent with the missing permission set in `SessionInfo`).
- Other gates (same pattern): `CustomerSelectModal.tsx:28,32`, `RecordPaymentModal.tsx:57,62`,
  `stock-gate.ts:15`, `StoreIdentityModal.tsx:68`, `OperationsMenu.tsx:40-41`,
  `CatalogModule.tsx:247`, `CustomerFormModal.tsx:31`, `CustomersModule.tsx:26`,
  `DocumentsModule.tsx:21`, `SettingsModule.tsx:157`, `StockModule.tsx:36,60`.
- Authorization is enforced server-side per use case (`require_user_manage` for users;
  `require_session` + use-case permission checks elsewhere); UI gates only hide affordances.

### 5.4 Users UI — `src/features/users/` (exists, Admin-only)

- `UsersModule.tsx` (145 lines): staff directory + HUD (`total/admins/cashAndSales/inactive` via
  `computeUserMetrics`), list via `ipcListUsers`, create/edit dialog, credential-reset dialog;
  "Mounted strictly behind the ADMIN gate in App.tsx" (file header).
- `UserListTable.tsx` (116), `UserFormModal.tsx` (204, injected `createUser`/`updateUser` bindings),
  `CredentialResetModal.tsx` (124), `user-helpers.ts` (137: `roleOptions`, `isRoleCode`,
  `roleLabel` FR/AR, `roleBadgeClass`, `statusBadgeClass`, `validateUsername/Password/Pin`
  mirroring engine rules, `computeUserMetrics`, `userErrorMessage` for the 6 wire codes).
- No other route/screen lists users; POS shell exposes nothing user-related to non-Admins.

---

## 6. Test coverage

### 6.1 Rust — application unit (`crates/application/src/use_cases/mod.rs:345-974`)

`login_success_returns_valid_session`, `unknown_user_and_wrong_password_are_identical`,
`five_failures_lock_and_sixth_skips_verifier` (verifier call-counted),
`lock_expiry_then_success_resets_counter`, `pin_verification_paths`,
`logout_is_idempotent_and_session_expiry_enforced`, `bootstrap_creates_single_admin_and_login_succeeds`,
`bootstrap_second_attempt_is_rejected_without_side_effects`,
`bootstrap_rejects_empty_username_and_empty_password`, `totals_subtract_discount`.

### 6.2 Rust — domain unit

`user.rs:136-156` (`username_trims_and_rejects_empty`, `lockout_state_transitions`);
`role.rs:255-380` (136-assertion permission matrix + `role_codes_round_trip`).

### 6.3 Rust — infrastructure integration

- `crates/infrastructure/tests/auth.rs` (7 tests): `argon2_round_trip_and_wrong_password_is_false`
  (:87), `login_success_returns_matching_session` (:96),
  `five_failures_lock_and_sixth_correct_is_rejected` (:114),
  `pin_paths_cover_set_unset_and_wrong` (:149), `logout_is_idempotent_and_revokes` (:170),
  `expired_session_validates_to_none` (:184), `inactive_account_is_rejected` (:193).
- `crates/infrastructure/tests/users_management.rs` (8 tests): `admin_lists_seeded_operators`
  (:92), `admin_creates_caissier_with_pin` (:105), `duplicate_username_conflicts` (:128),
  `vendeur_cannot_list_or_create` (:143), `last_admin_deactivation_is_rejected` (:162),
  `last_admin_demotion_is_rejected` (:172), `second_admin_frees_the_first` (:182),
  `admin_resets_password_and_pin` (:196).
- `crates/infrastructure/tests/first_run_bootstrap.rs` (6 tests):
  `fresh_database_requires_setup_with_zero_users` (:81),
  `bootstrap_creates_exactly_one_active_admin_with_argon2id_hash` (:88),
  `bootstrap_writes_first_admin_audit_event_with_terminal_origin` (:122),
  `second_bootstrap_is_rejected_with_zero_extra_rows` (:158),
  `concurrent_bootstrap_race_yields_exactly_one_admin` (:175),
  `database_path_resolves_under_app_data_dir_not_cwd` (:220) (+ dir-idempotency :237).

### 6.4 Vitest

- `src/features/users/users-module.test.ts` (267 lines, 6 describes): nav gating strict-Admin
  (extends AppView, ADMIN-only entry, unauthorized card, no GERANT/CAISSIER/VENDEUR mention);
  IPC bindings (four BRK-064 wrappers ↔ backend commands); module dispatch via injected bindings;
  FR/AR i18n namespace; role badges/labels; HUD metric calculations.
- Auth-adjacent: `src/features/settings/settings-module.test.ts:75` (ADMIN gate assertion),
  `src/features/stock/stock-module.test.ts:95` (ADMIN gate assertion).
- No Vitest file exercises login/logout/first-run flows (no session-store test exists;
  `src/stores/__tests__/` holds only cart-store; no login E2E in Vitest scope).

---

## 7. Gap matrix — Charter §11 vs codebase reality

Charter §11 bullet 1: *"Users management: create, edit, deactivate, reset password/PIN, list;
role validation (migration adds CHECK or application-level enforcement decision via ADR);
Admin-only; audit; protection against deactivating the last active Admin."*
Charter §11 bullet 2: *"SessionInfo carries the effective permission set so the UI gates
precisely."*

| Charter §11 requirement | Reality | Verdict |
|---|---|---|
| create user | `CreateUserUseCase` + `users_create` + `UserFormModal` + tests | DONE (unrecorded in Charter) |
| edit user (role) | `UpdateUserUseCase` + `users_update` + modal + tests | DONE (unrecorded) |
| deactivate / activate | `is_active` patch + status pill + tests | DONE (unrecorded) |
| reset password / PIN | `ResetUserCredentialsUseCase` + `users_reset_credentials` + `CredentialResetModal` + test | DONE (unrecorded) |
| list users | `ListUsersUseCase` + `users_list` + `UsersModule` + test | DONE (unrecorded) |
| role validation (CHECK vs app-level ADR) | Free TEXT, app-level `Role::from_code` at IPC edge (`ipc.rs:709-710,737-742`) and repository row-mapping (`sqlite_user_repository.rs:54-56,86-88`); **no CHECK, no ADR on record** | OPEN (decision undocumented) |
| Admin-only | `require_user_manage` in all 4 use cases + ADMIN nav gate + `vendeur_cannot_list_or_create` test | DONE |
| audit event for user admin actions | Bootstrap writes `FIRST_ADMIN_BOOTSTRAP` audit row (`sqlite_user_repository.rs:226-240`); **create/update/reset write NO audit row** | PARTIAL |
| last-active-Admin protection | Demote + deactivate blocked at `count_active_admins() <= 1`, tested both ways | DONE |
| SessionInfo carries permission set | Only role code at every layer (`dto.rs:73-96`, `pos.ts:121-127`) | OPEN |
| Users UI behind Admin gate | `UsersModule` + `App.tsx:74` + Vitest gate tests | DONE (unrecorded) |

Recommended follow-ups (not implemented by this task — read-only): (a) ADR recording the
CHECK-vs-application role-validation decision; (b) audit rows for create/update/reset-credentials;
(c) permission set in `SessionInfo` per §11 bullet 2; (d) PIN-clear path and self-deactivation
guard if the owner wants them; (e) wire `VerifyPinUseCase` to IPC or remove it. Also: Charter
§3.3/§3.4 statements "user management (NO engine command exists)" are now factually wrong and
should be corrected wherever the Charter/Copilot context is re-issued.

---

## 8. Quality gates (verbatim)

> Executed 2026-10-19 AFTER writing this report (docs-only change; cannot affect code gates).
> `CARGO_BUILD_JOBS=2` on all cargo commands per AGENTS.md §2. Runner note: this machine's
> exit-code/stdout capture is unreliable ("stdin is not a tty" false failures); the outputs below
> are the real tool stdout. Full per-suite lines for G3 were aggregated with grep over the real
> output (all suites `ok`, zero failures); the tail-40 window is pasted in §8.3.

### G1 `cargo fmt --check`

```
(empty output — clean)
EXIT:0
```

### G2 `cargo clippy --all-targets --all-features -- -D warnings`

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.99s
EXIT:0
```

### G3 `cargo test --workspace -j 2`

Tail window (verbatim):

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\window_config.rs (target\debug\deps\window_config-14724b51f72a69ff.exe)

running 2 tests
test main_window_geometry_matches_adr066 ... ok
test main_window_maximized_policy_brk060 ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests baraka_application

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests baraka_commands

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests baraka_domain

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests baraka_infrastructure

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests ultrapos_lib

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

EXIT:0
```

Aggregated over the full output: every suite `test result: ok`, **0 failed in all suites**
(includes the 65-test suite, auth/users_management/first_run_bootstrap integration suites;
representative passing lines observed: `test main_window_geometry_matches_adr066 ... ok`,
`test test_print_worker_exhausted_retries_transitions_to_failed ... ok`).

### G4 `npm run lint`

```
npm notice run baraka-ultrapos@1.0.0 lint
npm notice run eslint . --ext ts,tsx --report-unused-disable-directives --max-warnings 0
EXIT:0
```

### G5 `npm run typecheck`

```
npm notice run baraka-ultrapos@1.0.0 typecheck
npm notice run tsc --noEmit
EXIT:0
```

### G6 `npm run build`

```
npm notice run baraka-ultrapos@1.0.0 build
npm notice run tsc && vite build
vite v5.4.21 building for production...
transforming...
✓ 168 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                              0.51 kB │ gzip:   0.32 kB
dist/assets/baraka-logo-icon-CFLRGFSQ.png   31.80 kB
dist/assets/baraka-logo-CUnvKflC.png       323.86 kB
dist/assets/index-dGamyoW5.css              45.24 kB │ gzip:   8.09 kB
dist/assets/index-DxamEfD2.js              724.70 kB │ gzip: 186.45 kB
✓ built in 14.68s
EXIT:0
```

### G7 `npx vitest run`

Tail window (verbatim):

```
✓ src/components/pos/family2-contrast.test.ts (5 tests) 6ms
 ✓ src/hooks/use-pos-keyboard.test.ts (5 tests) 6ms
 ✓ src/features/settings/fiscal-settings-tab.test.ts (3 tests) 7ms
 ✓ src/features/suppliers/supplier-instant-search.test.ts (3 tests) 5ms
 ✓ src/components/pos/modals/transfer-instant-search.test.ts (3 tests) 4ms
 ✓ src/stores/pos-ui-store.test.ts (3 tests) 5ms
 ✓ src/components/pos/modals/return-instant-search.test.ts (4 tests) 5ms
 ✓ src/features/purchases/purchase-instant-search.test.ts (3 tests) 4ms
 ✓ src/features/pos/quick-palette-wiring.test.ts (3 tests) 6ms

 Test Files  68 passed (68)
      Tests  639 passed (639)
   Start at  14:32:12
   Duration  15.00s (transform 1.91s, setup 0ms, collect 6.70s, tests 1.84s, environment 22ms, prepare 15.08s)

EXIT:0
```

---

## 9. Report metadata

- Git status before: clean (`git status --porcelain` empty; branch master).
- Git status after: exactly one untracked path `audit/AUD-027/REPORT.md`; zero modified files
  (verified via `git status --porcelain` after execution).
- Line endings: migration and source files verified LF-only (`grep -c $'\r'` = 0 on sampled
  files); this report written LF-only; `.gitattributes` (`* -text`) untouched.
- Live data: never touched (`%APPDATA%/com.baraka.ultrapos` not accessed).
- Method: read-only inspection via file reads + `grep`; no Rust/TS/config file modified.
