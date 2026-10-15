# FORENSIC AUDIT REPORT — Baraka UltraPOS v1.0.0-rc.6

Task: TASK-REL-055 (+C1) · Date: 2026-10-15 (Africa/Algiers) · Baseline: rc.6 + `11c7fc8`
Scope: `src/features/catalog/` (CSV repair), `crates/infrastructure/tests/forensic_full_system_drill.rs` (NEW),
`docs/operations/FORENSIC-AUDIT-REPORT-v1.0.0.md` (this file). No migration, `.gitattributes`, live-DB, or dependency touched.
Presentation markup untouched (visual fixes deferred to TASK-DSN-048 per scope).

## 1. Executive summary & CSV bug resolution

**Field symptom (owner-verified on rc.6):** "Télécharger le modèle CSV standard" (import modal) and
"Exporter CSV" (catalog header) silently do nothing in desktop WebView2 — no file, no error, no feedback.

**Root cause (verified in source, both callers identical pattern):**
`triggerCsvDownload` (`src/features/catalog/catalog-export.ts`) and `downloadSampleCsvTemplate`
(`src/features/catalog/catalog-import-parser.ts`) built a `blob:` URL, dispatched the anchor click, then called
`URL.revokeObjectURL(url)` **synchronously in the same task** (`finally` block / next statement).
WebView2 picks up blob downloads **asynchronously**, so the revocation voids the URL before the download
pipeline grabs it — the click lands on a dead URL. A second, UX-level defect compounded it: even a successful
pickup shows no WebView2 download UI in the Tauri shell, so the user could never tell what happened.

**Fix (no new dependency; `plugin-dialog`/`plugin-fs` are not installed and no IPC save command was in scope):**
- NEW `src/features/catalog/desktop-download.ts` — single hardened path `downloadTextFile(content, filename, mime)`:
  anchor stays attached for the `dispatchEvent(new MouseEvent("click", …))`; revocation deferred
  `DOWNLOAD_URL_LIFETIME_MS = 30_000` (wins the async-pickup race on low-end hardware, never leaks);
  returns `{ ok, filename, reason }` (`started` | `no-dom` | `trigger-failed`); SSR/node-guarded.
- `triggerCsvDownload` delegates (signature kept `void` for callers); `downloadSampleCsvTemplate` returns the outcome.
- Instant inline confirmation in both UIs with **zero new authored copy** (UI-string law):
  `ProductImportModal.tsx` `templateNote` state + `CatalogModule.tsx` `exportNote` prop on `CatalogHubView`,
  both rendering a `role="status"` emerald pill with `✓ {filename}`, auto-dismissed after 8 s.
- Both payloads keep UTF-8 BOM (`\uFEFF` first char) + `;` separator, so Excel opens Arabic text flawlessly.

**Proof:** 146/146 catalog Vitest tests pass, including 4 new `__tests__/desktop-download.test.ts` tests that pin
the contract with a DOM/URL double (click dispatched, **zero synchronous revocations**, deferred release at the
documented lifetime, outcome echoes filename) plus BOM/header assertions for both flows.

## 2. Ten-pillar forensic drill — exact output

`CARGO_BUILD_JOBS=2 cargo test -p baraka-infrastructure --test forensic_full_system_drill -j 2`:

```
running 11 tests
test pillar_01_caisse_pos_sale_pipeline ... ok
test pillar_01b_tier_pricing_and_margin_guard ... ok
test pillar_02_stock_depots_adjustment ... ok
test pillar_03_clients_credit_lifecycle ... ok
test pillar_04_documents_factures_and_return ... ok
test pillar_05_fournisseurs_achats ... ok
test pillar_06_rapports ... ok
test pillar_07_catalogue_import ... ok
test pillar_08_caisse_session_lifecycle ... ok
test pillar_09_utilisateurs_rbac ... ok
test pillar_10_parametres_sauvegarde ... ok
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.38s
```

Pillar evidence (each on an isolated scratch DB; all money `i64` centimes, quantities 10⁴ micros):

