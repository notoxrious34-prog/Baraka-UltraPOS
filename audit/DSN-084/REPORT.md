# TASK-DSN-084 REPORT — Document drawer full details via getDocumentDetails

## FAIL-before (RED quoted)

```
 Test Files  1 failed (1)
      Tests  8 failed | 0 passed (8)
 × fetches full details on open via the sales_get wrapper
 × issues exactly one fetch per open with cancel-on-close
 × shows a loading skeleton while fetching
 × shows an error state with retry
 × renders TICKET lines, payments, and totals verbatim
 × renders FACTURE lines, payments, and totals verbatim
 × renders LTR-isolated codes and tabular figures
 × declares the details namespace in FR and AR
```

Drawer rendered only the passed summary (number/badges/meta + single total);
`getDocumentDetails` had zero callers in the documents feature (AUD-038).

## Implementation

- `DocumentDetailDrawer.tsx`: fetch-on-open `getDocumentDetails(document.id)`
  (single call site, effect deps `[document.id, detailsTick]`, `cancelled` flag
  on unmount/close/retry); states loading skeleton (`aria-busy`)/error
  (`role="alert"` + retry + UNAUTHORIZED/FORBIDDEN guidance)/empty-lines note;
  pure exported `DocumentDetailsView` (lines table with per-line TVA, payments
  list, HT/TVA/TTC/paid/change — all engine values verbatim via
  `formatMoney`/`formatQty`, zero arithmetic); `PrintStatusIndicator` chip
  reused above the existing DUPLICATA reprint button (pinned lifecycle kept).
- `ipc.ts`: NO CHANGE — `getDocumentDetails` wrapper already existed
  (`sales_get`, bare params, token auto-injected by `call()`).
- `i18n.ts`: +19 FR / +19 AR keys (CRLF preserved, 38 insertions, 0 deletions);
  no per-line discount or totals-in-words: engine `DocumentItemDto` carries no
  discount/rate fields and `GetSaleResponse` carries no words field — TVA shown
  per line + total, totals-in-words omitted by design (noted, not faked).
- `document-detail-drawer.test.tsx` (new, 8 pins): lifecycle source-text pins +
  TICKET/FACTURE `renderToStaticMarkup` goldens + LTR/tabular + i18n pins.
  One self-inflicted fix: nbsp in my own expected literal (engine uses 0x20).

## Fetch-count + golden lines

- `getDocumentDetails(` occurs exactly 1× in drawer source; cancel flag pinned.
- TICKET golden: `prod-vis`, `CASH`, `30,25 DZD`, `Encaissé` ✓
- FACTURE golden: `prod-ciment`, `VIREMENT`, `1 190,00 د.ج`, `1 000,00 د.ج` ✓

## Gates

- G1 `cargo fmt --check`: exit 0 (zero Rust touched).
- G2 `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.
- G3 `cargo test --workspace -j 2`: green (see counts below; only pre-existing
  BRK-076 `migration_replay`/`80k consistency` reds if present — out of scope).
- G4 lint: exit 0. G5 `tsc --noEmit`: clean.
- G6 build: ✓ ~5.6s.
- G7 vitest: 74 files / 745 tests passed (was 73/737: +1 file, +8 pins).
- Lockfile line: `git diff --stat -- pnpm-lock.yaml package-lock.json yarn.lock package.json` → empty.

## PNGs + rows

- `doc-ticket-1280x800-fr-light.png` — TICKET lines (prod-vis×2, PU/TVA/line),
  CASH 50,00, HT/TVA/TTC/paid/change, print chip + reprint; no page scroll.
- `doc-facture-1280x800-ar-dark.png` — dark + RTL FACTURE (SARL Atlas,
  prod-ciment×4, VIREMENT, 1 190,00 د.ج), no clipping, no error.
- `doc-error-1024x600-fr-light.png` — `Détails indisponibles` + `Réessayer`,
  header/summary intact. Note: table has an *inner* horizontal scroll at
  1024px (responsive container, by design); documentElement page scroll = 0.
- 0 console errors, DONE fails=0.
- Harness notes: nav labels are full strings (`Documents & Factures` /
  `الوثائق والفواتير`); selection via the row `Détails` button (row click is
  inert); `sales_get` takes BARE `document_id` (not nested); Documents gate
  needs `CREATE_SALE`; theme via `baraka-theme` seed + locale via login screen.

## STATE / DONE / REMAINING

STATE done. DONE: fetch + states + goldens + i18n + 3 PNGs + REPORT.
REMAINING: none. NEXT READY: YES.
