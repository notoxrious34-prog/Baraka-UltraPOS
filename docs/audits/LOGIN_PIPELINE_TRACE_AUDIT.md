# LOGIN PIPELINE TRACE AUDIT — Silent Failure Diagnosis (TASK-BRK-019)

> **Nature:** pure static (read-only) audit. Zero code changes made or authorized.
> Every claim below quotes the current source verbatim with file path and line
> number. Rankings are code-path analysis, not runtime proof — where static
> reading cannot decide, it is flagged explicitly.
>
> **Observed symptoms under diagnosis:**
> S1. After a successful first-admin bootstrap, the owner landed on the
> standard `LoginScreen` instead of the POS shell.
> S2. Clicking "Se connecter (Entrée)" with correct credentials produces
> absolutely nothing observable: no navigation, no error message, no state
> change of any kind.

---

## 1. Per-file responsibility map (SCOPE-IN)

| # | File | Role in login pipeline |
|---|------|------------------------|
| 1 | `src/features/auth/LoginScreen.tsx` | Form + submit handler + error display |
| 2 | `src/lib/ipc.ts` | `ipcLogin` wrapper around Tauri `invoke()` |
| 3 | `src-tauri/src/lib.rs` | Command registration + shared `AppState` wiring |
| 4 | `crates/commands/src/ipc.rs` | `auth_login` handler + `error_code` mapping |
| 5 | `crates/commands/src/dto.rs` | `LoginRequestDto` / `LoginResponseDto` shapes |
| 6 | `crates/application/src/use_cases/mod.rs` | `AuthenticateUserUseCase::execute` |
| 7 | `crates/infrastructure/src/adapters/sqlite_user_repository.rs` | `find_by_username`, `get_password_hash`, lockout writes |
| 8 | `src/stores/session-store.ts` | `login()` — session state mutation |
| 9 | `src/App.tsx` | Routing condition (`session === null ? … : shell`) |

---

## 2. Full async call-chain trace (in mandated order)

### Hop (a) — `src/features/auth/LoginScreen.tsx` lines 19–28 (submit) + 37–42 (form wiring)

```tsx
const submit = (): void => {
  if (authBusy) {
    return;                      // <-- lines 20-22: silent early return, no UI signal
  }
  setError(null);
  login(username, password).catch((failure: unknown) => {
    const code = typeof failure === "string" ? failure : "INTERNAL_ERROR";
    setError(translateBackendError(code, usePosUiStore.getState().locale));
  });
};
```

```tsx
<form
  className="flex w-full max-w-xs flex-col gap-2"
  onSubmit={(event) => {
    event.preventDefault();
    submit();
  }}
>
```

And the error-render branch, lines 67–69:

```tsx
{error !== null && (
  <p className="rounded-md bg-red-900/60 px-3 py-2 text-sm font-bold text-red-100">{error}</p>
)}
```

And the button, lines 70–76:

```tsx
<button
  type="submit"
  disabled={authBusy}
  className="h-12 rounded-lg bg-accent-500 text-base font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
>
  {authBusy ? t(locale, "login.loading") : t(locale, "login.submit")}
</button>
```

**Verdict for spec 4 (first half):** yes, the login call carries a `.catch()`
(line 24). The caught error is **always surfaced to UI state**: any string
rejection is translated via `translateBackendError` (which itself never
returns empty — unknown codes fall back to the `INTERNAL_ERROR` message), and
non-string rejections are coerced to `"INTERNAL_ERROR"`. There is **no
`console.error`-only path and no discard path** in this handler. A *rejected*
login promise therefore **cannot** be silent — it must render the red `<p>`
block. Consequence: symptom S2 (zero error text) rules out every path that
ends in a rejected promise, *provided* `submit()` got past line 20.

### Hop (b) — `src/lib/ipc.ts` lines 28–64 (`call`, `isTauri`, `ipcLogin`)

