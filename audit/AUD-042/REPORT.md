# TASK-AUD-042 — USB test-print silent-failure trace (read-only)

Status: read-only audit. No code, config, DB, build, or test command executed.
No live DB opened. Canonical tree only: `D:/Dev/Baraka UltraPOS`.

## 1. End-to-end call chain (live test-print path, transport = spooler)

| # | Layer | Location | What happens |
|---|-------|----------|--------------|
| 1 | UI click | `src/features/settings/PrinterSettingsTab.tsx:103-120` (`handleTestPrint`) | Calls `printingTestPrint(language)`; on resolve sets `setNotice({ok: true, text: ...})` unconditionally |
| 2 | IPC wrapper | `src/lib/ipc.ts:1533-1555` (`printingTestPrint`) | `call("printer_test_print", {request: {language}})`; session token auto-injected into `request` by `call` (`src/lib/ipc.ts:83-86`); maps wire `device_outcome` → `deviceOutcome` verbatim |
| 3 | Tauri command | `crates/commands/src/ipc.rs:2988-3035` (`printer_test_print`) | `require_session` + `SystemSettingsManage` gate; `TestPrintUseCase::new(... state.printer_transport ...)`; maps `TestPrintDeviceOutcome::{Sent→"sent", CaptureOnly→"capture-only", DeviceError(code)→code}` |
| 4 | Use case | `crates/application/src/use_cases/printing.rs:330-372` (`TestPrintUseCase::execute`) | load identity → `device_kind` → compose sample receipt → `preview_lines` → `render` → `write_capture` (ALWAYS, `printing.rs:351`) → **only if kind == "physical"**: `transport.send_raster` (`printing.rs:353-360`); transport error → `Ok(report with DeviceError)`, never `Err` |
| 5 | Router | `crates/infrastructure/src/printing/transport_router.rs:55-61` (`RoutingPrinterTransport::send_raster`) | Reads `settings.printer.transport` fresh; `== "spooler"` → spooler arm, **anything else (absent, unknown, untrimmed) silently falls back to serial** (`transport_router.rs:50-51, 56-60`) |
| 6 | Spooler arm | `crates/infrastructure/src/printing/transport_spooler.rs:65-71` (`SpoolerRawTransport::send_raster`) | Reads `settings.printer.queue_name`; empty/absent/locked-DB → `PrinterTransportUnavailable`; else `SpoolerQueue::new` → `submit_raw` |
| 7 | FFI | `crates/spooler_raw/src/lib.rs:166-225` (`SpoolerJob::open`) | `OpenPrinterW` (`lib.rs:180-194`) → `StartDocPrinterW` (`lib.rs:213-222`); any failure → `SpoolerError::OsCallFailed{call, queue, os_code}` |
| 8 | FFI write | `crates/spooler_raw/src/lib.rs:229-278` (`write_raw`) | `WritePrinter` loop (`lib.rs:239-246`); 0-byte success guard → error code 0 (`lib.rs:255-262`); `EndDocPrinter` (`lib.rs:267-276`); `Drop` → best-effort `EndDocPrinter` + `ClosePrinter`, errors ignored (`lib.rs:282-296`) |
| 9 | Wiring | `src-tauri/src/lib.rs:123,144-146` | `printer_transport = RoutingPrinterTransport`, `printer_capture = CaptureDevice`, `printer_renderer` shared with worker |

Error mapping on the way back: `SpoolerError` → `transport_spooler.rs:50-61` (`map_error`: empty-name → `PrinterTransportUnavailable`, **everything else → `InvalidCanonicalString(display)`) → `printing.rs:376-381` (`device_error_code`: `Validation(d)` → domain string, other → `"DEVICE_ERROR"`) → `ipc.rs:3029-3033` → `device_outcome` string → banner text.

Structural proof that test prints never touch `print_jobs`: `TestPrintUseCase` (`printing.rs:288-301`) takes NO `PrintJobRepository` generic — only `S,R,C,T,K`. The only `enqueue_*` in `printing.rs` is `ReprintDocumentUseCase::enqueue_reprint` (`printing.rs:49`), a document path. Dead code: legacy `ipcTestPrint(sessionToken)` (`src/lib/ipc.ts:1398-1409`) expects the obsolete `{job_id,status}` shape and has **zero callers** (verified by grep); the live path is `printingTestPrint` only.

## 2. Silent-point table

