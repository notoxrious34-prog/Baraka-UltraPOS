# TASK-AUD-046 REPORT — Spooler job-completion audit (read-only)

## 1. submit_raw quotes (`crates/spooler_raw/src/lib.rs`, 392 lines, ONLY unsafe site)

Entry (`submit_raw`, lines 311-314): `queue.open()` → `job.write_raw(bytes)`. No other steps.

Open (`SpoolerJob::open`, lines 166-225):
- `OpenPrinterW(queue, PRINTER_DEFAULTS { pDatatype: NULL, pDevMode: NULL, DesiredAccess: PRINTER_ACCESS_USE })` (180-194). Failure → `Err(OpenPrinterW, GetLastError)`.
- `DOC_INFO_1W { pDocName: "Baraka UltraPOS", pOutputFile: NULL, pDatatype: "RAW" }` (202-209). NULL output = default port routing.
- `StartDocPrinterW(handle, 1, &doc_info)`; `job_id == 0` → `Err(StartDocPrinterW, from_thread code)` (213-222). `document_open = true` only after success (223).

Write (`write_raw`, lines 229-277):
- Write-full loop (232-265): `WritePrinter(handle, remaining, len as u32, &mut written)`; `!ok` → `Err(WritePrinter, code)`; `written == 0` → `Err(WritePrinter, code 0)` anti-hang guard; else `written_total += written`. Short-writes are looped, never truncated — PROVEN.
- `EndDocPrinter(handle)` ALWAYS after a full write (267); `document_open = false` BEFORE the check (268); failure → `Err(EndDocPrinter, code)` (269-276); else `Ok(())` (277).

Drop (281-296): `if document_open { EndDocPrinter (best-effort) }; ClosePrinter (best-effort)`. Order PROVEN: EndDoc-then-Close. StartDoc-failure path returns with `document_open == false` → Drop skips EndDoc, closes only (no orphan document). Mid-write failure → Drop EndDocs the partial job (discards it) + closes.

Transport (`transport_spooler.rs`): `send_raster` = queue-name → `SpoolerQueue::new` → `submit_raw` → `map_error`. `is_available` = `probe()` only.

## 2. Completion verdict + labels

- **COMPLETES (PROVEN): the spooler handshake.** Every Win32 call in the submit sequence is present and return-checked: Open → StartDoc → write-full loop → EndDoc → Close. `Ok(())` means the spooler ACCEPTED a complete, scheduled job.
- **NOT print completion (PROVEN by absence):** `Ok(())` is returned with zero knowledge of the printer. No `GetJob`/`JOB_STATUS` poll exists anywhere in the workspace (grep: zero hits); no `SetJob`/`ScheduleJob` call exists either — scheduling relies on the EndDocPrinter default. A job can therefore sit SPOOLING/PRINTING/ERROR in the queue while we report success.
- Datatype verdict (PROVEN): `pDatatype: "RAW"` is correct for TSPL bytes (pass-through unaltered). NOT a suspect.
- EndDoc verdict (PROVEN): called on ALL paths — success path, mid-write failure (via Drop), never duplicated (flag cleared before check).
- Schedule verdict (PROVEN): `ScheduleJob` never called and not needed (job isn't paused); advancement is the spooler's — which is exactly what can park it.
- `remaining.len() as u32` truncation: jobs > 4 GiB would truncate; ours are ~0.2–2 KiB — stated non-issue.
- Error arms reconciled (AUD-042/BRK-089): EVERY failure is loud `Err` — empty queue → `PrinterTransportUnavailable`; overlong/unknown/OS failures → `Validation(InvalidCanonicalString("spooler call {X} failed for queue … (OS code N)"))`. There is NO Ok-with-warning on this path; codes are verbatim to the UI. Success-Ok means accepted-by-spooler ONLY — the funnel never claims printed.

## 3. Probe-vs-send difference (PROVEN)

`probe()` (115-149) = `OpenPrinterW` + `ClosePrinter`, NO `StartDocPrinterW` — creates no job, moves no bytes, cannot park a document. It SUCCEEDS wherever the queue opens, even with the print pipeline wedged downstream. Hence: availability green + send dead is EXPECTED, not contradictory. The Tool's Get (a USB read/open) resets the pipe/monitor and flushes the parked job — matching the owner evidence.

## 4. Seagull interplay (INFERENCE ONLY)

Owner-side .inf facts (not vendored — no .inf in repo): Seagull V3 TSC driver 11.5.0.0 with PrintDispatcher + ConfigDispatcher + NetMon. INFERENCE: a V3 language monitor interposes on the RAW path (status/bidi callbacks over USB); with the printer confused (blank NVRAM, uncalibrated sensor per AUD-045), the monitor can hold the job pending printer status indefinitely — the job never reaches COMPLETED until an external USB open (Tool Get) resets the pipe. No driver source available here; mechanism unprobed.

## 5. Fix spec (minimal completion handshake — NO code)

1. Keep `pDatatype: "RAW"`, keep EndDoc-on-all-paths (already correct); add a debug assertion that `document_open` is false after `write_raw` returns.
2. Return the spooler job id from `submit_raw` (StartDocPrinterW already yields it; currently discarded).
3. Post-send `GetJob` poll (bounded, e.g. 15 s): surface `JOB_STATUS_PRINTING/SPOOLING` persistence vs `PRINTED` vs `ERROR/OFFLINE/PAPEROUT/PAUSED` as distinct verbatim codes — turns today's blind-Ok into a proven-completed or named-stuck verdict.
4. Keep the write-full loop; document the u32 4 GiB non-issue beside the cast.
5. No `ScheduleJob` (job isn't paused); no datatype change.

## 6. Owner-side probe (HANDOFF — Win32 runtime behavior unprovable statically)

After a failed silent print, BEFORE pressing Get: open the Windows print queue window for `XPrinter XP-410B` and report: job present? status column text (Spooling / Printing / Error / Paused)? byte size? Then press Get and report whether that same job prints or vanishes. That single observation decides parked-job (fix §5) vs pipe-reset (driver/monitor).

## 7. Evidence status

- `git status --short`: only `?? audit/AUD-046/` (this REPORT). NOTHING modified. No builds/tests run (no cargo/npm). Live DB never touched.
- STATE: DONE. REMAINING: owner queue-window probe (§6) → completion-handshake fix (§5). NEXT READY: YES.