```ts
function bridgeError(): Error {
  const error = new Error("BRIDGE_UNAVAILABLE");
  return error;
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function call<T>(command: string, args: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw bridgeError();
  }
  return invoke<T>(command, args);
}
```

```ts
interface LoginWire {
  session_token: string;
  user_id: string;
  username: string;
  role: string;
  expires_at: string;
}

/** Authenticate; resolves a typed session or rejects with a wire code. */
export async function ipcLogin(username: string, password: string): Promise<SessionInfo> {
  const wire = await call<LoginWire>("auth_login", {
    request: { username, password },
  });
  return {
    token: wire.session_token,
    userId: wire.user_id,
    username: wire.username,
    role: wire.role,
    expiresAt: wire.expires_at,
  };
}
```

Arg shape matches the Rust signature exactly (`request: LoginRequestDto` —
see hop (d)). A non-Tauri context throws `BRIDGE_UNAVAILABLE`, which is a
*rejection* → visible error in hop (a). No discard here.

### Hop (c) — `src-tauri/src/lib.rs` lines 93, 129–130 (state + registration)

```rust
hasher: Arc::new(Argon2PasswordHasher),
```

```rust
.invoke_handler(tauri::generate_handler![
    baraka_commands::ipc::app_health,
    baraka_commands::ipc::auth_login,
    baraka_commands::ipc::auth_logout,
    baraka_commands::ipc::check_first_run_status,
    baraka_commands::ipc::bootstrap_first_admin,
    ...
```

`auth_login` **is registered**. `AppState` carries one shared
`Arc<Mutex<Connection>>` (`db`) cloned into every repository, one shared
`Arc<dyn PasswordHasher>`, one `InMemorySessionStore`, one `SystemClock`.
No pool, no async runtime dependency on this path.

### Hop (d) — `crates/commands/src/ipc.rs` lines 456–478 (handler) + 151–165 (error map)

```rust
/// `auth_login` command: username + password authentication.
#[tauri::command]
pub fn auth_login(
    state: tauri::State<'_, AppState>,
    request: LoginRequestDto,
) -> Result<LoginResponseDto, String> {
    let login = AuthenticateUserUseCase::new(
        state.hasher.as_ref(),
        state.users.as_ref(),
        state.sessions.as_ref(),
        state.clock.as_ref(),
    );
    match login.execute(&request.username, &request.password) {
        Ok(session) => Ok(LoginResponseDto {
            session_token: session.token.as_str().to_string(),
            user_id: session.user_id.to_string(),
            username: session.username,
            role: session.role.code().to_string(),
            expires_at: session.expires_at.to_rfc3339(),
        }),
        Err(error) => Err(error_code(&error)),
    }
}
```

```rust
fn error_code(error: &ApplicationError) -> String {
    match error {
        ApplicationError::Validation(DomainError::InvalidCredentials) => {
            "INVALID_CREDENTIALS".to_string()
        }
        ApplicationError::Validation(DomainError::AccountInactive) => {
            "ACCOUNT_INACTIVE".to_string()
        }
        ApplicationError::Validation(DomainError::AccountLocked { remaining_seconds }) => {
            format!("ACCOUNT_LOCKED:{remaining_seconds}")
        }
        ApplicationError::Validation(DomainError::PinNotSet) => "PIN_NOT_SET".to_string(),
        ...
```

Every `ApplicationError` variant maps to a **non-empty** `String` (terminal
arms: `NotFound(_) → "NOT_FOUND"`, `Unauthorized → "UNAUTHORIZED"`,
`Port(_) → "INTERNAL_ERROR"`). A backend failure therefore always arrives in
JS as a rejected promise carrying a displayable code. **No success-shaped
failure payload exists anywhere on this path** (spec 5, backend half).

### Hop (e) — `crates/application/src/use_cases/mod.rs` lines 134–193 (`execute`)

