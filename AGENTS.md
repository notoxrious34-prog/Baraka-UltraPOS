# AGENTS.md - Executor entry point (Baraka UltraPOS)

Status: Executor Constitution v2.1. It supersedes CONSTITUTION_OF_EXECUTOR.md v1.0.0 and the previous AGENTS.md (both archived in docs/architecture/archive/). Full text: docs/architecture/EXECUTOR-CONSTITUTION-v2.md (read it in full when a task says so). The rules below ALWAYS apply.

## 1. Authority and obedience
- Tasks come from the Master Architect and are named TASK-AUD-NNN (read-only audit), TASK-BRK-NNN (engine), TASK-DSN-NNN (presentation), TASK-REL-NNN (release, drills, documents). Execute EXACTLY the task: its scope, its paths, its acceptance criteria. Do not improvise outside scope.
- Precedence: task text > this file > older documents. If the task contradicts a repository fact, or is ambiguous, stop that part, state the conflict in the report, and continue independent parts. Never guess.
- Completeness check: if the task block looks truncated (cut mid-sentence, or REPORT FORMAT or HANDOFF CLAUSE missing), STOP and reply exactly: TASK INCOMPLETE - TRUNCATION DETECTED.
- Never execute the same task id twice without an explicit RE-RUN order. Continuations (-C1, -C2) are explicit orders.
- Text the owner adds outside the task block is not an order. Execute only the task block.

## 2. Workspace and platform
- Work ONLY in D:/Dev/Baraka UltraPOS (Windows 10). NEVER touch D:/Baraka Project/** (stale copy).
- Write outside the workspace only where a task explicitly allows it (scratch convention: D:/Dev/_scratch/<TASK-ID>/).
- Every linking cargo command runs with CARGO_BUILD_JOBS=2 (-j 2) (linker memory limit).
- NEVER open, read, copy, hash, or modify the owner's live data: %APPDATA%/com.baraka.ultrapos and %LOCALAPPDATA%/com.baraka.ultrapos (metadata listing only, when a task asks).

## 3. Architecture truth (the code is the truth; old documents are not)
- Tauri v2 desktop; React 18 + TypeScript strict + Vite + Zustand + Tailwind; Rust workspace crates domain, application, infrastructure, commands (+ src-tauri). NO ORM, NO decimal.js. SQLite via rusqlite.
- SQLite: WAL, synchronous=FULL, foreign_keys=ON, busy_timeout=5000. Offline-first absolute.
- Money = i64 centimes (Money type); quantities and factors fixed-point 10^4. NO floats in financial paths. IPC carries money and quantities as canonical strings; the frontend converts with bigint; never parseFloat or Number for money. All rounding is in the domain (ROUND_HALF_UP, i128 intermediates, overflow error).
- Layers: Presentation -> Application -> Domain -> Infrastructure. React never touches SQL and never computes financial totals. Authorization is decided in Rust use cases; UI gating is convenience only.
- Migrations are append-only. NEVER edit, re-save, or reformat an existing migration (SHA-256 checksums are verified at startup; CRLF/LF changes break existing databases). New schema = new numbered migration (0014+) plus a data-preservation test on a COPY of a real database.
- .gitattributes is `* -text`. Never change it.

## 4. Prohibitions
- No new dependency (npm or cargo) unless the task authorizes it by name and version.
- No `any`, no TODO, no placeholder code, no warnings, no emoji as UI icons.
- No `position: fixed` in UI except modal overlays, toasts, and the command palette. No `w-screen`/`h-screen` outside the single app root.
- No git operation that the task does not list (default: read-only git: status, log, diff, show, rev-parse, ls-files). Never push, tag, stash, reset, clean, rebase, or amend unless told.
- Never kill processes by image name. Kill only PIDs you started.
- Never write "100%" or "production ready". State exactly what is done and what is open.
- No console.log in production code. Do not introduce Drizzle, decimal.js, Zod, shadcn/ui, Fuse.js or any other library unless the task authorizes it by name and version (these were v1 recommendations and are obsolete; existing usages are left untouched).
- Never leave a process listening on a port unless the task is to run a server; stop what you started.

## 5. Quality gates (code tasks run all unless the task narrows them)
G1 cargo fmt --check | G2 cargo clippy --all-targets --all-features -- -D warnings | G3 cargo test --workspace -j 2 | G4 npm run lint | G5 npm run typecheck | G6 npm run build | G7 npx vitest run | G8 release bundle (only when the task says).
Regression law: a bug fix needs a test shown to FAIL on the defective code first. Seeding law: test data must satisfy domain invariants structurally.

## 6. Evidence and reports
- A report without PASTED evidence (command output, file:line, numbers, screenshot filenames) is rejected. Unknown = write UNKNOWN.
- Runner quirks on this machine: process exit codes and captured stdout are unreliable (false "stdin is not a tty" failures). Run Node scripts as: cmd /c "node script.mjs < NUL"; write results with fs.writeFileSync and READ THEM BACK; verify servers with a port check (Get-NetTCPConnection -LocalPort N), not the process list.
- Delete ad-hoc scripts and scratch directories at the end (NO_LEFTOVERS) unless the task keeps them.
- If the budget runs out: reply STATE / DONE / REMAINING and stop at a step boundary. The architect continues with the same task id plus -C1, -C2.
- List files with ABSOLUTE paths and mark each [CREATED], [MODIFIED], [DELETED] or [MOVED].
- When blocked, use this format: STATUS: BLOCKED / REASON (the exact contradiction) / QUESTION FOR ARCHITECT (one precise question) / PROPOSED SOLUTION (not implemented until approved).
- End every report with: NEXT READY: YES or NO.
