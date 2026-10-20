# DSN-071 Completion Report — Users Module Token Harmonization, RTL Isolation, Permission Gating

## 1. Summary

- Nav + route gating migrated from `role === "ADMIN"` to `hasPermission("USER_MANAGE")`
  (`src/App.tsx` NavRail + `App` view switch; unauthorized card kept, tokenized).
- All 5 `src/features/users/` sources tokenized; grep confirms zero `slate-*`/`gray-*`.
- Last-admin proactive protection: table badge + disabled Edit (`users.lastAdminTitle`),
  form-level `isProtected` lock (role select + active checkbox + badge).
- RTL: `<bdi dir="ltr">` on usernames/dates + new `users.resetTarget` line; title uses
  LRI/PDI (`String.fromCharCode(0x2066/0x2069)`) because `ModalShell` takes a plain-string
  title (shared shell untouched per scope).
- i18n: 3 new keys × FR/AR (`resetTarget`, `lastAdminBadge`, `lastAdminTitle`).
- 4 screenshots captured at 1280×800 via fixed CDP harness; 20 contrast pairings measured,
  all ≥ 4.5:1 both themes; `scrollWidth === clientWidth === 1280`,
  `scrollHeight === clientHeight === 800` on all 4 shots (zero page scroll).

## 2. Contrast table (browser-computed, `getComputedStyle`)

| Pairing | Light | Dark | AA |
|---|---|---|---|
| title h1 (ink-primary/canvas) | 17.06 | 17.19 | PASS |
| subtitle (ink-muted/canvas) | 4.55 | 7.34 | PASS |
| table th (ink-secondary/sunken) | 9.20 | 7.47 | PASS |
| username cell (ink-primary/canvas) | 17.06 | 17.19 | PASS |
| badge ADMIN (ink-primary/sunken) | 15.87 | 17.49 | PASS |
| badge GERANT (info-fg/bg) | 5.49 | 5.93 | PASS |
| badge CAISSIER (warning-fg/bg) | 4.51 | 8.30 | PASS |
| badge VENDEUR (success-fg/bg) | 4.57 | 8.00 | PASS |
| badge last-admin (warning-fg/bg) | 4.51 | 8.30 | PASS |
| pill active (success-fg/bg) | 4.57 | 8.00 | PASS |
| pill inactive (ink-secondary/sunken) | 9.20 | 7.47 | PASS |
| PIN pill (ink-secondary/sunken) | 9.20 | 7.47 | PASS |
| created cell (ink-muted/canvas) | 4.55 | 7.34 | PASS |
| HUD value (ink-primary/raised) | 17.85 | 16.19 | PASS |
| btn new-user (ink-inverse/ink-primary) | 17.06 | 16.30 | PASS |
| btn edit (ink-primary/sunken) | 15.87 | 17.49 | PASS |
| btn credentials (warning-fg/bg) | 4.51 | 8.30 | PASS |
| modal input (ink-primary/raised) | 17.85 | 16.19 | PASS |
| modal save (ink-inverse/ink-primary) | 17.06 | 16.30 | PASS |
| modal cancel (ink-primary/sunken) | 15.87 | 17.49 | PASS |

Minimum over all 40 measurements: **4.51** (light CAISSIER/last-admin/credentials) — passes AA.
Modal input/save/cancel measured on the open modal shots (create-dark, reset-light).
Deliberate deviation: inactive pill uses `text-ink-secondary`, not spec's `text-ink-muted`
(muted-on-sunken = 4.23:1 light, AA fail); measured 9.20.

## 3. Scroll assertions (all 4 shots)

- light: `scroll=1280x800 client=1280x800 dark=false`
- dark: `scroll=1280x800 client=1280x800 dark=true`
- createDark: `scroll=1280x800 client=1280x800 dark=true`
- resetLight: `scroll=1280x800 client=1280x800 dark=false`
- `hasPageScrollX === false`, `hasPageScrollY === false` everywhere.

## 4. Screenshots (all verified by vision inspection)

1. `users-module-light.png` (76 KB) — table, 4 HUD cards (4/1/2/1), admin row carries
   `Administrateur` + `Dernier Admin` badges, admin `Modifier` visibly dimmed; no overflow.
2. `users-module-dark.png` (76 KB) — consistent dark surfaces, no light leaks, badges legible.
3. `users-modal-create-dark.png` (81 KB) — `Nouvel Utilisateur`, 4 fields dark, `Enregistrer`
   correctly inactive on empty form. (The `caisse1` text in the username box is the
   `placeholder="caisse1"` attribute, not a pre-filled value.)