```rust
pub fn execute(
    &self,
    username: &str,
    password: &str,
) -> Result<AuthenticatedSession, ApplicationError> {
    let trimmed = username.trim();
    if trimmed.is_empty() {
        return Err(ApplicationError::Validation(
            DomainError::InvalidCredentials,
        ));
    }
    let user = match self.users.find_by_username(trimmed)? {
        Some(user) => user,
        None => {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCredentials,
            ));
        }
    };
    if !user.is_active {
        return Err(ApplicationError::Validation(DomainError::AccountInactive));
    }
    let now = self.clock.now();
    if let Some(locked_until) = user.locked_until {
        if now < locked_until {
            let remaining = (locked_until - now).num_seconds();
            return Err(ApplicationError::Validation(DomainError::AccountLocked {
                remaining_seconds: remaining,
            }));
        }
    }
    let stored = self.users.get_password_hash(user.id)?;
    if !self.hasher.verify(password, &stored)? {
        let attempts = user.failed_login_attempts.saturating_add(1);
        if attempts >= MAX_LOGIN_ATTEMPTS {
            let locked_until = now + TimeDelta::seconds(LOCKOUT_SECONDS);
            self.users
                .record_login_failure(user.id, Some(locked_until))?;
            return Err(ApplicationError::Validation(DomainError::AccountLocked {
                remaining_seconds: LOCKOUT_SECONDS,
            }));
        }
        self.users.record_login_failure(user.id, None)?;
        return Err(ApplicationError::Validation(
            DomainError::InvalidCredentials,
        ));
    }
    self.users.reset_login_failures(user.id)?;
    let token = self
        .sessions
        .create_session(user.id, user.role, SESSION_TTL_SECONDS);
    Ok(AuthenticatedSession {
        token,
        user_id: user.id,
        username: user.username.as_str().to_string(),
        role: user.role,
        expires_at: now + TimeDelta::seconds(SESSION_TTL_SECONDS),
    })
}
```

Production code of this file (lines 1–~340) contains **zero** `unwrap`,
`expect`, `.ok()`, `unwrap_or`, `if let Err(_)`, or `let _ =` — the grep hits
in this file (lines 363+) are all inside `#[cfg(test)]`. Every fallible step
uses `?` into `ApplicationError`, which hop (d) renders visible.

### Hop (f) — `crates/infrastructure/src/adapters/sqlite_user_repository.rs` lines 78–91, 127–141

```rust
fn find_by_username(&self, username: &str) -> Result<Option<User>, ApplicationError> {
    let guard = self.lock()?;
    let mut statement = guard
        .prepare(
            "SELECT id, username, role, is_active, failed_login_attempts, locked_until
         FROM users WHERE username = ?1",
        )
        .map_err(|_| store_failure())?;
    let mut rows = statement.query([username]).map_err(|_| store_failure())?;
    match rows.next().map_err(|_| store_failure())? {
        Some(row) => Ok(Some(Self::map_row(row)?)),
        None => Ok(None),
    }
}
```

```rust
fn get_password_hash(&self, user_id: UserId) -> Result<String, ApplicationError> {
    let guard = self.lock()?;
    guard
        .query_row(
            "SELECT password_hash FROM users WHERE id = ?1",
            [user_id.as_uuid().to_string()],
            |row| row.get(0),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                ApplicationError::NotFound("user".to_string())
            }
            _ => store_failure(),
        })
}
```

`lock()` maps a poisoned mutex to `ApplicationError::Port("database lock
poisoned")` → visible `INTERNAL_ERROR`. Every rusqlite failure maps to the
static `store_failure()` → visible. Password verification itself is
`Argon2PasswordHasher::verify` (hop (e), line 167) — see §4 below.

### Hop (g) — return path `(f) → (e) → (d) → (b) → (a)`

- (f) `Ok(stored_hash)` → (e) `verify` → `Ok(AuthenticatedSession)` → (d)
  `Ok(LoginResponseDto)` → Tauri serializes → (b) `LoginWire` → mapped to
  `SessionInfo` → (a) `login()` continues after line 34.
