# TASK-DSN-078 REPORT — DocumentsModule root gate migration

## 1. Implementation

- `src/features/documents/DocumentsModule.tsx` [MODIFIED, +10/−9]:
  removed local `isManagerRole` (:19-21) + doc comment; added
  `useSessionStore((s) => s.hasPermission)` reactive selector; gate is now
  `session !== null && hasPermission("CREATE_SALE")` — the exact shell rule
  (App.tsx:95 `canAccessDocuments`), engine authority sales.rs:1312-1313.
  SEARCH_LIMIT 100, debounce 250 ms, wire params unchanged. Existing
  `translateBackendError("UNAUTHORIZED")` fallback kept (no new i18n keys;
  `common.accessDenied` FR+AR already exist). No `session.role` remains.
- `src/features/documents/documents-module.test.ts` [MODIFIED, +24]:
  new `module root gate (TASK-DSN-078)` describe — CREATE_SALE pin +
  reactive selector pin, `isManagerRole`/`session.role` absence, fallback
  branch pin, journal surface pins (filter/table/drawer).
- i18n: untouched (0 lines). No new dependencies. No live DB touched.

## 2. Verification

- documents-module.test.ts: 15/15 pass (12 existing + 3 new).
- CDP drill `audit/DSN-078/capture.mjs` (Chrome headless, stubbed IPC,
  6-row journal fixture): **5/5 steps passed**, 0 console errors.
- Visual: AR-dark-1280 screenshot verified — full RTL, 6 rows, all 8
  columns visible, dark theme, no h-scroll, no overlap, tabular figures
  LTR-isolated (`1 250,00` + `د.ع.`).

## 3. Gates

- G1 `cargo fmt --check`: exit 0. G2 clippy `-D warnings`: exit 0.
- G3 `cargo test --workspace -j 2`: 469 passed / 0 failed / 0 ignored.
- G4 lint: clean. G5 typecheck: clean.
- G6 build: `✓ built in 5.57s`, JS 739.03 kB / gzip 190.15 kB.
- G7 vitest full: **72 files / 704 tests passed** (701 + 3 new).

## 4. Scope & cleanliness

- `git diff --stat`: exactly the 2 allowed paths (34 insertions, 9 deletions).
- `position: fixed` in src: zero matches. Vite :5173 server stopped
  (port DOWN confirmed). No scratch files outside `audit/DSN-078/`
  (capture.mjs + results JSON + 4 PNGs).
- `git status --short`: 2 modified + 3 untracked audit dirs (AUD-033/034
  pre-existing from prior tasks, DSN-078 new). No commit ordered.
