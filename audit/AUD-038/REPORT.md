# TASK-AUD-038 REPORT — print path + Details drawer diagnosis (read-only)

Zero builds started; file reads + ripgrep only (~5 min CPU). Stage tables: TRACE.md.

## Issue A: printing does not work for ticket/bon/facture
Per-type verdicts: TICKET **BROKEN**, BL **BROKEN**, FACTURE **BROKEN**
(all three share one outbox path; see TRACE.md P1).

Ranked root causes:
1. **Checkout never enqueues a print job (root).** `sales_checkout`
   (`ipc.rs:1401`) contains no print/enqueue reference; `enqueue_job` has no
   production caller — the ONLY live job source is `documents_reprint`.
   Evidence: `grep enqueue_job` → ports + repo impl only.
2. **No document-type rendering split.** Worker renders every type through
   the single thermal `render_with_paper` (58/80mm); no A4 path for BL/FACTURE
   exists anywhere (no A4 renderer file, no dispatch on doc type).
3. **Device config unvalidated at sale time.** `SerialPrinterTransport`
   opens the COM port per job and maps failure to `PrinterTransportUnavailable`
   → retry → `EXHAUSTED_MAX_RETRIES`; a misconfigured port surfaces only as
   silent retries, never at checkout.

Fix proposals:
- A1 (engine, S): enqueue a print job in the checkout success path (same tx
  boundary as the sale, idempotency key = document id) — restores the whole
  chain; worker/transport/renderer already work.
- A2 (engine, M): A4 renderer + dispatch on doc type for BL/FACTURE
  (thermal stays for TICKET); paper-width already domain-validated.
- A3 (engine+frontend, S): surface `documents_get_print_status` failures at
  POS (the `PrintStatusIndicator` already polls + offers reprint).

A real print has only ever been possible via reprint/test-print jobs —
never automatically on sale.

## Issue B: Details click must show ticket details
Root causes:
1. **Drawer never queries details (root).** It renders the passed
   `DocumentSummaryWireDto` only; `getDocumentDetails` (`sales_get` full
   view) exists with zero callers in the documents feature.
2. **Render surface is summary-only** (badges, metadata, total) — no lines,
   payments, tax, or change sections exist in the drawer.

Fix proposal:
- B1 (frontend, S): on drawer open, call `getDocumentDetails(document.id)`,
  render items/payments/tax sections from `DocumentDetailsWireResponse`;
  handle UNAUTHORIZED/loading states per DSN-078 conventions. No engine work
  (query exists and is permission-gated server-side).

## Worktree
- `git status --short` shows only `?? audit/AUD-038/` added (REPORT.md + TRACE.md).
- Probe process untouched (not inspected beyond pre-existing knowledge).

STATE: complete / DONE: REPORT.md + TRACE.md / REMAINING: none.