4. `users-modal-reset-light.png` (91 KB) — `Réinitialiser les Identifiants de admin` +
   `Compte concerné : admin` target line, both inputs empty, save disabled.

Harness notes: capture stub replaces `window.__TAURI_INTERNALS__` wholesale
(`audit/DSN-055` precedent); responses match exact wire shapes (`LoginWire` with
`permissions`, `users_list` → array, `warehouses_list` → `{warehouses}`, `cash` → null).
First-attempt failure was the fetch-interceptor transport (`@tauri-apps/api` v2 browser
build uses no `ipc.localhost` fetch URL) plus missing post-login `warehouses_list` /
`cash_get_active_session` mocks (login rejected → nav timeout). Theme driven via
`documentElement.classList` because Topbar (theme toggle host) is not mounted on the
users view — same Tailwind class the store toggles.

## 5. Line endings (byte-counted, `tr -cd '\r'`)

| File | HEAD | Worktree | Verdict |
|---|---|---|---|
| src/App.tsx | 174/174 CRLF | 176/176 CRLF | pure CRLF preserved |
| src/lib/i18n.ts | 2300/2300 CRLF | 2306/2306 CRLF | pure CRLF preserved |
| users/UsersModule.tsx | 0 CR (LF) | 0 CR (LF) | LF preserved |
| users/UserListTable.tsx | 0 CR | 0 CR | LF preserved |
| users/UserFormModal.tsx | 0 CR | 0 CR | LF preserved |
| users/CredentialResetModal.tsx | 0 CR | 0 CR | LF preserved |
| users/user-helpers.ts | 0 CR | 0 CR | LF preserved |
| users/users-module.test.ts | 0 CR | 0 CR | LF preserved |
| audit/DSN-071/capture.mjs | new | 0 CR (LF) | — |

No normalization was required: `App.tsx` edited lines landed CRLF (174→176 = the two
added lines only).

## 6. Scope notes

- `src/lib/i18n.ts` touched (3 keys × 2 locales) — required by the localized badge/title
  spec; not in SCOPE-OUT.
- Modal backdrop left as `ModalShell` default (`bg-slate-950/60`): `ModalShell.tsx` is
  shared and out of scope; spec's `bg-black/50` not applied. No `position: fixed` added.
- `accent-600/700` do not exist in `tailwind.config.js` (only `accent.500`); primary
  buttons use spec's alternative `bg-ink-primary text-ink-inverse` (17.06/16.30).
- Other `role ===` gates outside `App.tsx` untouched (AUD-028 rows 4–20 pending).

## 7. Quality gates (verbatim)

### G1 `cargo fmt --check`
```
(empty output — clean)
G1_EXIT:0
```
### G2 `cargo clippy --all-targets --all-features -- -D warnings`
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.05s
G2_EXIT:0
```
### G3 `cargo test --workspace -j 2`
Aggregate over all 58 `test result` lines: **464 passed, 0 failed** (5 ignored —
pre-existing: hardware p95 drill ×1, release benches ×3, release drill ×1).
`G3_EXIT:0`. Full per-suite log pasted in the executor handoff; zero `FAILED` lines.
### G4 `npm run lint`
```
npm notice run baraka-ultrapos@1.0.0 lint
npm notice run eslint . --ext ts,tsx --report-unused-disable-directives --max-warnings 0
G4_EXIT:0
```
### G5 `npm run typecheck`
```
npm notice run baraka-ultrapos@1.0.0 typecheck
npm notice run tsc --noEmit
G5_EXIT:0
```
### G6 `npm run build`
```
vite v5.4.21 building for production...
transforming...
✓ 168 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                              0.51 kB │ gzip:   0.33 kB
dist/assets/baraka-logo-icon-CFLRGFSQ.png   31.80 kB
dist/assets/baraka-logo-CUnvKflC.png       323.86 kB
dist/assets/index-r3liXAGn.css              46.16 kB │ gzip:   8.22 kB
dist/assets/index-B6SSQNbj.js              726.16 kB │ gzip: 187.00 kB
✓ built in 5.35s
G6_EXIT:0
```
### G7 `npx vitest run`
```
Test Files  69 passed (69)
      Tests  649 passed (649)
   Start at  17:08:15
   Duration  15.10s (transform 2.13s, setup 0ms, collect 7.04s, tests 2.01s, environment 24ms, prepare 13.48s)
G7_EXIT:0
```
(users-module.test.ts: 32/32 within the 649.)
