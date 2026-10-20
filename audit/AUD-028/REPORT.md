# TASK-AUD-028 — Exploratory Audit: Users Management UI, Navigation Integration, Frontend Role Gating

- Task: TASK-AUD-028 (read-only audit)
- Date: 2026-10-19
- Workspace: D:/Dev/Baraka UltraPOS, branch master
- Baseline: worktree carries uncommitted TASK-BRK-070 changes (permission set in contract + store);
  this audit reads that state. Output: `audit/AUD-028/REPORT.md` (this file — the ONLY file created)

## 0. Headline findings

1. The Users module (`src/features/users/`, 6 files / 877 lines incl. tests) is complete and
   Admin-gated, but uses **zero semantic design tokens** (no `bg-surface-*` / `text-ink-*` /
   `border-line-*` anywhere in the directory) — 100% raw `slate`/`red`/`amber`/`violet`/`sky`/
   `emerald` + `accent-500` classes, all with paired `dark:` variants. Functional in dark mode,
   but bypasses the ADR-094/095 token system.
2. `hasPermission()` (ADR-100) has **zero UI call sites** — the entire frontend still gates on
   ~25 raw `role === "ADMIN"/"GERANT"/…` string comparisons. Full inventory with engine-verified
   target permissions is in §4.
3. Two UI/engine mismatches found: CatalogModule and CustomersModule gate browsing on
   GERANT/ADMIN while the engine requires only `PosSell` (catalog search, catalog.rs:417,459,644)
   and `ViewCustomers` (customers.rs:250,283,313) — Caissier/Vendeur are blocked in UI from
   actions the engine would authorize (notably `customers_create` needs only `CreateCustomer`,
   which Caissier holds, yet CustomerFormModal requires GERANT/ADMIN).
4. One RTL gap: `CredentialResetModal.tsx:66` interpolates `target.username` into the title
   without a `<bdi dir="ltr">` guard (every other username/date rendering in the module has one).
5. Tests assert on source text, never render components: 10 Vitest describes, zero DOM mounts.

## 1. Component architecture (`src/features/users/`)

| File | Lines | Exported symbols |
|---|---|---|
| `UsersModule.tsx` | 145 | `UsersModule` |
| `UserListTable.tsx` | 116 | `UserListTable` |
| `UserFormModal.tsx` | 204 | `UserFormModal` |
| `CredentialResetModal.tsx` | 124 | `CredentialResetModal` |
| `user-helpers.ts` | 137 | `ManagedRole`, `roleOptions`, `isRoleCode`, `roleLabel`, `roleBadgeClass`, `statusBadgeClass`, `validateUsername`, `validatePassword`, `validatePin`, `UserMetrics`, `computeUserMetrics`, `userErrorMessage` |
| `users-module.test.ts` | 267 | tests only |

Component tree: `UsersModule` (HUD cards + list + dialog switch) → `UserListTable` (pure view,
callbacks lift out) + `UserFormModal` (create/edit) + `CredentialResetModal` (reset). All three
children are pure props-driven views; modals render through `ModalShell`
(`src/components/pos/modals/ModalShell.tsx:11`, `fixed inset-0 z-50` — permitted modal-overlay
usage).

State management: **local `useState` only** (`UsersModule.tsx:28-31` — `users`, `loading`,
`loadError`, `dialog` discriminated union `:23`); no dedicated Zustand store. Refresh-after-save
pattern (`onSaved` → `setDialog(null)` + `void refresh()`, `:120-123,136-139`).

Create flow (`UserFormModal.tsx:98-162`): fields username (create-only, placeholder `caisse1`,
`:108`) + password (`type="password"`, `:138`) + optional PIN (`type="password"`
`inputMode="numeric"`, `:151-152`) + role select over `roleOptions()` (`:126`, 4 canonical
codes). Client validation (`:56-59`): `usernameOk` / `passwordOk` / `pinOk` via
`validateUsername` (3–30 ASCII alnum+`_`, `user-helpers.ts:70-75`), `validatePassword` (≥6 chars,
`:78-80`), `validatePin` (4–6 digits, `:83-85`); save disabled unless `canSave` (`:59,195`).
**No password-confirmation field.** Edit flow (`:111-175`): username read-only (`:111-115`),
role select, active checkbox (`:164-175`); sparse patch — only changed role/active are sent
(`:78-79`), so saving unchanged opens the engine `NOTHING_TO_RESET` rejection, localized via
`userErrorMessage` (`:84`). No username rename path (engine has none either).