| Pillar | Core assertions observed |
|---|---|
| 1 Caisse POS | TICKET 2×10 000 → TTC **20 500** (HT 20 000 + 500 timbre floor, TVA 0 unseeded), change 3 300 on 23 800; stock 100→98 units; 1 doc/item/payment/movement; 1 PENDING print job |
| 1b Tier + margin | Wholesale customer with NULL wholesale price falls back to retail (BL TTC 10 000, no timbre on BL); margin (10 000, 8 000) = **(2 000 bps, false)**; loss vector (8 000, 10 000) = (−2 500, true) flagged |
| 2 Stock | DEP-01 listed; `Regularisation` +25 units → 1 250 000 micro; reason persisted on movement |
| 3 Clients | Credit BL posts debt 10 000; payment 1 900 → 8 100; overpayment 99 999 rejected `PaymentExceedsBalance{balance 8 100}` with zero ledger writes; statement ≥ 2 entries |
| 4 Documents | `get_by_number(TICKET, year, number)` resolves; PENDING → DUPLICATA reprint (2 jobs); 1-unit NoRefund return → AVOIR, refund TTC 10 000 (**timbre not refunded**), stock +10 000 micro |
| 5 Fournisseurs | Supplier created; invoice 10×8 000 = 80 000; statement non-empty; balance 80 000 |
| 6 Rapports | Daily summary revenue **20 500** / 1 doc; by-product revenue 20 000 (line sums exclude header timbre) / 20 000 micro; outstanding readable |
| 7 Catalogue | `ImportProductsUseCase` 2 rows → `imported_count 2`; both SKUs POS-searchable; 1 stock level (provisioned row only) |
| 8 Session | OPEN → expense −50 000 (`EXPENSE_OUT` persisted) → mid report non-final → begin/confirm close at 50 000 → CLOSED + final Z-report |
| 9 Utilisateurs | PIN `1234` ok / `0000` → `InvalidCredentials`; 4 operators; Vendeur supplier-create → `Unauthorized`, 0 writes; last-admin deactivation → `Conflict` |
| 10 Paramètres | Identity round-trips identically; WAL backup artifact exists, non-empty, integrity verified |

Engine rules discovered and pinned (not assumed): timbre = TICKET/FACTURE + cash tender only,
`compute_timbre` floor 500 (`pricing.rs:172-194`); CREDIT legs must tender exactly (`ChangeNotPermittedForNonCash`);
credit payments resolve the OPEN session by terminal id; close requires begin→confirm handshake
(`CashSessionNotInClosing`); margin is computed on price (`(15000-10000)*10000/15000 = 3333 bps`).

## 3. Visual friction analysis — blueprint for TASK-DSN-048 (no markup changed here)

1. **Caisse duplicate header (~56 px waste).** `src/App.tsx:54` renders the manager `NavRail`
   (`flex h-10 … bg-primary-900`, Baraka logo + `Baraka` wordmark + module entries). Inside Caisse,
   `src/components/pos/Topbar.tsx:85` renders a **second** brand header
   (`flex h-14 … bg-primary-900`, logo + `app.title` split + `app.subtitle`) above the same operational
   controls (session indicator, warehouse select, lang/logout). Action for DSN-048: collapse to one bar —
   keep NavRail for navigation, strip the brand block from Topbar (or vice versa), reclaiming the `h-14` row.
2. **Tile edit clutter.** `src/components/pos/ProductDiscovery.tsx:238-252` — every POS tile carries
   `✎ {t(locale, "editProduct")}` (`editProduct` = "Modifier", `src/lib/i18n.ts:516` / "تعديل" `:1459`),
   gated by `canEditCatalog`. Action: remove per-tile edit affordance from the fast-sale grid (edit stays in
   Catalogue module); tiles keep tap-to-add only.
3. **Catalog double scrollbars.** Nested same-axis scroll containers: `src/features/catalog/CatalogModule.tsx:873`
   section (`flex h-full flex-col gap-3 overflow-y-auto p-4`) wraps `CatalogModule.tsx:532` table wrapper
   (`min-h-0 flex-1 overflow-y-auto`, `w-full border-collapse text-xs` with wide `whitespace-nowrap` cells);
   horizontally, `src/features/catalog/CategoryPillStrip.tsx:52` (`flex … overflow-x-auto` with `shrink-0` pills)
   sits inside the same section, stacking its bar against the section's. Action: single scroll owner per axis
   (sticky `thead` already present `:534`), constrain table `min-width` with one `overflow-x-auto` wrapper,
   let pills shrink/wrap instead of scrolling.

## 4. Sign-off

- Drill: **11/11 green** (`forensic_full_system_drill`), catalog Vitest **146/146** (full-suite G6 count in gates).
- Full quality gates G1–G7 executed (see executor report); commit baseline below.
- Residual honesty: WebView2 pickup verified by contract + code reasoning (deferred lifecycle), not by a
  live WebView2 click harness — owner re-verification on desktop hardware closes that loop; the inline
  filename note makes success/failure self-evident either way.

NEXT READY: YES (visual fixes scoped to TASK-DSN-048).
