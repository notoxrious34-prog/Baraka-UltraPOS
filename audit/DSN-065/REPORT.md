# TASK-DSN-065 — Persistent Global Theme Store & Desync Elimination: REPORT

## 1. ROOT CAUSE & TOPBAR DIFF

`Topbar.tsx` owned theme in `useDarkMode()` (`const [dark, setDark] = useState(false)`
+ effect `document.documentElement.classList.toggle("dark", dark)`). Topbar mounts
ONLY in the POS view (`PosShellView.tsx:23`); navigating away unmounts it
(destroying state, class left behind — no cleanup), returning remounts at `false`
and strips `dark` from the root. Nothing persisted. (Variance: task text quotes
`isDark` naming; real code used `dark`/`useDarkMode` — same mechanism.)
```diff
-function useDarkMode(): { dark: boolean; toggle: () => void } {
-  const [dark, setDark] = useState(false);
-  useEffect(() => {
-    document.documentElement.classList.toggle("dark", dark);
-  }, [dark]);
-  return { dark, toggle: () => setDark((value) => !value) };
-}
+import { useThemeStore } from "@/stores/theme-store";
+  const dark = useThemeStore((state) => state.isDark);
+  const toggle = useThemeStore((state) => state.toggleTheme);
```
Button UI untouched (icon/label/flow identical); file no longer references
`document` at all. `App.tsx`: added `useThemeStore.getState().initTheme()` in a
mount-once `useEffect` (+ import).

## 2. `src/stores/theme-store.ts` (new)

Exact task interface (`ThemeMode`, `theme/isDark/toggleTheme/setTheme/initTheme`)
plus `THEME_STORAGE_KEY = "baraka-theme"`. Every transition funnels through
`applyTheme`: `persistTheme` (localStorage, try-guarded) + `syncDocumentRoot`
(`documentElement.classList.toggle("dark", …)`, try-guarded for non-DOM harnesses).
`readStoredTheme` returns `"dark"` only on exact match, else `"light"`.

## 3. FAIL-THEN-PASS

`src/stores/theme-store.test.ts` (6 tests: init dark/light, toggle flips + persists
+ DOM sync, re-init survival simulating view transition/reload, 2 Topbar source
contracts). Faked localStorage/document (node env, resetModules per test).
- BEFORE: 6 failed (store module missing; Topbar owned `useDarkMode` + touched `document`).
- AFTER: `✓ theme-store.test.ts (6 tests)`; full `vitest`: **610 passed** (604 + 6).

## 4. GATES / CRLF

`grep -c $'\r'` before+after on all touched files: `0` (LF-only, zero drift).
1. `cargo fmt --check` → 0. 2. clippy → Finished dev profile, 0.
3. `cargo test --workspace` → 463/0 (unchanged, frontend-only).
4. lint → 0. 5. typecheck → 0. 6. build → 5.36s, 0. 7. vitest → 610 passed.

## 5. VISUAL (1280×720, stubbed invoke)

- `dsn065-1-pos-dark.png`: toggle click → `DARK_AFTER_TOGGLE=yes STORED=dark`.
- `dsn065-2-stock-dark.png`: after nav Stock → `DARK_ON_STOCK=yes`.
- `dsn065-3-pos-returned-dark.png`: back to Caisse → `DARK_BACK_POS=yes` (no reset).
- Reload: `DARK_AFTER_RELOAD=yes STORED=dark`; re-login → `DARK_AFTER_RELOGIN=yes`
  (initTheme). `VISUAL_DONE no-desync`. (Note: run 2 accidentally proved cold-start
  restore — a reused profile booted dark from storage before the baseline reset was
  added; script now resets to light first for determinism.)

## 6. FILES

- [CREATED] `D:/Dev/Baraka UltraPOS/src/stores/theme-store.ts`
- [MODIFIED] `D:/Dev/Baraka UltraPOS/src/components/pos/Topbar.tsx`
- [MODIFIED] `D:/Dev/Baraka UltraPOS/src/App.tsx`
- [CREATED] `D:/Dev/Baraka UltraPOS/src/stores/theme-store.test.ts`
- [CREATED] `D:/Dev/Baraka UltraPOS/audit/DSN-065/screenshots/dsn065-1-pos-dark.png`
- [CREATED] `D:/Dev/Baraka UltraPOS/audit/DSN-065/screenshots/dsn065-2-stock-dark.png`
- [CREATED] `D:/Dev/Baraka UltraPOS/audit/DSN-065/screenshots/dsn065-3-pos-returned-dark.png`
- [CREATED] `D:/Dev/Baraka UltraPOS/audit/DSN-065/REPORT.md`
- Ad-hoc `node_modules/.dsn065-visual.mjs` + profile — DELETED (NO_LEFTOVERS).

STATE complete / DONE store + refactor + init + tests + visuals / REMAINING none.
NEXT READY: YES