Credential reset (`CredentialResetModal.tsx:33-95`): password and/or PIN, at least one required
(`hasOne`, `:40-41`); both masked + `autoComplete="new-password"` (`:72-91`).

Errors: all IPC failures funnel through `userErrorMessage(String(failure), locale)`
(`UserFormModal.tsx:84`, `CredentialResetModal.tsx:57`, `UsersModule.tsx:42`) covering 6 codes
(`LAST_ADMIN_PROTECTED`, `USERNAME_TAKEN`, `INVALID_PIN`, `WEAK_PASSWORD`,
`INVALID_USERNAME`, `NOTHING_TO_RESET`, `user-helpers.ts:110-136`) with raw-code fallback
(`:134-135`). Success feedback: none — modal closes and the list refreshes silently.

Last-active-Admin protection UX: **entirely reactive**. The engine guard
(`UpdateUserUseCase`, `crates/application/src/use_cases/users.rs:195-203`) surfaces
`LAST_ADMIN_PROTECTED`, localized at `user-helpers.ts:112-115` and shown in the modal error
banner. No proactive UI: no warning when opening the sole active Admin, no disabling of the
demote/deactivate controls.

IPC calls made: `ipcListUsers(session.token)` (`UsersModule.tsx:40`),
`ipcCreateUser` / `ipcUpdateUser` (injected as props, `:124-125`, invoked `:69-80`),
`ipcResetUserCredentials` (injected `:140`, invoked `CredentialResetModal.tsx:50-54`).
Tokens are passed explicitly (predating the wrapper auto-injection; both coincide).

## 2. Design tokens, visual compliance, RTL, i18n, states

Tokens: `grep` for `surface- | text-ink | border-line- | bg-success | text-success | bg-danger |
text-danger | bg-warning` in `src/features/users/` returns **NONE**. The semantic system exists
(`--color-surface-canvas`, `--color-ink-primary/secondary/inverse/muted`,
`--color-line-default` in `src/index.css:97-146`, wired as `surface.*` / `ink.*` / `line.*` in
`tailwind.config.js:29-41`, ADR-094/095) but the Users module predates/ignores it. Raw-palette
census (all with paired `dark:` variants — dark mode is functionally covered, just not
token-driven): `text-slate-600×7`, `text-slate-300×7`, `text-slate-500×6`, `bg-slate-800×5`,
`bg-slate-700×5`, `text-red-700`/`text-red-200`/`bg-red-900`/`bg-red-50` error family,
`bg-amber-200`/`amber-900` credentials buttons, `bg-accent-500` + `text-slate-950` primaries,
role badges violet (ADMIN) / sky (GERANT) / amber (CAISSIER) / emerald (VENDEUR)
(`roleBadgeClass`, `user-helpers.ts:47-60`), status pills green/red (`:63-67`). Hardcoded white
surfaces exist (`bg-white` inputs `:91`, HUD cards `UsersModule.tsx:83`) but each has a
`dark:bg-slate-*` pair — no unpaired light-only surface found.

RTL/Arabic: root sets `dir`/`lang` per locale (`src/App.tsx:131-134`); module uses logical
properties (`text-start` ×6, `UserListTable.tsx:39-55`; `mr-auto`, `justify-end`). All 36
`users.*` i18n keys exist in exactly 2 copies (FR + AR — verified by count; e.g. `users.title`
`src/lib/i18n.ts:167` FR / `:1196` AR). No hardcoded French/Arabic copy in the module except the
neutral `placeholder="caisse1"` (`UserFormModal.tsx:108`); unknown role codes fall back to the
raw code (`roleLabel` default, `user-helpers.ts:38-39`). `<bdi dir="ltr">` guards: username cell
(`UserListTable.tsx:66`), `created_at` (`:88`), edit-mode username (`UserFormModal.tsx:113`).
**Gap:** reset-modal title `` `${t(locale, "users.resetTitle")} ${target.username}` ``
(`CredentialResetModal.tsx:66`) has no bidi isolation.

State catalog: loading text (`users.loading`, `UsersModule.tsx:90-93` — **no skeleton**), error
card (`:94-99`), empty state (`users.empty`, `UserListTable.tsx:25-33`). No success toast/inline
confirmation anywhere.

## 3. Navigation integration & shell