| # | Cause | Code swallows where | Owner-visible symptom |
|---|-------|--------------------|-----------------------|
| S1 | Diagnostics device kind = `capture` (default `deviceKind: "capture"`, `src/stores/store-identity-store.ts:168`; separate setting from `printer.transport`, saved from identity modal) | `printing.rs:353-354`: `kind == "capture"` skips `send_raster` entirely, `device = CaptureOnly` — by design, no error | Green banner `Tâche créée: capture-only — test-….escpos`; no paper, no blink. Indistinguishable from success |
| S2 | Spooler accepts bytes but printer never pulls them (paused queue, offline printer, wrong-but-existing queue name, spooler service degraded) | Nowhere in this repo: `submit_raw` returns `Ok` once the spooler takes the bytes (`spooler_raw/lib.rs:311-314`); there is no post-spool status check | Green banner `Tâche créée: sent — test-….escpos`; no paper, no blink. **Most likely owner scenario — UNPROVEN statically** (see probe P1) |
| S3 | Unknown queue name (`OpenPrinterW` fails, e.g. OS code 1801) | Surfaced only as `deviceOutcome` text: `lib.rs:190-194` → `transport_spooler.rs:57-59` → `printing.rs:358` → `ipc.rs:3032` | **Green (ok:true) banner** containing `spooler call OpenPrinterW failed for queue 'X' (OS code 1801) — test-….escpos`; no paper. Error present as text but styled success; no retry offered |
| S4 | `StartDocPrinterW` / `WritePrinter` / `EndDocPrinter` failure (incl. 0-byte-write guard, `lib.rs:255-262`) | Same funnel as S3 (`OsCallFailed` → `DeviceError` string in `Ok` response) | Green banner with `spooler call WritePrinter/EndDocPrinter failed …` text; no (or partial, for EndDocPrinter-after-write) paper |
| S5 | `ClosePrinter` failure in `Drop` (`lib.rs:288-294`, best-effort `let _ =`) | Ignored by construction | Invisible; benign (bytes already spooled) |
| S6 | Router silently takes serial arm (`printer.transport` absent/unknown/untrimmed) | `transport_router.rs:50, 56-60`: fallback, no log, no error | Serial path runs against COM port (possibly empty → `PrinterTransportUnavailable`, `transport_serial.rs:60-62`); green banner shows `PRINTER_TRANSPORT_UNAVAILABLE`-class text; USB printer silent |
| S7 | Serial arm collapses ALL failures to one code | `transport_serial.rs:63-70`: open/write/flush errors → `PrinterTransportUnavailable`, OS detail discarded | Green banner `PRINTER_TRANSPORT_UNAVAILABLE`-class text; no diagnosis possible from UI |
| S8 | Queue name validated but never probed at save | `UpdatePrinterSettingsUseCase::execute` (`printing.rs:462-467`) validates non-empty ≤220 only; no `probe()` call anywhere on the save path | Save succeeds for a typo'd queue; failure surfaces only at test-print time as S3 |
| S9 | Settings-DB lock/read failure inside transport (`setting()` → `None`) | `transport_spooler.rs:26-33` (`.lock().ok()?`), `transport_serial.rs:29-36`: `None` masquerades as "unconfigured" | Green banner `PRINTER_TRANSPORT_UNAVAILABLE`-class text; actual cause (lock) invisible |
| S10 | Audit write fails AFTER bytes sent | `printing.rs:361-365`: `append_settings_audit` error → whole command `Err` (inverse silent point) | Paper prints, but UI shows RED error banner; operator retries → duplicate page |
| S11 | Capture file always written, even on send failure | `printing.rs:351` runs before `send_raster` | Banner always names a `test-….escpos` artifact (`capture.rs:57`), which reads as "a task was created" even when nothing reached the printer |

`print_jobs` lifecycle for test jobs: **none exists.** No row is created (`PENDING`/`DONE`/`FAILED`/`last_error` never written for test prints); `PrintStatusIndicator` polls `documents_get_print_status` only for real document ids (`CheckoutModal.tsx:428`, `DocumentDetailDrawer.tsx:569`) — the UI never re-reads anything after the test banner because there is nothing to re-read. The task premise's "created" = the capture artifact file name + `testPrintSent` i18n string (`src/lib/i18n.ts:168`: `"Tâche créée"`), NOT a job row.

## 3. Banner-vs-truth proof (file:line)

