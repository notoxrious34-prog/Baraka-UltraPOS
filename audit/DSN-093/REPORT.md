# TASK-DSN-093 REPORT — Graphique 80mm ticket (OS graphics path + mode selector + auto-print + honest banner)

- ADR-104 (ACCEPTED): label-class printers (XP-410B) silently discard ESC/POS bytes; graphique path emits zero bytes, frontend renders 80mm HTML via the OS path.
- BRK-090 committed (029c0f7): `ticket_render_mode` validated/default `escpos` in KV + IPC; worker marks `FRONTEND_RENDERED` with zero bytes in graphique mode.
- Proven pipeline reused: label browser-print (ProductLabelModal → hidden iframe → `window.print` → OS spooler, DSN-083).
- AUD-042 §4a: test banner split by outcome class (no new IPC fields).
- Status: DONE. No commit. Live DB never touched. No new dependencies. Zero Rust (G1-G3 N/A — no `.rs` file in the diff).

## 1. FAIL-before (quoted)

- No graphique ticket path existed: `PrinterSettingsTab.tsx` carried no `ticket_render_mode` state or control; `src/lib/ipc.ts` `PrinterSettings` had no `ticketRenderMode`; the checkout success view showed only `PrintStatusIndicator` (engine job row); `DocumentDetailDrawer` reprint used only `ipcReprint` (engine ESC/POS `documents_reprint`).
- The two new pin files failed red pre-implementation (`Failed to load url ./graphique-ticket`, `.../test-print-notice` — modules did not exist), proving no prior graphique path.

## 2. Component / template lines

| Piece | File |
|---|---|
| `TICKET_PAGE_WIDTH_MM = 80`, `buildTicketPrintHtml` (80mm `@page`, LTR-isolated numbers via `bdi`, DUPLICATA mark on reprint, HTML-escaped), `ticketArgsFromWire` (details wire + summary only, names per document language with cross-locale fallback, figures untouched), `printTicketDocument` (hidden-iframe dispatch, DSN-083 reuse), `ticketMoney`/`ticketQty` verbatim renderers | `src/features/tickets/graphique-ticket.ts` [CREATED] |
| `resolveTestPrintNotice`: `sent`→success, `capture-only`→neutral + switching guidance, anything else→error carrying the outcome text; `testPrintToneClass` | `src/features/settings/test-print-notice.ts` [CREATED] |
| `TicketRenderMode`, `PrinterSettings.ticketRenderMode`, `toTicketRenderMode` (absent/invalid→`escpos`); get/update plumb `ticket_render_mode` (update sends `null` to keep stored mode — BRK-090 None-keeps) | `src/lib/ipc.ts` [MODIFIED] |
| FR+AR keys: `settings.ticketRenderTitle/Escpos/Graphique/Help` (help carries the 80mm driver-paper-size line), `settings.testPrintCaptureNote`, `ticket.*` (12 chrome keys), `documents.printTicket`, `checkout.printTicket` | `src/lib/i18n.ts` [MODIFIED] |
| Render-mode segmented group (DocumentsIcon ESC/POS + PrinterIcon graphique, `aria-pressed` per mode) + 80mm help note + honest 3-tone notice | `src/features/settings/PrinterSettingsTab.tsx` [MODIFIED] |
| Auto-print effect: `ticketPrintedFor` guard claimed BEFORE async (never twice per sale, even on re-render); graphique-only; payload from `getDocumentDetails` + `getStoreIdentity` via `ticketArgsFromWire`; `printTicketDocument`; ticket failure never fails the committed sale; fallback reprint button when HTML ready; escpos path untouched | `src/components/pos/modals/CheckoutModal.tsx` [MODIFIED] |
| Per-open `renderMode` resolve; graphique→`printTicket` (same HTML, always DUPLICATA, UI-locale language fallback — the details wire carries no `document_language`, documented gap, no engine change); escpos→existing `ipcReprint` untouched | `src/features/documents/DocumentDetailDrawer.tsx` [MODIFIED] |

## 3. Pin lines

- `src/features/tickets/graphique-ticket.test.ts` [CREATED, 9 tests]: wire centimes/micros fixtures; store/names/SKU/totals verbatim; DUPLICATA on/off; 80mm `@page` + hostile-input escaping; 2 `data-testid="ticket-line"` + `dir="ltr"` numbers; AR `dir="rtl"` root; wire-mapper FR/AR selection + dash degradation; `TICKET_PAGE_WIDTH_MM = 80`.
- `src/features/settings/graphique-mode.test.ts` [CREATED, 11 tests]: 3 pure classifier pins (sent / capture-only FR+AR / `OpenPrinterW` error text) + tone classes; segmented-control shape + FR+AR keys; bridge mapping + escpos fallback; banner split; auto-print guard (`ticketPrintedFor`); escpos-untouched (`ipcReprint` kept, no `documents_reprint` in checkout).
- `src/features/settings/printer-transport.test.ts` [MODIFIED, 2 lines]: import-line + `DocumentsIcon` assert updated for the new control (same icon set, no new icons — pictograph scan still green).

## 4. PNG names + asserts (`audit/DSN-093/capture-results.json` 7/7 GREEN, 0 console errors)

- `ticket-80mm-fr-light.png`: 2 lines, wire figures, no DUPLICATA on clean sale, no page scroll.
- `ticket-80mm-ar-reprint.png`: `dir="rtl"` root (description right, amounts left — true RTL, not mirrored), DUPLICATA boxed and visible, 16 LTR-isolated numbers, no page scroll.
- `settings-render-mode-1280x800-fr-light.png`: `Ticket ESC/POS=false, Ticket graphique 80mm=true`, 80mm help note, save payload assert `ticket_render_mode=graphique`.
- Cosmetic note (accepted): narrow item columns wrap the currency suffix (`DZD`/`د.ج` drops to a second line; totals block stays single-line); `البيان` header tight to the right edge. Legible, no clipping; real-driver font metrics will differ — owner trial confirms.

## 5. Gates

| Gate | Result |
|---|---|
| G1-G3 | N/A — zero Rust: no `.rs` file created or modified (engine/worker/bytes frozen per scope) |
| G4 `npm run lint` (eslint, max-warnings 0) | GREEN, exit 0 |
| G5 `npx tsc -p tsconfig.json --noEmit` | GREEN, no errors |
| G6 `npm run build` | GREEN, `✓ built in 6.09s` |
| G7 `npx vitest run` | GREEN — **797 passed, 0 failed** (incl. 20 new DSN-093 pins) |

## 6. Diff scope

Modified: `src/lib/{ipc.ts,i18n.ts}`, `src/features/settings/PrinterSettingsTab.tsx`, `src/components/pos/modals/CheckoutModal.tsx`, `src/features/documents/DocumentDetailDrawer.tsx`, `src/features/settings/printer-transport.test.ts`. Created: `src/features/tickets/graphique-ticket.ts`, `src/features/tickets/graphique-ticket.test.ts`, `src/features/settings/test-print-notice.ts`, `src/features/settings/graphique-mode.test.ts`, `audit/DSN-093/{capture.mjs,capture-results.json,3×png,REPORT.md}`. No production changes outside SCOPE-IN. Scratch: none created (captures live in `audit/DSN-093/` as deliverables). Vite :5173 stopped after captures; port verified free.

## 7. UNKNOWNs

- None blocking. HARDWARE-PENDING: owner USB print of a graphique ticket on the XP-410B (blocks trial build + owner first ticket). `document_language` absent from the `sales_get` wire — drawer reprints fall back to UI locale (documented, no engine change).

NEXT READY: YES