`AppView` includes `"users"` (`src/stores/app-view-store.ts:11`). `NavRail`
(`src/App.tsx:36-76`): `entry()` helper (`:41-51`); users entry rendered **only for
`role === "ADMIN"`** (`:74`); rail itself renders only for `manager` (`:140`), and ADMIN implies
manager (`isManagerRole`, `:28-30`), so the entry is reachable. View switch: `view`/`setView`
from `useAppViewStore` (`:120-121`); module branch chain (`:142-168`):
stock/customers/documents/suppliers/reports/catalog/settings all require `manager`, but
`users` (`:156-165`) has its **own branch**: `isAdmin` (`:123,157`, `session.role === "ADMIN"`)
→ `<UsersModule />` (`:158`), else an unauthorized card with `users.unauthorized`
(`:160-164`). Direct `setView("users")` by a non-Admin lands on the card, never the module —
defense-in-depth behind the engine's `UserManage` gate. `Topbar.tsx:121` renders
`username · ROLE` as an informational badge (not a gate).

## 4. Frontend role-gating inventory → ADR-100 target permissions

Engine facts verified read-only: catalog search/get `PosSell` (`catalog.rs:417,459,644`);
catalog mutate `StockAdjustment` OR `SystemSettingsManage` (`MANAGE_PERMISSIONS`,
`catalog.rs:19-21`, enforced `:206,277,331,539`); customers create `CreateCustomer`
(`customers.rs:87,164`), reads `ViewCustomers` (`:250,283,313`), payment
`RecordCreditPayment` (`:346`), customer delete `ManageCustomerCredit`
(`deletion.rs:117`); warehouses `ManageWarehouses` (`deletion.rs:210,263`); identity save
`SystemSettingsManage` (`printing.rs:159`); reprint `ReprintDocument` (`printing.rs:43`);
restore Admin-only (`backup.rs:257-265`); sales `CreateSale` + overrides
(`PriceOverride×2`, `CreditLimitOverride×4`, `NegativeStockOverride×3` across sales set);
`hasPermission` has **zero UI call sites** (only `session-store.ts:28,104` + its test).

| # | File:line | Current expression | Target permission (engine-verified) |
|---|---|---|---|
| 1 | `App.tsx:29` `isManagerRole` | GERANT\|\|ADMIN | Manager bundle (no single perm; fans out to rows 9–12, 15) |
| 2 | `App.tsx:74` users nav | `role === "ADMIN"` | `UserManage` |
| 3 | `App.tsx:123` `isAdmin` (users branch) | `role === "ADMIN"` | `UserManage` |
| 4 | `OperationsMenu.tsx:40-41` | GERANT\|\|ADMIN → stock section; ADMIN → system section | `TransferStock`+`AdjustStock` / `BackupRestoreManage`+`SystemSettingsManage` |
| 5 | `CustomerSelectModal.tsx:28` `canCreate` | CAISSIER\|\|GERANT\|\|ADMIN | `CreateCustomer` ✔ exact |
| 6 | `CustomerSelectModal.tsx:32` `canManageCredit` | GERANT\|\|ADMIN | `ManageCustomerCredit` ✔ exact |
| 7 | `RecordPaymentModal.tsx:57` `isVendeur` | `role === "VENDEUR"` (inverted block) | NOT `RecordCreditPayment` ✔ exact |
| 8 | `RecordPaymentModal.tsx:62` `canUseOverpayment` | GERANT\|\|ADMIN | `ManageCustomerCredit` ✔ exact |
| 9 | `stock-gate.ts:15` `isPrivilegedStockRole` | ADMIN\|\|GERANT | `NegativeStockOverride` ✔ exact |
| 10 | `CheckoutModal.tsx:171` `evaluateStockGate({role…})` | role string in | `NegativeStockOverride` ✔ exact |
| 11 | `CatalogModule.tsx:247` `isManagerRole` + `:715` `authed` token gate | GERANT\|\|ADMIN | Mutations ✔ (`StockAdjustment` OR `SystemSettingsManage`); **reads over-gated** (engine needs only `PosSell`, held by all roles) |
| 12 | `CustomersModule.tsx:26,66` + `CustomerFormModal.tsx:31` | GERANT\|\|ADMIN | Reads **over-gated** (`ViewCustomers`, all roles); `customers_create` needs only `CreateCustomer` (Caissier holds it) yet UI blocks Caissier — mismatch |
| 13 | `DocumentsModule.tsx:21,39` | GERANT\|\|ADMIN | Reprint needs only `ReprintDocument` (Caissier holds it) — discovery over-gated (search-command gate not probed; marked) |
| 14 | `StockModule.tsx:36,58` manager / `:60` `isAdmin` | GERANT\|\|ADMIN / ADMIN | `ViewStock` et al. (manager); `ManageWarehouses` (admin, warehouses CRUD) |
| 15 | `SettingsModule.tsx:157` `isAdmin` | ADMIN | Per-tab: identity ✔ `SystemSettingsManage`; printer → `ManagePrinterSettings`; categories → catalog `MANAGE_PERMISSIONS` (fine-grained split pending) |
| 16 | `StoreIdentityModal.tsx:68` + `store-identity-store.ts:120` `canManageIdentity` | ADMIN | `SystemSettingsManage` ✔ exact |
| 17 | `backup-modal-store.ts:66` `canRestoreForRole` | ADMIN | Admin-only restore ✔ (`require_restore_admin`) |
| 18 | `warehouse-transfer-store.ts:64` `isAdminRole` / `:68` `isManagerRole` | ADMIN / GERANT\|\|ADMIN | `ManageWarehouses` / `TransferStock` |
| 19 | `user-helpers.ts:98-99` metric filters | `role ===` inside display-count predicates | N/A (not gates; `hasPermission` migration not applicable) |
| 20 | `Topbar.tsx:121` | displays `session.role` | N/A (informational) |

