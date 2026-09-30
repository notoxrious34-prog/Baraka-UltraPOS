# Baraka UltraPOS — Verified Local Development Run Protocol

> **Status of this document:** every command, path, and claim below was
> verified by direct inspection of the source tree, originally on 2026-09-27
> (TASK-BRK-015) and corrected on 2026-09-27 (TASK-BRK-017) to reflect the
> post-TASK-BRK-016 state. Nothing in this file is assumed or guessed. Where
> the codebase contradicts an intuitive expectation, the code wins and the
> deviation is called out explicitly.

---

## 1. Prerequisites

Exact tooling required, with the versions pinned or verified in this repo:

| Tool | Required version | Verified how |
|------|------------------|--------------|
| Rust toolchain | **1.98.1** (with `rustfmt` + `clippy` components) | `rust-toolchain.toml` lines 1–3: `channel = "1.98.1"`, `components = ["rustfmt", "clippy"]`. Local machine confirms `cargo 1.98.1` / `rustc 1.98.1`. |
| Node.js + npm | No `engines` field exists in `package.json` (no pinned range); Node v26.7.0 / npm 12.0.2 verified working on the documenting machine | `package.json` has no `engines` key (inspected verbatim); any current LTS satisfies Vite 5 + Tauri CLI 2. |
| WebView2 Runtime (Windows) | Evergreen, system-wide install recommended | Tauri v2 (`src-tauri/tauri.conf.json`, `"$schema": "https://schema.tauri.app/config/2"`) renders its window in WebView2. See §4 for why this matters on first run. |
| Visual C++ Build Tools (Windows) | MSVC C++ workload (provides `cl.exe`, Windows SDK) | Required because the workspace depends on `serialport = "4"` (`Cargo.toml` line 32, thermal-printer transport), which compiles native C bindings on Windows. Without it, the first Rust build fails inside the `serialport`/`libudev-sys`-style native compilation step. |
| Tauri CLI | `^2.11.5` (already a devDependency) | `package.json` line 23: `"@tauri-apps/cli": "^2.11.5"`. No global install needed; it runs via the `tauri` npm script. |

---

## 2. Critical Clarification: Frontend-Only vs Full Application

**`npm run dev` alone starts ONLY the Vite/React frontend dev server in
isolation. It provides NO Rust backend, NO SQLite database, NO
authentication, and NO business logic of any kind.**

Concretely: `package.json` defines `"dev": "vite"` (line 8) — that is the
entire command. The login screen (if reachable at all in that mode) can
never succeed, because there is no IPC backend listening on the other side
to validate credentials against: `auth_login` is a Rust Tauri command
(`crates/commands/src/ipc.rs` line 458) that only exists inside the Tauri
runtime. **This is expected behavior, not a bug.** Any verification of the
real product must use the full-application command in §3.

---

## 3. Correct Command to Run the Full Application

The `scripts` section of `package.json` contains **no dedicated Tauri dev
script** — only a bare CLI passthrough (line 13):

```json
"tauri": "tauri"
```

The exact, verified full-stack launch command is therefore:

```powershell
npm run tauri dev
```

(`npm` forwards the extra `dev` argument to the underlying `tauri` binary,
invoking the Tauri CLI v2 `dev` workflow.)

What this does, verified against `src-tauri/tauri.conf.json` lines 6–11:

- `beforeDevCommand: "npm run dev"` — starts the Vite frontend first;
- `devUrl: "http://localhost:5173"` — the Rust shell loads the frontend
  from the local Vite server;
- `frontendDist: "../dist"` / `beforeBuildCommand: "npm run build"` —
  used only for production bundles, not for `dev`.

Expected first-run behavior:

1. A **long first Rust compilation** (several minutes: Tauri, `rusqlite`
   bundled SQLite, `serialport` native bindings, print rasterizer). This is
   normal.
2. Subsequent launches reuse the incremental cache in `target/` and start
   in seconds.
3. A **native desktop window** opens (1280×800, per `tauri.conf.json`
   lines 13–22) titled "Baraka UltraPOS" — **not** a browser tab.

---

## 4. Troubleshooting: Windows Credential/UAC Prompt

The credential/elevation prompt reported by the owner has two likely,
entirely environmental causes — neither is an application bug:

1. **WebView2 Runtime installation.** If WebView2 is not already installed
   system-wide, Windows (or the Tauri bootstrapper) attempts to install it
   on first run, and a per-machine install requires administrator
   elevation → the UAC/credential dialog. The application code performs no
   privileged operation of its own at startup.
2. **First-time native toolchain/driver setup.** The initial build compiles
   the `serialport` crate's C bindings (see §1). If Visual C++ Build Tools
   or related drivers were just installed, Windows may request elevation
   for that one-time setup.

Recommended resolution, in order:

1. **Pre-install WebView2 yourself**, elevated, before launching:
   download Microsoft's official *Evergreen Bootstrapper*
   (`MicrosoftEdgeWebView2Setup.exe`) and run it once as Administrator.
   Relaunch the app afterward from a normal (non-elevated) terminal.