- Any `Err` at (f)/(e) → (d) `Err(error_code)` → invoke **rejects** → (a)
  `.catch` → red error block. Always visible.

`session-store.ts` lines 31–43 (the state mutation that drives navigation):

```ts
login: async (username, password) => {
  set({ authBusy: true, authError: null });
  try {
    const session = await ipcLogin(username.trim(), password);
    set({ session, authBusy: false });            // <-- the ONLY write to `session`
    await get().loadWarehouses();
    await useCashSessionStore.getState().refresh(session.token);
  } catch (error: unknown) {
    const code = typeof error === "string" ? error : "INTERNAL_ERROR";
    set({ authBusy: false, authError: code, session: null });
    throw error;
  }
},
```

**Spec 5 verdict:** `session` is written **only** inside the verified-success
branch (line 35), after `await ipcLogin` resolved. There is no
`{ success: false }`-style payload anywhere: the backend contract is
`Result<LoginResponseDto, String>`, so failure **must** arrive as a rejected
promise. The frontend never inspects a failure payload because none can exist.

---

## 3. Error-discarding pattern sweep (verbatim, with disposition)

### CANDIDATE SILENT FAILURE POINT #1 — `src/features/auth/LoginScreen.tsx:20-22

```tsx
if (authBusy) {
  return;
}
```

