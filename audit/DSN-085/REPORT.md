# TASK-DSN-085 REPORT — A4 print path for BL/FACTURE

## Per-type button decision

| Type | A4 button | Rationale |
|------|-----------|-----------|
| BL | YES (`Imprimer A4`) | Delivery note, non-fiscal paper companion |
| FACTURE | YES (`Imprimer A4`) | Invoice, the A4 use case |
| TICKET | NO | Fiscal thermal path (existing DUPLICATA reprint); no A4 reformatting of fiscal tickets |
| DEVIS/AVOIR | NO | No A4 layout in this task |

Pinned: `canPrintA4` unit test + drawer source pin (`canPrintA4(document.document_type)`)
+ live shot proving absence on TICKET. Permission: no new semantics — the module
gate (`DocumentsModule.tsx:37-40`, CREATE_SALE per DSN-078) covers the drawer.

## Implementation

- `DocumentDetailDrawer.tsx`: `canPrintA4`, `A4Identity`, pure `DocumentA4Sheet`
  (renders engine values verbatim; date comes from the summary row — the details
  wire carries no date), print flow (click → `getStoreIdentity()` → body
  `printing-a4` → rAF → `window.print()` → `afterprint` cleanup; identity-fetch
  failure falls back to empty identity = header omitted, never fabricated).
- `document-print.css` (new): screen-hides `.a4-sheet`; print media: `@page A4
  12mm`, app hidden via `body.printing-a4` visibility technique, sheet at
  186mm width, black-on-white, `thead → table-header-group`, `.a4-num` monospace.
- `i18n.ts`: +2 FR / +2 AR (`documents.printA4`, `documents.printFooter`;
  CRLF preserved, 4 insertions).

## ENGINE-GAP field list (for BRK follow-up — omitted, never faked)

Per-line discount + discount rate · per-rate TVA breakdown (only line totals) ·
totals-in-words (no engine field) · customer fiscal identifiers (summary has
name only) · unit display names/designations (ids only) · document creation
date in details wire (worked around via summary `created_at`).

## Gates

- G1/G2/G3: N/A — zero Rust touched (drawer + css + tests + i18n only).
- G4 lint: exit 0. G5 `tsc --noEmit`: clean (fixed 3 self-made errors:
  wire `doc_type` vs `document_type`; `document` prop shadowing DOM `document`
  → `globalThis.document`).
- G6 build: ✓ 5.83s. G7 vitest: **75 files / 752 passed** (was 74/745: +7 pins).
- Lockfile line: `git diff --stat -- pnpm-lock.yaml package-lock.json yarn.lock package.json` → empty.

## PNGs + rows

- `a4-facture-1280x800-fr-light.png` — print-emulated, sheetW=703px (186mm):
  store header + NIF/NIS/RC/AI + `Factures F-2026-0042` + date + 2-line table +
  HT/TVA/TTC + footer; zero clip, zero chrome.
- `a4-bl-1280x800-ar-dark.png` — RTL BL, sheetW=703px: `سندات تسليم (BL)`,
  3 lines, totals `595,00 د.ج`, no clip, no chrome.
- `drawer-ticket-noa4-1280x800-fr-light.png` — TICKET drawer renders
  `[data-testid="details-lines"]` with NO `[data-testid="print-a4"]`.
- 0 console errors, DONE fails=0.
- Harness notes: `window.print` neutralized (flag); print CSS forced via
  `Emulation.setEmulatedMedia({media:'print'})`; `store_identity_get` wire is
  snake_case (`store_name` — camelCase stub silently yields empty header);
  `sales_get` takes bare `document_id`; nav labels are full strings.

## STATE / DONE / REMAINING

STATE done. DONE: button + sheet + CSS + pins + i18n + 3 PNGs + REPORT.
REMAINING: none (engine enrichment is a separate BRK). NEXT READY: YES.
