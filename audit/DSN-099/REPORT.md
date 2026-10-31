# TASK-DSN-099 REPORT — Graphique mirror of the professional design + label 9-char budget

## 1. FAIL-before (quoted frozen refs — survive the mirror)

- `printTicketDocument` hidden-iframe dispatch (`document.createElement("iframe")` + `win.print()`) unchanged in `src/features/tickets/graphique-ticket.ts`.
- `TICKET_PAGE_WIDTH_MM = 80` (`@page 80mm`) kept; `DUPLICATA` reprint mark kept.
- Basic-layout superseded facts: single-line contact meta (no tax rows), 4-col table without badge row, totals without barcode band, owner footer absent.
- Preview superseded fact: `SILENT_LABEL_BARCODE_DOTS = 304` (10-char capable, shrank to fit — diverged from the engine's 9-char reject).

## 2. Mirror changes (transcription-only, no invention)

`src/features/tickets/graphique-ticket.ts` — BRK-098 anatomy order, block for block:
store block (name + address/phone + NIF/NIS/RC/AI tax rows, blank fields collapse the line) →
meta (doc number · date) → depot row (`warehouse_id` verbatim) → title/reprint →
column header row → item rows (SKU → designation; `qty unit × discounted-PU = line total`, all verbatim wire strings) →
per-line badge (`remise <pu>` / `خصم <pu>`, stored `discount_per_unit_centimes` only, `null` legacy = old format untouched) →
totals hierarchy (Sous-total HT, TVA, TOTAL TTC emphasized, payment, paid, change — all stored centimes) →
doc-number Code128 (`generateCode128Svg`, height 48, human-readable caption mandatory, quiet zones via CSS padding on white) →
owner footer (`ticketFooterMessage` verbatim) → thanks footer. All figures LTR-isolated (`<bdi dir="ltr">`); AR root `dir="rtl"`.
New args/opts: `taxRows`, `warehouseText`, `ownerFooter`, `TicketPrintLine.discountPerUnit`, `TicketPrintStrings.depot/discountBadge`.

`src/features/documents/DocumentDetailDrawer.tsx` (forced plumbing): `A4Identity.ticketFooterMessage`; reprint passes tax/depot/footer from the same identity + details wire; A4 rows reuse the drawer per-unit badge (`documents.perUnitDiscount`) + `.a4-badge` print style (`document-print.css`).
`src/components/pos/modals/CheckoutModal.tsx` (forced plumbing): same tax/depot/footer from identity + receipt wire.
`src/lib/i18n.ts`: `ticket.depot` (Dépôt/المستودع), `ticket.discountBadge` (remise/خصم), `tspl.labelTooWide` (FR/AR), no emoji.
`src/features/catalog/SilentLabelModal.tsx`: `SILENT_LABEL_BARCODE_DOTS` 304→280 (320−2×20 quiet, engine parity), `SILENT_LABEL_BARCODE_MAX_CHARS = 9`, `silentLabelPreviewSvg` null over 9 (engine would reject loudly — preview refuses instead of previewing a lie), `silent-label-too-wide` alert + send disabled when over budget.

## 3. Absent-wire block notes (HANDOFF — omitted without fabrication)

- **Cashier row**: `sales_get` → `DocumentDto` carries no `created_by` (verified in `crates/commands/src/dto.rs`; summaries likewise). The TSPL `Caissier:` row has no graphique equivalent. Deferred with depot UUIDs verbatim per SCOPE-OUT.
- **Remise totale row**: `sales_get` carries no `global_discount_centimes`; per-unit badges ride the stored wire, the global row is omitted. Totals shown (Sous-total HT/TVA/TTC/paid/change) are stored values only, never recomputed.

## 4. Pins

`src/features/tickets/graphique-mirror.test.ts` (9 tests): FAIL-before refs; anatomy order store→tax→meta→depot→header→rows+badge→totals→barcode→owner→thanks; depot verbatim + exactly 1 badge on 2 lines; Code128 SVG + caption + quiet-zone CSS; blank tax/depot/footer collapse (thanks survives); legacy-NULL old format (raw PU, no badge); 9-char budget (9 ok / 10 null + `silent-label-too-wide` + FR/AR keys quoted); AR RTL with LTR figures.
Updated: `graphique-ticket.test.ts` fixtures (new required fields); `document-print.test.tsx` (`ticketFooterMessage` fixtures, words-test renamed, new single-badge A4 pin).

## 5. Captures (browser harness, chunked injection — file:// org-blocked, loopback refused)

- `audit/DSN-099/ticket-mirror-fr-light.png` (25,310 B): full ticket, asserts `lines=2, badges=1, svg=1`, anatomy order asserted in-page (all 15 markers in order), 0 console errors. Visual: header → NIF/NIS/RC → number/date → Dépôt → title → 2 rows (badge `remise 1,00 DZD` on discounted only) → totals (TOTAL TTC emphasized) → Code128 + `T-2026-000123` caption → owner footer → thanks. No overlap/clipping.
- `audit/DSN-099/label-modal-9char-ar.png` (22,194 B): AR RTL modal, 10-char payload `0123456789`, asserts `tooWide=true`, send `disabled=true`, guidance `الرمز الشريطي طويل — 9 أحرف كحد أقصى`, 0 console errors (`errs:[]`). Guidance renders in place of the bars (refusal, not a preview); send blocked.
- Method notes: ticket HTML injected via `document.write` (2 chunks); label modal via scratch vite IIFE harness (33×10k chunks; first attempt failed — non-IIFE bundle + `+=` on script textContent; fixed with `format:iife` + string-accumulator insert). All scratch deleted.

## 6. Gates

- G4 `npm run lint` EXIT 0, 0 warnings.
- G5 `npm run typecheck` (`tsc --noEmit`) EXIT 0.
- G6 `npm run build` EXIT 0 (179 modules, dist 809 KB JS / 50.5 KB CSS).
- G7 `npx vitest run` EXIT 0 — 84 files, 874 tests, 0 failed.
- G1–G3 N/A — zero Rust changed (TSPL bytes frozen, stated).

## 7. Status

- `git status --short`: `M CheckoutModal.tsx, SilentLabelModal.tsx, DocumentDetailDrawer.tsx, document-print.css, document-print.test.tsx, graphique-ticket.test.ts, graphique-ticket.ts, i18n.ts` + `?? audit/DSN-099/` + `?? src/features/tickets/graphique-mirror.test.ts`. No strays (`__dsn099*` gone from src, `D:/Dev/_scratch/DSN-099/` removed; `_scratch` keeps pre-existing AUD-005/TASK-DSN-012 only). DO NOT COMMIT honored.
- STATE: DONE. REMAINING: none. NEXT READY: YES.