Fully silent early return: no state change, no error, no log. If `authBusy`
is ever stuck `true` (see Candidate #2), **every** subsequent click is a
no-op, and the `disabled={authBusy}` button (line 72) does not even fire
`onSubmit`. Matches symptom S2 exactly, click after click.

### CANDIDATE SILENT FAILURE POINT #2 — hanging `await` freezing `authBusy=true`

`src/stores/session-store.ts:32-37`:

```ts
set({ authBusy: true, authError: null });
try {
  const session = await ipcLogin(username.trim(), password);
  set({ session, authBusy: false });
  await get().loadWarehouses();
  await useCashSessionStore.getState().refresh(session.token);
```

If the promise awaited at line 34, 36, or 37 **never settles** (neither
resolves nor rejects), execution never reaches line 35 or the `catch` at
line 38: `authBusy` stays `true` forever and no error is ever set. From that
moment on, Candidate #1 silences all future clicks. This is the only
frontend mechanism that yields *zero* UI signal, and it is self-latching.
(The Rust command itself is synchronous and registered — §6 — so a
never-settling promise would implicate the transport/lock layer beneath the
command, not the command body; see Q2.)

### CANDIDATE SILENT FAILURE POINT #3 — synchronous throw before `.catch` attaches

`src/features/auth/LoginScreen.tsx:24`:

```tsx
login(username, password).catch((failure: unknown) => {
```

If evaluating `login(username, password)` itself throws synchronously
(e.g. the `login` selector binding were `undefined` after a failed module
load, or `setError(null)` threw), the exception propagates out of `submit()`
into React's event dispatch — console-only, UI unchanged, fully silent. The
store provably defines `login` (`session-store.ts:31`), so this requires a
runtime module-loading anomaly; listed because its silence signature matches
S2 and it is checkable in one glance at the DevTools console.

### Examined and CLEARED (quoted for the record — not silent)

- `src/stores/session-store.ts:62-64` — `catch {}` discards logout revocation
  errors only (`auth_logout` is idempotent server-side; local state is cleared
  unconditionally at line 67). Not on the login path.
- `src/stores/cash-session-store.ts:35-38` — `refresh()` catches without
  rethrow (`set({ loading: false, error: code })`). Harmless to login:
  `session-store.ts:37` awaits it, and since it never rejects, login proceeds
  to completion regardless of cash-probe outcome.
- `src/App.tsx:71-77` — first-run probe `.catch(() => setRequiresSetup(false))`
  falls back to the login form. Deliberate, commented, login-preserving.
- `crates/infrastructure/src/adapters/sqlite_user_repository.rs:55` —
  `u8::try_from(attempts_raw).unwrap_or(u8::MAX)` clamps a corrupt counter to
  `u8::MAX`; it cannot block or misroute a login (worst case: premature
  lockout → *visible* `AccountLocked`).
- `crates/commands/src/ipc.rs:723,1325,1330` — `.unwrap_or_default()` on
  non-auth DTO fallbacks (empty base-unit id / optional display strings);
  none on the auth path. Lines 2254/2258 `expect()` are `#[cfg(test)]`.
- All `expect()` in `crates/application/src/use_cases/mod.rs` (lines 363+)
  are `#[cfg(test)]` fakes/harness only.

---

## 4. Spec 7 — password hash consistency (HIGH-PRIORITY SUSPECT): CLEARED

Bootstrap write path — `crates/application/src/use_cases/mod.rs:323`:

```rust
let password_hash = self.hasher.hash(&input.password)?;
```

Login verify path — same file, line 167:

```rust
if !self.hasher.verify(password, &stored)? {
```

Both `self.hasher` resolve to the **same** `Arc<dyn PasswordHasher>`
constructed once in `src-tauri/src/lib.rs:93`:

```rust
hasher: Arc::new(Argon2PasswordHasher),
```

The single shared implementation — `crates/infrastructure/src/adapters/argon2_hasher.rs:18-33`:

```rust
impl PasswordHasher for Argon2PasswordHasher {
    fn hash(&self, plaintext: &str) -> Result<String, ApplicationError> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(plaintext.as_bytes(), &salt)
            ...
    fn verify(&self, plaintext: &str, hash: &str) -> Result<bool, ApplicationError> {
        let parsed = PasswordHash::new(hash)...;
        Ok(Argon2::default()
            .verify_password(plaintext.as_bytes(), &parsed)
            .is_ok())
```

Identical algorithm (`Argon2::default()` = Argon2id, OWASP params), identical
PHC encoding, **one** module — no divergent implementations exist. A genuine
mismatch would surface as *visible* `INVALID_CREDENTIALS` in any case, so
hash divergence is eliminated both statically **and** symptomatically as the
cause of a *silent* failure.

---

## 5. Spec 8 — App.tsx routing variable vs. login mutation: CONNECTED (cleared)

Routing condition — `src/App.tsx:93,105-107`:

```tsx
const session = useSessionStore((state) => state.session);
...
{session === null ? (
  <UnauthenticatedGate />
) : (
```

Login mutation — `src/stores/session-store.ts:35`:

```ts
set({ session, authBusy: false });
```

Same Zustand store (`useSessionStore`), same key (`session`). A successful
login **does** mutate the exact variable App.tsx reads; there is no
state-A/state-B disconnect. Eliminated.

Note on symptom S1 (bootstrap → LoginScreen instead of shell): `bootstrap()`
(`session-store.ts:44-56`) delegates to the identical `login()`; had that
login resolved, `session` would be non-null and the shell would render. The
observed LoginScreen means the gate probed `requires_setup: false` with
`session === null` — i.e. **a fresh mount after the user row existed** (app
restart/reload between bootstrap and the login attempt), not a routing
defect. The gate (`App.tsx:60-90`) probes once per mount and never re-probes,
which is consistent with this reading.

---

## 6. Spec 6 + Spec 9 — Rust command shape and async behavior: VERIFIED

- Return type is `Result<LoginResponseDto, String>` (`ipc.rs:461`) — the
  ADR-050 shape. Success `T` (`dto.rs:72-84`): `session_token`, `user_id`,
  `username`, `role`, `expires_at`. Navigation is triggered indirectly: any
  non-null `session` object flips `App.tsx:105`. The token itself is consumed
  by subsequent calls (`loadWarehouses`, cash `refresh`).
- The command is a **synchronous** `pub fn` (`ipc.rs:458`), not `async`.
  There is no future to leave unawaited on either side: Tauri executes it on
  its threadpool and settles the JS promise with `Ok` (resolve) or `Err`
  (reject). No dangling-future class of bug is constructible here.

---

## 7. Q1 / Q2 — the two mandated questions

### Q1: Is the login command actually invoked when the button is clicked?

**Statically: YES — conditionally.** The chain form-submit → `submit()` →
`login()` → `ipcLogin()` → `invoke("auth_login")` is intact and the command
is registered. The two static preconditions are (i) `authBusy === false` at
click time (else Candidate #1 returns silently) and (ii) the `login` binding
is a function (else Candidate #3 throws silently). Whether `invoke` was
actually dispatched on the owner's machine **cannot be proven by static
reading** — it requires one runtime check: DevTools console (any red
exception? any `ACCOUNT_LOCKED`/timer text?) plus whether the button shows
the `login.loading` ("Connexion…") state, which proves `authBusy === true`
was reached, i.e. `login()` was entered.

### Q2: If invoked, does it return Ok(...), Err(...), or panic/never return?

**By code-path analysis: it must return `Ok` or `Err` — both observable.**
Every `Err` maps to a non-empty wire code (§2 hop (d)) → rejected promise →
red error block (§2 hop (a)). A Rust panic inside the handler would surface
as a rejected invoke (and, if it poisoned the shared `Mutex<Connection>`,
all later repo calls fail fast with visible `INTERNAL_ERROR`, not silence).
The **only** statically constructible silent outcome is *never returning*:
a stall beneath the command (OS mutex stall, WebView2 IPC stall, or a hung
`Argon2::verify`), which freezes `authBusy=true` via Candidate #2 and then
latches all future clicks silent via Candidate #1. Confirming or eliminating
the stall class **requires runtime instrumentation** (backend stderr log
around `auth_login` entry/exit + a DevTools promise-settlement check) and
cannot be decided by reading alone.

---

## 8. CANDIDATE ROOT CAUSES (ranked most → least likely)

1. **Never-settling `invoke("auth_login")` latching `authBusy=true`** (Candidates
   #2→#1) — the *only* mechanism yielding zero error text AND zero navigation
   AND persistence across clicks; fully matches S2.
2. **Synchronous throw before `.catch` attaches** (Candidate #3) — matches S2's
   silence signature; checkable in seconds via the DevTools console.
3. **Post-login `loadWarehouses()` throw wiping a just-set session**
   (`session-store.ts:35-41`) — produces a *visible* error, so it contradicts
   S2 as reported; listed to be eliminated at runtime, not pursued.
4. **Username/credential mismatch at lookup** (wrong name, lockout, inactive
   flag) — always *visible* (`INVALID_CREDENTIALS` / `ACCOUNT_LOCKED` /
   `ACCOUNT_INACTIVE`); contradicts S2, but the DB-side counters
   (`failed_login_attempts`) will confirm at runtime whether the backend was
   ever reached.
5. **Argon2id hash/verify divergence** — eliminated statically (§4) and
   symptomatically (would be visible).
6. **App.tsx reading disconnected state** — eliminated statically (§5).
7. **Missing command registration / async misuse** — eliminated statically (§6,
   `lib.rs:129` registered, sync fn).

**Recommended runtime instrumentation (no fix authorized by this task):**
(a) DevTools console open before clicking — any exception text immediately
decides Candidate #3; (b) observe whether the button sticks on "Connexion…"
— decides whether `login()` was entered; (c) backend stderr log at
`auth_login` entry/exit — decides whether the invoke reached Rust and whether
it returned; (d) `SELECT username, failed_login_attempts, locked_until FROM
users` after a click — a bumped counter proves the backend ran and rejected,
i.e. the silence is frontend-side (Candidates #1–#3), while an unchanged row
points at the transport stall (Candidate #2's lower half).
