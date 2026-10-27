# TASK-DSN-098 REPORT — Silent print buttons + 40×20 label modal

- BRK-097 committed (e07b495): `print_tspl_label` (product + 1..99 copies) + `print_tspl_ticket` (document id, stored values, legacy fallback), spooler-only gate, verbatim codes. ADR-105 (81cfbd3). Owner taste: icon+text buttons, full "Baraka UltraPOS" name, Arabic clarity.
- Status: DONE. No commit. No new dependencies. Live DB never touched.

## 1. FAIL-before (quoted on the e07b495 tree)

`src/lib/silent-print-path.test.ts` — 3/3 baseline pins green (frozen references):
- drawer: `DocumentDetailDrawer.tsx` contains `window.print()` (afterprint + rAF effect) + `printTicketDocument(...)` import from graphique-ticket.
- checkout: `CheckoutModal.tsx` contains `printTicketDocument(html)` (auto-print effect) + `printTicketDocument(ticketHtml)` (manual button).
- catalog: `ProductLabelModal.tsx` contains `win.print()` (isolated iframe dispatch).
- 3 surface pins red-by-design until the work landed (facade, shared gate, modal file).

## 2. Switches (component lines)

- `src/lib/ipc.ts` (+45): `TsplLabelJob { name, priceCentimes, barcode, copies }`, `TsplPrintOutcome { bytesSent }`, `ipcPrintTsplLabel` → `print_tspl_label` (snake_case `price_centimes`/`copies`), `ipcPrintTsplTicket` → `print_tspl_ticket`. Wire mirrors `TsplLabelRequest`/`TsplTicketRequest`/`TsplPrintResponse` exactly.
- `src/lib/tspl-print.ts` [CREATED]: `validateTsplCopies` (integer 1..99, throws verbatim `TSPL_COPIES_OUT_OF_RANGE`), `ipcPrintTsplLabel`/`ipcPrintTsplTicket` bridges. Copies enforced once here; engine PRINTs n — no double multiplication (source pin).
- `src/lib/ticket-print-path.ts` [CREATED]: `resolveTicketPrintPath` — spooler + non-A4 → `silent-tspl`; serial + graphique → `graphique`; serial + escpos → `engine-reprint`; BL/FACTURE → `existing` (A4 untouched).
- `src/lib/i18n.ts`: 4 verbatim error-code mappings (FR+AR, code→code via `translateBackendError` — no new prose) + 11 `tspl.*` keys FR+AR (silent ticket, copies, `{bytes}`/`{n}` notices).
- `src/features/catalog/SilentLabelModal.tsx` [CREATED]: 40×20 modal, `aspectRatio 40/20`, name + `formatMoney` price + unit + Code128 vector preview (`fitModuleWidth`/`SILENT_LABEL_BARCODE_DOTS=304`, null-safe), copies stepper 1..99 + raw input (`data-testid` silent-label-copies/send/preview/notice/error), honest `labelsOk {n}/{bytes}` notice, verbatim error surface. Never `window.print` (source pin).
- `src/features/catalog/CatalogModule.tsx`: transport probe (`ipcGetPrinterSettings` → `spoolerMode`); spooler mounts `SilentLabelModal`, else frozen `ProductLabelModal` (entry point unchanged; catalog-hub `<ProductLabelModal` pin still green).
- `src/features/documents/DocumentDetailDrawer.tsx`: `transport` state + shared-gate `ticketPath`; spooler + thermal → `data-testid="silent-tspl-ticket"` (`tspl.silentTicket`) + ok/fail notices with byte count; else frozen graphique/engine branches.
- `src/components/pos/modals/CheckoutModal.tsx`: post-sale `done` effect claims the receipt and stops (no graphique html, no auto `window.print`) when spooler; renders `data-testid="silent-tspl-postsale"` + notices via the shared gate (`documentType: "TICKET"`); serial/graphique arms byte-identical.

## 3. Pins

`src/lib/silent-print-path.test.ts` — **8/8 green**: 3 FAIL-before quotes + facade validation + gate table + modal absence (`40`, no `window.print`/`printTicketDocument`/`printLabelDocument`) + silent-modules dialog-free (drawer gains `printTsplTicketSilent`, never `printTicketDocument(ticketHtml)`) + copies-once (`validateTsplCopies(input.copies)`, `copies: job.copies`, no `copies *`).

## 4. Captures (method + asserts)

