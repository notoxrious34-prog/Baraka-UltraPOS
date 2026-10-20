# TASK-AUD-031 REPORT — Live End-to-End Systemic Verification (Session-022 Reply 10/11)

## 1. Working tree state

Before execution: ~38 tracked modifications carried over from DSN-072 / DSN-073 (uncommitted session stack) + untracked `audit/AUD-027/`, `audit/AUD-028/`, `audit/AUD-029/`, `audit/AUD-030/`, `audit/DSN-071/`, ADR-100 and new test files.
After execution (`git status --porcelain`, 45 tracked modified + 10 untracked paths): the same stack plus AUD-031 changes:

- [MODIFIED] `src/App.tsx` — session reactivity pin (309 lines, 309/309 CRLF preserved)
- [MODIFIED] `src/navigation-permissions.test.tsx` — reactivity pin test (LF, matches file convention)
- [MODIFIED] `src/features/stock/StockModule.tsx` — one-line `<bdi>`-in-`<option>` console-error fix (LF preserved)
- [CREATED] `audit/AUD-031/drill.mjs` — live CDP drill (29 steps)
- [CREATED] `audit/AUD-031/drill-results.json` — machine-readable drill evidence (29/29 pass)

Untouched per scope: migrations 0001–0015, `crates/**`, `src-tauri/**`, `.gitattributes`, live DB / `%APPDATA%/com.baraka.ultrapos`. No new production dependencies. Temporary probe scripts lived in `$TMPDIR` only (never in the repo). Vite dev server on :5173 was reused for the drill and left running.

## 2. Session reactivity verification (`src/App.tsx`)

Finding: `AppShell` selected only `getSessionToken` / `hasPermission` — both stable function references. Under zustand v5, `useSyncExternalStore` bails out on identical selector output, so `login()` / `logout()` `set()` calls produced no re-render: first paint after auth transition was stale until an unrelated render. (SSR/static markup additionally freezes value selectors, which is why the token-closure read was kept.)

Fix applied (AUD-031):

```tsx
// Reactive pin (AUD-031): `getSessionToken`/`hasPermission` are stable
// function references, so selecting them alone NEVER re-renders when the
// session object identity changes. Subscribing to `session` forces an
// immediate re-render on login/logout; the token check below covers
// SSR/static markup, where value selectors freeze on initial state.
useSessionStore((state) => state.session);
```

Live proof: drill step B1 (admin login → `<nav>` present within the same DOM session, no reload) PASSES, as do all four logout-return steps (B6/C5/D3/G5). Regression pin added to `src/navigation-permissions.test.tsx` (`App.tsx` must contain `useSessionStore((state) => state.session)`); suite 14/14.

## 3. Live CDP drill execution — 29/29 PASS

`node audit/AUD-031/drill.mjs` against `http://localhost:5173/` (headless Chrome, CDP :9224, `__TAURI_INTERNALS__.invoke` stub per DSN-056 idiom; exact wire shapes; per-username roles: admin→ADMIN/34, karim→GERANT/29, vendeur→VENDEUR/5, caissier→CAISSIER/13). Full output in `audit/AUD-031/drill-results.json`:

- A1 boot shows LoginScreen, no AppShell nav — PASS (`login=true nav=false`)
- B1 login renders AppShell immediately (reactivity proof) — PASS
- B2 rail shows all 9 entries — PASS (`9/9 present`)
- B3 ×9 view renders (Caisse, Catalogue, Stock & Dépôts, Clients & Crédit, Documents & Factures, Fournisseurs, Rapports, Paramètres, Utilisateurs & Accès) — PASS (`ok:1` each)
- B4 users view: 4 rows + last-admin badge + disabled control — PASS (`4 rows, badge ok, disabled=1`)
- B5 scroll clean on users view — PASS (`sw=1280 cw=1280 sh=800 ch=800`)
- B6 logout returns to LoginScreen — PASS
- C1 vendeur rail: 5 entries, 4 absent — PASS (`5 present, 4 absent`)
- C2 customers module: real Vendeur sees module-level unauthorized — PASS (see §4 note)
- C3 POS customer search lists directory row — PASS (`search wiring live`)
- C4 payment modal: vendeur inputs disabled with guidance + scroll clean — PASS (`notice ok, title ok, scroll clean, modal closed`; submit title contains guidance `réservé à la caisse`; `Rôle Vendeur` notice shown)
- C5 logout returns to LoginScreen — PASS
- D1 caissier payment modal enabled — PASS (`payment open, inputs enabled, modal closed`; no `Rôle Vendeur` notice)
- D2 customers module: real Caissier sees module-level unauthorized — PASS (see §4 note)
- D3 logout returns to LoginScreen — PASS
- G1 gerant rail: settings present, users absent — PASS (`7 present, users absent`)
- G2 gerant: Nouveau Client enabled (CREATE_CUSTOMER held) — PASS (`enabled`)
- G3 gerant: credit limit input enabled (MANAGE_CUSTOMER_CREDIT held) — PASS (`limit enabled for gerant`)
- G4 gerant: payment modal enabled (RECORD_CREDIT_PAYMENT held) — PASS (`payment enabled, modal closed`)
- G5 logout returns to LoginScreen — PASS
- Z1 zero console errors and zero page exceptions — PASS (`clean`)