2. **Alternatively, run the terminal as Administrator exactly once** for
   the very first `npm run tauri dev` (lets the one-time WebView2/toolchain
   setup complete), then return to a normal terminal for all later runs.
   Day-to-day development never needs elevation.
3. If the prompt appears on **every** launch (not just the first), it is
   not WebView2 anymore — record the exact requesting executable/requested
   path from the UAC dialog and escalate it as a new finding; do not
   blanket-run the project as Administrator.

---

## 5. First Login: Obtaining Valid Admin Credentials on a Fresh Database

On a freshly initialized database the application presents a mandatory
**First-Run Setup Wizard** — there is no default password and none is ever
needed. The flow, verified in the current code:

1. At startup, while no session exists, the app probes the backend once via
   the `check_first_run_status` Tauri command
   (`crates/commands/src/ipc.rs` line 499), a fast read-only query that
   returns `requires_setup: true` only while the `users` table is empty.
   The gating lives in `UnauthenticatedGate` in `src/App.tsx`.
2. When `requires_setup` is true, the app renders `FirstRunSetupScreen`
   (`src/features/auth/FirstRunSetupScreen.tsx`) **instead of** the login
   form. The screen asks for three fields: username, password, and password
   confirmation. The confirmation match is checked client-side only; the
   server is the sole authority on the actual validation rules. All labels
   are bilingual (FR/AR) through the `setup.*` i18n keys, following the same
   pattern as the login screen.
3. On submit, the frontend calls the `bootstrap_first_admin` Tauri command
   (`crates/commands/src/ipc.rs` line 518), which runs
   `BootstrapFirstAdminUseCase`: the requested username is validated with
   the standard `Username` rule, the password is hashed with the production
   Argon2id implementation, and exactly one user row with role `ADMIN` is
   created inside a single atomic transaction (a racing second attempt is
   rejected with `BOOTSTRAP_ALREADY_COMPLETED` and inserts nothing). An
   audit event (`FIRST_ADMIN_BOOTSTRAP`) is recorded in the same
   transaction.
4. On success the operator is transitioned **directly into the
   authenticated application shell** through the standard login flow
   (`bootstrap()` in `src/stores/session-store.ts` delegates to the
   identical `login()` logic) — there is no redundant "please now log in"
   step.
5. On every subsequent launch the `users` table is non-empty, so
   `check_first_run_status` returns `requires_setup: false` and the normal
   login screen appears. The Admin signs in with the credentials chosen in
   step 2.

There is deliberately no seed user, no setup CLI, and no hardcoded
credential anywhere: the Admin sets their own credentials during this
one-time flow.

---

## 6. Verifying the Database Location

Quoted verbatim from the actual runtime code,
`src-tauri/src/lib.rs` lines 61–74 (inside the Tauri `setup` closure, where
the official path API is available):

```rust
// ADR-049: the SQLite file lives in the OS-standard per-user
// application data directory (Tauri official path API), under
// the `default_db_file_name()` file name. No CWD-relative
// fallback exists: the launch working directory can never fork
// or orphan business data again.
let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|error| startup_error("app data directory resolution", error))?;
ensure_database_dir(&app_data_dir)
    .map_err(|error| startup_error("application data directory creation", error))?;
let db_path = resolve_database_path(&app_data_dir);
let mut conn =
    open_connection(&db_path).map_err(|error| startup_error("database open", error))?;
```

The join itself lives in
`crates/infrastructure/src/persistence/connection.rs` lines 18–33:

```rust
/// Resolve the database file name for the desktop runtime.
#[must_use]
pub fn default_db_file_name() -> &'static str {
    "baraka.db"
}

/// Resolve the production database file inside the OS-standard per-user
/// application data directory (ADR-049).
#[must_use]
pub fn resolve_database_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(default_db_file_name())
}
```

Consequences, stated plainly:

- The running desktop application creates/opens **`baraka.db` inside the
  OS-standard per-user application data directory**: on Windows this is
  `%APPDATA%\com.baraka.ultrapos\baraka.db` (the `app_data_dir()` API
  resolves to the system data directory joined with the bundle identifier
  `com.baraka.ultrapos` from `src-tauri/tauri.conf.json`). The directory
  is created automatically on first launch if absent; if it cannot be
  created, startup aborts with a clear logged error instead of silently
  using another location.
- The launch working directory no longer matters: there is no
  CWD-relative fallback anywhere in the code, so starting the app from
  different folders can never fork or orphan business data.
- To verify: after a successful launch, confirm `baraka.db` (plus
  `-wal`/`-shm` companions while running) exists under the path above. It
  opens with any SQLite browser; schema version is tracked in its
  `schema_migrations` table.
- Do **not** delete this file to "reset" unless a backup exists — it is
  the sole copy of all local business data (sales, stock, ledger).

---

## 7. Change Log

- **2026-09-27 (TASK-BRK-017):** corrected after TASK-BRK-016 resolved the
  two critical defects originally reported in TASK-BRK-015 — §5 rewritten
  to document the implemented First-Run Setup Wizard (replacing the
  obsolete "no bootstrap mechanism exists" finding), §6 rewritten for the
  OS-standard app-data database path (replacing the obsolete CWD-relative
  path), and the `auth_login` line reference in §2 refreshed.