- Env block: this sandbox refuses loopback (`ERR_CONNECTION_REFUSED` on 127.0.0.1; host server unreachable from the browser net-namespace) and org-blocks `file://`. CDP param ceiling ≈ 50–100 kB literal (`Separator ... chunk ... limit`). Workaround: `vite build` harness → `bundle.js` (322 kB) + `bundle.css` (49 kB, `@font-face` stripped) → single bundle.html (fonts base64) → chunked injection (30 kB base64 parts, `TextDecoder`, `textContent` assembly, no HTML parsing of payloads) on `about:blank` + synthetic `__TAURI_INTERNALS__` stubs + synthetic harness session (the invoke edge throws `IPC_NO_ACTIVE_SESSION` tokenless — harness seeds one). Real production components/stores/mapper; only the mount + transport stubs are synthetic (stated).
- `ticket-spooler-fr-light.png` (93,534 B): drawer `T-2026-000123`, badges, box `Client/Date/Articles`, lines (`Vis 8x40`, `VIS-001 · Pièce · 3`, struck `10,00` + net `9,00` + `27,00`), payments, totals, orange `🖨️ Ticket (silencieux)`, green `Ticket envoyé (214 o)`. Asserts: `data-testid="silent-tspl-ticket"` text exact, notice `214`, `bdi[dir=ltr]` number, `__consoleErrors` 0. Vision-verified: styled, no other print button visible.
- `label-modal-40x20-ar.png` (56,227 B): RTL modal `ملصق 40 × 20`, note `320 × 160 نقطة — الاسم + السعر + Code128`, preview (name, `10,00 د.ج / Pièce`, Code128 bars, `VIS-001` mono), copies `1` → `+` `+` → `3` (stepper live), amber `🖨️ طباعة`, green `تم إرسال 3 ملصق — 486 بايت`. Asserts: `silent-label-preview` + `aspect-ratio: 40 / 20`, copies input value, `document.dir === "rtl"`, notice `3/486`, `__consoleErrors` 0. Vision-verified: RTL mirroring intentional, nothing broken.
- No-dialog proof: silent-path modules contain zero `window.print` (pin §3); both shots reached paper-state without any dialog call (stub intercepts at `invoke`, before any dialog API). Owner live photos on the XP-410B remain the hardware acceptance.
- Note: raw ISO date in the ticket shot is harness fake data, not product formatting.

## 5. Gates

| Gate | Command | Result |
|---|---|---|
| G1–G3 | N/A — zero Rust (engine frozen; state reason) | not run |
| G4 | `npm run lint` | EXIT 0 |
| G5 | `npm run typecheck` (`tsc --noEmit`) | EXIT 0 |
| G6 | `npm run build` | EXIT 0 |
| G7 | `npx vitest run` | EXIT 0 — **83 files, 864 tests passed** (82/856 + pin file) |
- G4–G7 ran on final product source; no product file changed after (only harness/scratch files came later) — no re-run needed, stated.

## 6. Files (absolute)

- `D:/Dev/Baraka UltraPOS/src/lib/ipc.ts` [MODIFIED] (+45 bridges)
- `D:/Dev/Baraka UltraPOS/src/lib/i18n.ts` [MODIFIED] (+4 codes FR+AR, +11 `tspl.*` FR+AR)
- `D:/Dev/Baraka UltraPOS/src/lib/tspl-print.ts` [CREATED]
- `D:/Dev/Baraka UltraPOS/src/lib/ticket-print-path.ts` [CREATED]
- `D:/Dev/Baraka UltraPOS/src/features/catalog/SilentLabelModal.tsx` [CREATED]
- `D:/Dev/Baraka UltraPOS/src/features/catalog/CatalogModule.tsx` [MODIFIED] (probe + render switch)
- `D:/Dev/Baraka UltraPOS/src/features/documents/DocumentDetailDrawer.tsx` [MODIFIED] (silent button + notices)
- `D:/Dev/Baraka UltraPOS/src/components/pos/modals/CheckoutModal.tsx` [MODIFIED] (post-sale silent reprint)
- `D:/Dev/Baraka UltraPOS/src/lib/silent-print-path.test.ts` [CREATED] (8 pins)
- `D:/Dev/Baraka UltraPOS/audit/DSN-098/ticket-spooler-fr-light.png` [CREATED] (93,534 B)
- `D:/Dev/Baraka UltraPOS/audit/DSN-098/label-modal-40x20-ar.png` [CREATED] (56,227 B)
- `D:/Dev/Baraka UltraPOS/audit/DSN-098/REPORT.md` [CREATED] (this file)

## 7. Strays + port proof

- Harness deleted: `src/__dsn098.tsx`, `__dsn098.html`, `vite.__dsn098.config.ts`, `_dsn098-*.log`, `D:/Dev/_scratch/DSN-098/` (dist + inline/shots scripts) — `ls` + `grep -rln dsn098/__dsn098` clean except audit + REPORT.
- No servers started by this task (all captures via CDP injection; the `node http` probe exited 1 and owned no port). Port proof: no listening PIDs from this task.

## 8. UNKNOWNs / deviations

- Copies stepper: two rapid clicks batch in React (read-back shows stale `1`, send used `3`, notice confirms) — harness timing artifact, product control honest.
- `o` unit in `Ticket envoyé (214 o)` is the intentional short key (`tspl.ticketOk`), not truncation.
- TSPL reference assumptions (XP-410B fonts/density) remain STATED from BRK-097 — hardware trial pending (trial 4).

NEXT READY: YES.
