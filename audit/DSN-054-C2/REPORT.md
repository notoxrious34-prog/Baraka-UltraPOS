# TASK-DSN-054-C2 REPORT — Wire `suppliers_list` (default list + teaching state)

## 1. STEP 0 FINDINGS (verbatim, pre-implementation)
1. **Screen** `src/features/suppliers/SupplierListScreen.tsx` (export `SuppliersModule`, 377 lines).
   State: `suppliers/selected/loading/storeError` from `useSupplierStore`, local
   `query/panel/overlay/editing/deleting/deleteNote`, `token = session?.token ?? null`
   (:29-43). Debounced effect (:48-58) calls `search(token, query)` ~150 ms.
   (a) Empty query: store returns `[]` (backend guard) → :225-227 renders
   `<p className="text-sm font-bold text-slate-500">{t(locale, "sup.noResults")}</p>`
   ("Aucun fournisseur — lancez une recherche") — a dead-end paragraph, nothing fetched.
   (b) Non-empty + zero rows: no-match panel (:187-221, `query.trim() !== ""`):
   store-icon svg 28px `text-line-strong`, `p.text-sm.font-bold.text-ink-secondary`
   with `sup.noMatchLead` + guillemets + `<bdi className="tabular-nums">`, clear-search
   button `sup.clearSearch` → `setQuery("")`.
   (c) Non-empty + rows: table (:228-300) name/phone/balance(`formatMoney`)/active-pill/actions.
2. **Call site is NOT a component ternary.** Component calls `search(token, query)`
   (:53, :65). The IPC call lives in the Zustand store
   `src/stores/supplier-store.ts:38`: `await ipcSearchSuppliers(sessionToken, query, 50)`.
   Insertion point = store `search` action (in SCOPE-IN) — no shared hook touched.
3. **Customers precedent** `CustomerDossierPane.tsx:53-90` (`detail === null`):
   section flex-centered p-8; users-icon svg 36px 1.75 `text-line-strong`;
   `h2.text-base.font-bold.text-ink-primary` ("Sélectionnez un client" /
   "اختر زبونًا"); `p.max-w-md.text-sm.font-bold.text-ink-secondary` (FR body / AR body);
   CTA `button.h-10.bg-accent-500.text-slate-950.hover:bg-accent-400` ("Nouveau client" /
   "زبون جديد", `onNew`). Mirrored with supplier nouns/CTA only.
4. **Gating exists, no new check needed.** `src/App.tsx:121`
   `const manager = session !== null && isManagerRole(session.role);`
   (`isManagerRole` = GERANT||ADMIN, `warehouse-transfer-store.ts:67-69`);
   NavRail only when manager (:134); `view === "suppliers" && manager` (:142-143).
   Vendeur/Caissier cannot reach the screen. NOT blocked.
5. **SupplierView** `src/types/supplier.ts:11-24`: id, name, phone?, address?, nif?, rc?,
   notes?, isActive, currentBalanceCentimes (centimes STRING, may be negative),
   originDeviceId, createdAt, updatedAt. Money: `formatMoney` (`src/lib/money.ts:44`,
   bigint, Latin digits, GROUP=" " regular space, `DZD`/د.ج suffix) — reused, none written.

BLOCKED — ARCHITECTURE MISMATCH: none found. BLOCKED — PRE-EXISTING GAP: none found.
Scope note (declared): teaching copy required 3 i18n keys (`sup.emptyTitle/Body/Cta`,
FR+AR) in `src/lib/i18n.ts` — catalog-only addition, no signature change; glossary untouched.

## 2. BEFORE (FAIL state)
`audit/DSN-054-C2/screenshots/dsn054c2-BEFORE-empty-query-broken-state.png`:
empty query renders only "Aucun fournisseur — lancez une recherche"
(measured `before-text=4.55/4.5/PASS`, SCROLL 1280/1280/720/720) — a dead end:
no list attempted, no teaching, no CTA. Captured pre-fix via
`audit/DSN-054-C2/scripts/capture.mjs` (MODE=before).

## 3. IMPLEMENTATION (exact diff)
- `src/stores/supplier-store.ts`: import `ipcListSuppliers`; `search` action now
  `query.trim() === "" ? await ipcListSuppliers(sessionToken) : await ipcSearchSuppliers(sessionToken, query, 50)`.
- `src/lib/i18n.ts` (+6 lines, CRLF preserved): FR `sup.emptyTitle` "Ajoutez votre
  premier fournisseur" / Body "Aucun fournisseur enregistré. Ajoutez votre premier
  fournisseur pour suivre vos achats, factures et soldes." / CTA "+ Nouveau fournisseur";
  AR "أضف أول مورّد" / "لا يوجد مورّدون مسجلون. أضف مورّدك الأول لتتبع المشتريات
  والفواتير والأرصدة." / "+ مورّد جديد".
- `src/features/suppliers/SupplierListScreen.tsx`: empty-query branch replaced by teaching
  state (store-icon 36px, h2 `text-ink-primary`, p `text-ink-secondary`, CTA `bg-accent-500
  text-slate-950` → `setOverlay("create")`); balance cell wrapped in `<bdi dir="ltr">`
  (parent keeps `font-mono tabular-nums`).
