# TRACE.md — print path + Details drawer stage tables (TASK-AUD-038)

## P1. Ticket/BL/Facture: sale checkout → device (all three types share one path)

| stage | file:line | verdict |
|---|---|---|
| job created at checkout | `crates/commands/src/ipc.rs:1401` `sales_checkout` — zero print/enqueue references (grep empty); `enqueue_job` (ports.rs:1239, sqlite_print_job_repository.rs:40) has NO production caller | **BROKEN (root)** — no job ever enters the outbox on sale |
| worker picks PENDING | `worker_thread.rs:3` (500ms poll `claim_next_pending`), spawned at startup `src-tauri/src/lib.rs:157` | WORKS (starved: queue always empty) |
| renderer renders | `print_worker_service.rs:process_claimed` → `renderer.render_with_paper(&receipt, paper)` (~line 380); single thermal renderer `rasterizer.rs:409/422`, 58mm=384 / 80mm=576 dots (`paper.rs:23-24`) | WORKS (thermal only — no A4/BL/FACTURE variant anywhere) |
| device send | `SerialPrinterTransport::send_raster` (`transport_serial.rs:59-70`), wired `lib.rs:123`; opens `printer.serial_port`@baud, error → `PrinterTransportUnavailable` → retry/exhaust | WORKS (code path real; never reached from checkout) |
| status updated | `mark_done` / `release_for_retry(PRINTER_OFFLINE…)` / `mark_failed(EXHAUSTED…)` (`print_worker_service.rs:292-325`) | WORKS |
| reprint enqueue | `documents_reprint` (`ipc.rs:2833`) → `ReprintDocumentUseCase` (`printing.rs:37-49`) → `enqueue_reprint` (`sqlite_print_job_repository.rs:201`, DUPLICATA watermark) | WORKS — the ONLY live job source |
| test-print | `printer_test_print` (`ipc.rs:2976`) → `TestPrintUseCase` (`printing.rs:328-370`): compose → `capture.write_capture` → capture-kind short-circuits device, physical kind attempts real send | WORKS (settings diagnostic, not the outbox path) |

Verdict one-liners: TICKET **BROKEN** (no checkout job), BL **BROKEN** (same + no A4 layout), FACTURE **BROKEN** (same + no A4 layout). A real print has only ever been possible via reprint/test-print jobs, never automatically on sale. No `unimplemented!/todo!/FIXME` in the print path (grep empty) — the gap is a missing call, not a stub.

## P2. Details drawer: click → query → render

| stage | file:line | verdict |
|---|---|---|
| click opens drawer | `DocumentsModule.tsx` → `DocumentDetailDrawer.tsx:33` (`document: DocumentSummaryWireDto`) | WORKS |
| detail query | NONE — drawer receives the already-loaded summary; `getDocumentDetails` (`ipc.ts:1105`, full view: document + items + payments + print_status via `sales_get`) exists but has ZERO callers in `src/features/documents/` (grep empty) | **ABSENT (root)** |
| render | number + badges + metadata + financial cards + `total_ttc_centimes` only (`DocumentDetailDrawer.tsx:120-122`); reprint button wired (`:63` → `ipcReprint`) | WORKS (summary-only) |

Missing for ticket details: item lines (product, qty, unit price, line TVA/TTC), payments (method + amount), tax breakdown (subtotal HT, TVA total), change/paid amounts — all present in `DocumentDetailsWireResponse` (`ipc.ts:1071-1103`), none rendered.