- `src/features/settings/PrinterSettingsTab.tsx:111-114`: `setNotice({ok: true, text: \`${t(locale,"settings.testPrintSent")}: ${result.deviceOutcome} — ${result.captureFileName}\`})` — `ok: true` is **hardcoded** on every resolved IPC call, including `deviceOutcome = "capture-only"`, `"sent"`, `"PRINTER_TRANSPORT_UNAVAILABLE"`, `"DEVICE_ERROR"`, or a full `spooler call … failed … (OS code …)` string.
- Styling follows `ok`: `PrinterSettingsTab.tsx:281-287` (`notice.ok ? green : red`). Every transport failure renders **green**.
- The red branch (`PrinterSettingsTab.tsx:115-117`, `catch`) fires ONLY for hard IPC `Err`: Unauthorized, identity load failure, renderer failure, capture failure, audit failure (`printing.rs:341,349-351,361`) — never for any spooler/serial outcome, because those travel inside `Ok(TestPrintResponse)` (`printing.rs:356-359`).
- Proven states showing "Tâche créée …" while NOTHING printed: `capture-only` (S1), `sent`-but-spooled-nowhere (S2), and every `DeviceError` code (S3/S4/S6/S7/S9) — the banner text differs only in the middle token, with identical success styling.

## 4. Surfacing spec (shape only, no implementation)

1. **Split the notice by outcome class** at the single construction site (`PrinterSettingsTab.tsx:111-114`): `deviceOutcome == "sent"` → success; `"capture-only"` → neutral/informational ("no physical send attempted — switch device kind to physical"); anything else → error styling (`ok: false`) with the code as the message body. No new IPC fields needed for this step.
2. **Poll/display point for true job state**: test prints currently have no `print_jobs` row and no pollable handle. To show spooler truth the UI needs a new pollable source — either (a) enqueue test prints through the existing outbox (new row + existing `documents_get_print_status`-style status read + existing worker retry/`last_error`), or (b) a lightweight `printer_spooler_status { queue, last_os_code, last_call }` read evaluated after `submit_raw`. Either way the display point is the settings-tab notice area, refreshed after the initial response — today nothing is re-read.
3. **Minimum fields the UI must display per attempt**: transport arm actually used (`serial`|`spooler`), queue/COM actually addressed, spooler call that failed (`OpenPrinterW`|`StartDocPrinterW`|`WritePrinter`|`EndDocPrinter`), OS code, and whether bytes were accepted by the spooler (`sent` ≠ printed). All except arm/queue already cross the IPC boundary inside `device_outcome`; arm + queue do not.
4. **Save-time validation**: probe the queue (`SpoolerQueue::probe`, `spooler_raw/lib.rs:115-149` — opens/closes without spooling a job) before persisting, and refuse/flag unknown queues instead of deferring failure to test time (S8).

## 5. UNPROVEN items + exact runtime probes (do not guess)

- P1 (S2 — leading hypothesis): whether `submit_raw` returned `Ok("sent")` while the Windows queue held the job. Probe: on the owner machine, reproduce one test print, then read the Windows queue (`Get-PrintJob -PrinterName "XPrinter XP-410B"`, queue paused/offline flags via `Get-Printer`), and `Microsoft-Windows-PrintService/Operational` event log around the timestamp; correlate with the `PRINT_TEST_PERFORMED` audit row + capture file in `%APPDATA%/com.baraka.ultrapos/printer-capture/`. Live DB was NOT opened per task rules.
- P2: whether the owner's run took the `capture-only` branch (S1) vs `sent` (S2). Probe: read back the exact banner middle token from the owner screenshot text (`capture-only` vs `sent` vs `spooler call …`), or re-run once with the identity-modal device kind visible.

## 6. Files consulted (read-only)

`src/features/settings/PrinterSettingsTab.tsx` · `src/lib/ipc.ts:41-92,1398-1409,1533-1555` · `src/lib/i18n.ts:164-183` · `src/stores/store-identity-store.ts:168,283-300` · `crates/commands/src/ipc.rs:2983-3035` · `crates/commands/src/dto.rs:1118-1138` · `crates/application/src/use_cases/printing.rs:284-381,412-477` · `crates/application/src/ports.rs:1267-1330,1440-1446` · `crates/application/src/services/print_worker_service.rs:238-324` · `crates/domain/src/printing/transport.rs` · `crates/infrastructure/src/printing/{transport_router.rs,transport_spooler.rs,transport_serial.rs,capture.rs}` · `crates/infrastructure/src/adapters/sqlite_print_job_repository.rs:62-153,182-242` · `crates/spooler_raw/src/lib.rs` (full) · `src-tauri/src/lib.rs:33-35,122-146,263`
