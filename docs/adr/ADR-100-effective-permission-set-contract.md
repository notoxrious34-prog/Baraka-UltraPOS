# ADR-100: Effective Permission Set in Session Contract & Granular UI Gating

## Status
Accepted

## Context
Charter v2.0 §11 requires: "SessionInfo carries the effective permission set so the UI
gates precisely." Audit AUD-027 established the facts:

- `crates/domain/src/role.rs` defines an exhaustive, tested 34-permission matrix across
  4 strictly hierarchical roles (Vendeur 5, Caissier 13, Gerant 29, Admin 34), with
  `Role::permissions()` as the single source of truth and `has_permission()` as the
  single decision function. Rust use cases remain the sole authorization authority.
- `LoginResponseDto` (`crates/commands/src/dto.rs`) carried only the role name as a bare
  string (`role: String`), and no `SessionInfoDto` exists anywhere in the contract.
- The frontend `SessionInfo` (`src/types/pos.ts`) therefore carried only `role: string`,
  forcing every UI gate (`src/App.tsx`, settings, stock, operations menu, modals) to
  perform crude string comparisons on role names (`session.role === "ADMIN"`), duplicating
  role knowledge in the presentation layer with no compiler-checked link to the domain
  matrix. Any future role/permission change would silently desynchronize the UI.

## Decision
1. `LoginResponseDto` gains `pub permissions: Vec<String>` — the role's full effective
   permission set, computed at login from `session.role.permissions()`.
2. Permission codes are the serde `SCREAMING_SNAKE_CASE` wire form of `Permission`
   (e.g. `USER_MANAGE`, `CREATE_SALE`, `CREDIT_LIMIT_OVERRIDE`), NOT the Rust PascalCase
   variant names. Rationale: the enum already derives `Serialize` with
   `rename_all = "SCREAMING_SNAKE_CASE"`, and every existing wire role code (`ADMIN`,
   `CAISSIER`, …) is SCREAMING_SNAKE_CASE; reusing the derived form keeps one canonical
   string mapping instead of two, with zero domain changes and zero new dependencies
   (realized via the `serde_json` dependency the `commands` crate already carries).
   The task text's PascalCase examples were non-binding illustrations; the repo's own
   serde convention governs.
3. The vector is sorted lexicographically before serialization, so the contract is
   deterministic byte-for-byte across runs (stable snapshots, stable tests).
4. The mapping lives in one function, `sorted_permission_codes(role: Role)` in
   `crates/commands/src/ipc.rs`, next to its single call site (`auth_login`). No
   `SessionInfoDto` is introduced because no `session_info` command exists; if one is
   ever added, it must reuse `sorted_permission_codes`.
5. Frontend: `SessionInfo` (`src/types/pos.ts`) and the `LoginWire` mapping in
   `src/lib/ipc.ts` gain `permissions: string[]`. The session store
   (`src/stores/session-store.ts`) persists `permissions: string[]` alongside the
   session and exposes `hasPermission: (permission: string) => boolean`, resetting both
   on login failure, bootstrap failure, and logout.
6. Rust authorization is unchanged: `SessionContext` still carries only the role, and
   use cases still decide via `has_permission()`. The frontend permission set is a
   read-only mirror for gating convenience, never an authority.

## Consequences
- Positive: UI gates can check granular capabilities (`hasPermission("USER_MANAGE")`)
  instead of role identity, closing Charter §11 bullet 2 and Charter §3.3 engineering
  debt #3 (`SessionInfo` lacks permission set). Presentation no longer hard-codes which
  roles hold which capabilities; that knowledge lives once, in `role.rs`.
- Positive: deterministic sorted serialization keeps the contract snapshot-testable;
  covered by `login_response_carries_sorted_effective_permissions` (commands unit) and
  `src/stores/session-store.test.ts` (3 tests: initial-deny, login-store, logout-clear).
- Negative / accepted: the permission vector travels on every login response (34 short
  strings for Admin — negligible on a local Tauri bridge). Role changes mid-session do
  not refresh the stored set (pre-existing snapshot semantics: `SessionContext` is a
  login-time snapshot; a fresh login picks up changes). No migration, no schema, no
  query, and no live-data impact: pure contract + store extension.
- Out of scope (follow-up DSN tasks): migrating existing `role ===` UI gates to
  `hasPermission`; a `session_info` refresh command; self-service permission views.