C1-script issues fixed in `drill.mjs`: B5 `SCROLL_FN` → `SCROLL_EXPR`; G3/C-flow `clickTextExpr("Modifier")` → real label `Modifier la fiche` (probe-verified); POS client button `Client Comptant\nF2` requires includes-match (`clickIncludesExpr`); logout navigates via Caisse first (Déconnexion lives in POS Topbar); payment modals closed via `Annuler` instead of reload (session store is in-memory by design — reload correctly returns to LoginScreen).

## 4. Scroll dimensions, console hygiene, and matrix-vs-module finding

- Scroll: `scrollWidth<=clientWidth && scrollHeight<=clientHeight` asserted on users view (B5: 1280×800 exact) and vendeur payment modal (C4). All PASS — zero page scrollbars.
- Console: Z1 asserts zero `Console.messageAdded level=error` and zero `Runtime.exceptionThrown` across all four role flows — PASS. The drill caught and fixed one real defect: `StockModule.tsx:162-164` rendered `<bdi dir="ltr">` inside `<option>`, producing a React `validateDOMNesting` console error on the stock view. Replaced with plain text (`{warehouse.code} · {warehouse.name}`); `<option>` children must be text. One-line, behavior-identical, LF preserved.
- Spec-vs-reality (no guessing, engine-cited): the task text assumed real Vendeur holds `CREATE_CUSTOMER`, but `crates/domain/src/role.rs:121` gives Vendeur exactly `[PosSell, PosParkResume, ViewStock, CreateSale, ViewCustomers]` (no `CreateCustomer`; enforced `customers.rs:87,164`). Additionally `CustomersModule.tsx:64-67` still gates the whole module on `isManagerRole` (ADMIN/GERANT) — AUD-030 row 12, Batch 2b scope. Consequence, asserted live: real Vendeur and real Caissier see `Non autorisé pour votre rôle` on the customers view (C2/D2 PASS as documented behavior). DSN-073 component gates were verified live where reachable (Gerant: G2/G3/G4) and at unit level otherwise (`customers-permissions.test.ts` 13/13, `record-payment-wire.test.ts` 19/19).

## 5. `docs/architecture/tools/build-boot-package.mjs` — read-only inspection

126 lines, Node built-ins only (`fs`, `path`, `crypto`, `fileURLToPath`), no timestamps, no env-dependent content. Deterministic boot-package generator:

- Arguments: `--charter <p>` (default `docs/architecture/CHARTER-v2.0.md`), `--ctp <p>` (default `docs/architecture/CTP-SESSION-ARC-002.md`), `--out <p>` (default `docs/architecture/BOOT-SESSION-ARC-002.md`), `--check` (verify-only: exit 0 `CHECK: OK` / exit 1 `CHECK: DIFFERENT`), `--prev1/--prev2 <p>` (explicit prior CTPs), `--history <n>` (default `2`: auto-discovers `CTP-SESSION-ARC-(N-k).md`, missing files skipped gracefully). Unknown/missing values fall back to defaults; `--prev` overrides auto-discovery per slot.
- Concatenation order (byte-for-byte, LF separator appended only when a part lacks trailing LF, counted in `added separator LFs` log): PREAMBLE → PART 1 CHARTER → PART 2 current CTP → PART 3+ priors, wrapped in `=====BEGIN/END-PART-…=====` markers. Session tag derived from CTP basename (`CTP-SESSION-ARC-003.md` → `ARC-003` → `SESSION-ARC-003`; non-matching basename falls back to `ARC-002`).
- Output path: `--out` value (default `docs/architecture/BOOT-SESSION-ARC-002.md`); logs byte sizes + sha256 of every input and the output.
- Invocation: `node docs/architecture/tools/build-boot-package.mjs [--charter <p>] [--ctp <p>] [--out <p>] [--check] [--prev1 <p>] [--prev2 <p>] [--history <n>]`. File was inspected read-only; not executed, not modified.

## 6. Quality Gates G1–G7 (verbatim)

- G1 `cargo fmt --check` → `G1 CLEAN` (exit 0, no diff output)
- G2 `CARGO_BUILD_JOBS=2 cargo clippy --all-targets --all-features -- -D warnings` → `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 1.08s` (exit 0)
- G3 `CARGO_BUILD_JOBS=2 cargo test --workspace -j 2` → `G3 passed=464 failed=0` (exit 0)
- G4 `npm run lint` → exit 0, zero `error` matches in output (`G4 exit=0`)
- G5 `npm run typecheck` → exit 0, zero `error` matches in output (`G5 exit=0`)
- G6 `npm run build` → `dist/assets/index-BuTqDlAP.js 728.47 kB │ gzip: 187.74 kB` / `✓ built in 5.78s` (exit 0)
- G7 `npx vitest run` → `Test Files 71 passed (71)` / `Tests 675 passed (675)` / `Duration 16.26s` (exit 0; +1 vs 674 = new AUD-031 reactivity pin test)

Line endings: `src/App.tsx` 309/309 CR (HEAD blob 174/174 CR — CRLF convention preserved through DSN-072 expansion + AUD-031 pin); `src/lib/i18n.ts` 2320/2320 CR (HEAD 2300/2300 CR); `src/features/stock/StockModule.tsx` 0 CR worktree vs 0 CR HEAD (LF preserved); `src/navigation-permissions.test.tsx` LF (matches its convention); `audit/AUD-031/*` LF (new files, DSN-071 precedent).

NEXT READY: YES (CTP-SESSION-ARC-022 Reply-10 update and Reply-12 transfer package remain architect-side).