Net: 15 gate sites migrate to `hasPermission(<PERMISSION>)`, 2 are over-gated relative to the
engine (rows 11–13), 3 are non-gates.

## 5. Test coverage

`src/features/users/users-module.test.ts` (267 lines, 10 describes): nav gating strict-Admin
(`:53`), IPC bindings (`:78`), i18n FR/AR symmetric (`:99`), role badges/labels (`:141`),
metric HUD (`:166`), form validation vs engine rules (`:191` — pure `validate*` helpers),
credential-reset modal (`:213` — source-text: 2× `type="password"`, `inputMode="numeric"`,
`hasOne` requirement), create/edit modal (`:225` — masking, trim, `roleOptions()`),
conflict-code localization incl. `LAST_ADMIN_PROTECTED` FR+AR (`:236`), table contract incl.
bidi guards (`:256`). Pass status: from G7 below (69 files / 642 tests, all green — includes
this file). Uncovered: **no component is ever rendered** (all assertions are source-text
contains or pure-helper calls); no test mounts `UsersModule`/`UserListTable`/modals; no
last-admin UX interaction test; no deactivate-sole-admin or PIN-reset round-trip test;
`userErrorMessage` fallback covered (`:251-253`), success path (silent refresh) unasserted.

## 6. Quality gates (verbatim, run after writing this report — docs-only change)

### G1 `cargo fmt --check`
```
(empty output — clean)
G1_EXIT:0
```
### G2 `cargo clippy --all-targets --all-features -- -D warnings`
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.16s
G2_EXIT:0
```
### G3 `cargo test --workspace -j 2`
```
G3 passed=464 failed=0
```
(aggregate of every `test result: ok` line; zero `failed` in any suite)
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
dist/assets/index-dGamyoW5.css              45.24 kB │ gzip:   8.09 kB
dist/assets/index-2lRffrtr.js              724.86 kB │ gzip: 186.49 kB
✓ built in 5.68s
```
### G7 `npx vitest run`
```
Test Files  69 passed (69)
      Tests  642 passed (642)
   Start at  15:09:28
   Duration  15.01s (transform 2.00s, setup 0ms, collect 6.86s, tests 1.95s, environment 22ms, prepare 13.95s)
```

## 7. Report metadata

- Git status before: 26 modified + 2 untracked (TASK-BRK-070 change set) + `?? audit/AUD-027/`
  (prior task). Full `git status --porcelain` before/after is pasted in the executor handoff;
  after execution the only delta is `?? audit/AUD-028/`.
- Verification that only `audit/AUD-028/REPORT.md` was created: `git status --porcelain` shows no
  other new path; zero modifications to `.ts/.tsx/.rs/.css/.json/.sql` (no-op audit — every
  inspection was a read or `grep`).
- Line endings: measured with byte counting (`tr -cd '\r' | wc -c`), NOT the
  `grep -c $'\r'` idiom — that idiom silently returns 0 in this runner (control character is
  mangled before reaching grep; it also invalidates AUD-027 §9's "LF-only" claim: repo files
  `crates/commands/src/dto.rs` and `src/lib/ipc.ts` genuinely contain CRLF regions, preserved
  untouched). This report is LF-only; `.gitattributes` (`* -text`) untouched; migrations
  untouched (checksums safe); live data never accessed.
- Incidental: the earlier `hermes verify` bootstrap ran `pnpm install`, which rewrote 5 lines of
  `pnpm-lock.yaml` (`@tauri-apps/api` specifier sync — package.json already pins `^2.12.4`);
  restored via `git checkout -- pnpm-lock.yaml`, so the worktree carries no trace of it.