- `src/features/suppliers/supplier-empty-states.test.ts`: +3 regression tests (a/b/c).
- Token grep on added lines: only `text-ink-primary/secondary`, `text-line-strong`,
  `bg-accent-500/hover:bg-accent-400` — plus `text-slate-950`, a byte-exact mirror of the
  APPROVED customers teaching CTA (`CustomerDossierPane.tsx:84`), kept deliberately for
  precedent identity (declared, not hidden). No new token, no ADR.

## 4. AFTER (PASS) — 6 screenshots + computations
- `dsn054c2-suppliers-zero-db-teaching-light.png`:
  teach-title=17.06/4.5/PASS, teach-body=9.9/4.5/PASS, teach-cta=14.48/4.5/PASS; SCROLL clean.
- `dsn054c2-suppliers-zero-db-teaching-dark.png`:
  teach-title=17.19/4.5/PASS, teach-body=7.34/4.5/PASS, teach-cta=11.87/4.5/PASS; SCROLL clean.
  (Dark visually verified: centered store icon, "Ajoutez votre premier fournisseur",
  2-line body, orange CTA, zero defects.)
- `dsn054c2-suppliers-default-list-light.png`:
  row-name/pos/neg/zero = 17.06/4.5/PASS ×4; 5 rows visible
  (Quincaillerie El Amel 15 250,00 DZD; Ets Benali & Frères -4 250,75 DZD;
  Sarl Atlas Négoce 0,00 DZD; Grossiste Sidi Moussa 9 800,00 DZD;
  Comptoir du Bâtiment -120,00 DZD); SCROLL clean. (Visually verified, zero defects.)
- `dsn054c2-suppliers-default-list-dark.png`: same 4 keys = 17.19/4.5/PASS; SCROLL clean.
- `dsn054c2-suppliers-nomatch-recheck-light.png`: nomatch-lead=9.9/4.5/PASS.
- `dsn054c2-suppliers-nomatch-recheck-dark.png`: nomatch-lead=7.34/4.5/PASS.
- No-match byte-identical: DSN-055 REPORT line 34 recorded "Suppliers no-match body:
  1.18 → 7.34"; recheck dark = 7.34 (exact), light 9.9 = token spec (ink-secondary/canvas
  9.90). Region :187-221 re-read post-fix, character-identical to Step 0.

## 5. MONEY PROOF (rendered DOM, MODE=markup run)
`BALANCE_CELL_MARKUP=<bdi dir="ltr">15 250,00 DZD</bdi>` — Latin digits, regular-space
thousands (GROUP=" "), `dir="ltr"`, parent td `font-mono tabular-nums`. Negative
(`-4 250,75 DZD`) and zero (`0,00 DZD`) needles found in DOM during list captures.
`EMOJI_SCAN=emoji-clean` (Extended_Pictographic regex over full DOM).
`FIXED_SCAN=no-fixed` (all section div/p/h2/button/table).

## 6. REGRESSION TESTS — FAIL-then-PASS (verbatim)
FAIL (pre-fix, new tests only): `12 tests | 2 failed` —
× (a) teaching state, × (b) store ipcListSuppliers; ✓ (c) no-match intact.
PASS (post-fix): `Test Files 1 passed (1) / Tests 12 passed (12)`.
Full suite total stated in §7.

## 7. GATES (final pass, verbatim)
```
FMT_OK
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.67s
CLIPPY_OK
CARGO_PASSED=463 FAILED=0
npm notice run eslint . --ext ts,tsx --report-unused-disable-directives --max-warnings 0
LINT_OK
npm notice run tsc --noEmit
TYPECHECK_OK
✓ built in 10.99s
BUILD_OK
 Test Files  59 passed (59)
      Tests  600 passed (600)
```
Rust 463/0 unchanged (no Rust touched ✓). Vitest 597 + 3 new = 600/600.

## 8. SCOPE + CRLF
`git status --porcelain` (C2 entries only; prior-task entries omitted for brevity,
full listing available via git):
```
M src/features/suppliers/SupplierListScreen.tsx
M src/lib/i18n.ts
M src/stores/supplier-store.ts
M src/features/suppliers/supplier-empty-states.test.ts
?? audit/DSN-054-C2/
```
Zero edits outside SCOPE-IN (+ declared 3-key i18n catalog addition, §1).
No-match JSX untouched (§4). `src/lib/ipc.ts` unmodified (binding imported only).
No Rust file touched. No new dependency.
CRLF/LF (`file` before → after): supplier-store.ts ASCII→ASCII;
SupplierListScreen.tsx UTF-8(LF)→UTF-8(LF); supplier-empty-states.test.ts
UTF-8→UTF-8; i18n.ts "with CRLF line terminators"→"with CRLF line terminators"
(diff +6/-0 on i18n.ts); capture.mjs new LF. No drift.

HANDOFF: complete. NEXT READY: YES.
