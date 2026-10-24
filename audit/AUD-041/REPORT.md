# TASK-AUD-041 REPORT — Owner-trial points (printer USB, drawer UUIDs, footer)

Read-only. No source/config/settings/DB writes, no installs, no test prints,
no builds. OS inspection is reads-only (PowerShell `Get-Printer` family).

## (A) Print fails "printer transport unavailable" with EMPTY COM port

### A1. Supported transports in code — exactly ONE physical path

- `crates/infrastructure/src/printing/transport_serial.rs:18-21,58-71`
  — `SerialPrinterTransport`: opens `printer.serial_port` via the
  `serialport` crate (COM only), 2 s write timeout.
- `crates/infrastructure/src/printing/capture.rs:64`
  — `CaptureDevice`: diagnostics-only capture sink, NOT a physical path.
- Wiring (the whole app): `src-tauri/src/lib.rs:123`
  (`SerialPrinterTransport::new`) → `:144` (`printer_transport`).
- Test fakes (`FakeTransport`, `UnavailableTransport`) exist only under
  `crates/infrastructure/tests/` — no USB, no spooler-RAW, no Win32
  `WritePrinter`, no browser-print path anywhere in prod code.

### A2. Where the owner's message is raised

- `transport_serial.rs:39-48` (`port_name()`): missing/empty/whitespace
  `printer.serial_port` → `None`.
- `transport_serial.rs:60-62`: `None` → `DomainError::PrinterTransportUnavailable`.
  `:66` (COM open fails) and `:70` (write/flush fails) map to the same error.
- `crates/domain/src/errors.rs:110-111`: Display = `"printer transport unavailable"`.
- Test-print surfacing: `TestPrintUseCase::execute`
  (`crates/application/src/use_cases/printing.rs:330-372`) → on physical send
  failure `TestPrintDeviceOutcome::DeviceError(device_error_code(...))`
  (`:376-380` renders the domain Display verbatim) → frontend notice
  `src/features/settings/PrinterSettingsTab.tsx:91-94`
  (`${testPrintSent}: ${deviceOutcome} — ${captureFileName}`).
- Sale-time surfacing: `crates/commands/src/ipc.rs:285` maps the same error
  out of the print-worker path. The owner's screenshot (Paramètres →
  Impression & Matériel, test print, red notice) is the test-print branch:
  **empty COM field → `port_name()` = None → exact string above.**

### A3. Settings inventory + defaults

- Keys (`settings` table): `printer.serial_port` (default `""` =
  unconfigured — `sqlite_printer_settings.rs:62`), `printer.serial_baud`
  (default `9600`, `:45-48`), `printer.paper_width_mm` (default `80`,
  `:49-60`; legacy dots migrate forward). Footer key `store.footer_message`
  (`store_keys::FOOTER`, `:119`) and device selector
  `printing.device.kind` (`"physical"` default / `"capture"`, `:120-122`).
- UI: `PrinterSettingsTab.tsx:30-32` (local defaults `""` / `9600` / `80`),
  free-text COM input `:112-123`, save `:60-82` (ADMIN `ManagePrinterSettings`),
  FR/AR test-print buttons `:84-101`, result notice `:198-208`.
- Migration default baud: `migrations/0006_print_worker_columns.sql:17`
  (`'printer.serial_baud', '9600'`).

### A4. What Windows reports (this box; shop hardware MUST re-confirm)

```
Get-Printer:  Xprinter XP-410B  →  PortName USB001
              (also: OneNote, XPS Writer, Print to PDF, Fax, Canon MF4700 USB005)
Get-PrinterPort: COM1:, COM2:, COM3:, COM4:, FILE:, LPT1-3, PORTPROMPT:, USB001, …
SerialPort.getportnames(): COM1
```

- The XP-410B is a **spooler printer on USB001**, NOT a serial device.
  `serialport::open("USB001")` is meaningless (COM namespace only) and
  `getportnames()` offers only **COM1** — there is no COM target for the
  XP-410B on this box, so ANY non-empty port the owner could type fails at
  `transport_serial.rs:66` with the same message.
- Caveat: this enumerates the dev box. The owner's screenshot already proves
  the mechanism on shop hardware (empty port → `None` → error) independent
  of which box; but the USB001/COM1 facts above must be re-confirmed on the
  shop PC with the same three one-liners before ordering hardware steps.

### A5. Verdict + minimal path + fix shape

- **USB-spooler printing is NOT supported** — single COM transport, by code.
- Minimal path (zero code, ops only — **S**): install the XPrinter
  virtual-COM (USB-to-serial) driver so the XP-410B appears as e.g. `COM3`,
  then set `printer.serial_port=COM3` (+9600 baud) in Paramètres →
  Impression & Matériel and re-run test-print FR. If the vendor driver
  offers no COM emulation, this path is closed.
- Code path (**M**): new `PrinterTransportPort` impl speaking spooler-RAW
  (Win32 `OpenPrinter`/`WritePrinter` or `windows` crate) + wiring +
  worker/test-print coverage. Likely needs a new dependency (architect
  authorization required) — do NOT start unasked. No settings-schema change
  needed (a `printer.device` selector already exists via `device_kind`).

## (B) Document drawer shows item UUIDs, not product names

### B1. Wire (frontend contract) — names absent by construction

- `src/lib/ipc.ts:1074-1097` (`DocumentDetailsWireResponse`, via
  `sales_get`): each item carries ONLY `product_id`, `unit_id`,
  `quantity_micro`, `unit_price_centimes`, `line_tva_centimes`,
  `line_ttc_centimes`. No `product_name`, no `sku`, no `unit_name`.

### B2. Backend — the names are never fetched

- `crates/commands/src/ipc.rs:2485-2543` (`sales_get`): maps
  `detail.items` to `DocumentItemDto { product_id, unit_id, … }` — IDs only.
- `crates/application/src/use_cases/sales.rs:1242-1280`
  (`GetSaleUseCase::execute`): `get_items(id)` pass-through, no enrichment.
- `crates/infrastructure/src/adapters/sqlite_document_repository.rs:174`
  (`ITEM_COLUMNS`: 14 columns, no name columns) and `:324-338`
  (`SELECT … FROM document_items WHERE document_id = ?1` — **no JOIN**).
- `crates/domain/src/sales/document.rs:259-288` (`DocumentItem`):
  `product_id` + `unit_id` only — the domain type itself has no name field.

### B3. Render — proves the UUID display

- `src/features/documents/DocumentDetailDrawer.tsx:129-136`
  (A4 block): renders `{item.product_id}` under the product column header.
- `:227-236` (details block): renders `{item.product_id}` +
  `{item.unit_id}` (mono, 10px sub-line).

### B4. Verdict + fix shape

- **ENGINE gap (query + DTO + domain type), not a frontend gap** — the
  frontend renders exactly what the wire provides. Precedent that the engine
  CAN resolve names: `documents_get_return_context` returns
  `ReturnableLine { product_name, sku, unit_name }` (`src/lib/ipc.ts`
  return-context interface).
- Fix shape **M**: JOIN `products`/`product_units` (live names) or a
  names-lookup in `get_items`, extend `DocumentItem` + `DocumentItemDto` +
  wire + drawer render. Design decision required: live names vs
  sale-time snapshots (frozen document vs renamed product) — flag to
  architect, do not assume. Frontend-only change cannot fix this (S frontend
  tail once the wire carries names).

## (C) Footer restyle — file/keybinding/top-bar map

### C1. Files

- `src/components/pos/KeyFooter.tsx` (12 affordances): `h-14` (56px) legend,
  `bg-slate-900` (dark:black), `bg-slate-700` chips, `<kbd>` code chips,
  `bg-accent-500` accent reserved for F12 (`accent:true`,
  dimmed when cart empty or no OPEN cash session).
- `src/components/pos/TotalsFooter.tsx` (47 lines): subtotal / discount /
  TVA rows + hero-total block (`h-16 rounded-lg bg-slate-950`, amber-400
  mono 4xl) — touching distance above the key legend.
- `src/components/pos/Topbar.tsx` (155 lines): `h-10` header,
  `bg-primary-900` white text, `rounded-full`/`rounded-md` pills,
  `text-[11px]`/`xs font-bold` — the token set for visual matching.
- Composition: `PosShellView.tsx:8-9` (imports), `:23` (`<Topbar />`),
  `:41` (`<KeyFooter />`).

### C2. Keybinding inventory (`src/hooks/use-pos-keyboard.ts`, global listener)

| key | action (handler line) | footer legend |
|---|---|---|
| F1 | focus search (`:51`) | F1 Recherche |
| F2 | customer modal (`:58`) | F2 Client |
| F3 | cycle line unit (`:63`) | F3 Unité |
| F4 | quantity modal (`:81`) | F4 Quantité |
| F5 | discount modal (`:88`) | F5 Remise |
| F6 | parked-sales list (`:95`) | "F6 Reprendre" ⚠ label redundantly includes the code (`i18n.ts:741`, AR `:1819`) |
| F8 | park sale (`:102`) | F8 Parquer |
| F10 | cycle doc type (`:109`) | F10 Document |
| F11 | **swallow only** — `preventDefault`, no action (`:118-122`, DSN-049 anti-fullscreen) | NOT in legend |
| F12 | checkout modal (`:122+`, no-op without OPEN session) | F12 Encaisser (accent) |
| Esc | close (`:24`) | Esc Fermer |
| Suppr/Backspace | remove line (`:161`) | Suppr. ligne |
| ↑↓ | navigate (`:171-176`) | ↑↓ Naviguer |
| Ctrl/⌘+K | command palette (`:181`) | NOT in legend |
| F7 / F9 | unbound | NOT in legend |

### C3. Verdict + fix shape

- Restyle is **DSN-only (S)**: tokens above give the Topbar/KeyFooter/
  TotalsFooter matching set; no engine work. Restyle inputs to carry:
  F6 label quirk, F7/F9/F11/Ctrl+K legend gaps (deliberate today —
  confirm before adding), F12 dimmed-state semantics.
- No fix proposed beyond the map (DSN-printer-UX / DSN-footer own the design).

## Verdicts (one line each)

1. Print fail = **empty/unmatched COM port against a COM-only transport**;
   XP-410B on USB001 is unreachable by code as configured. Path: vendor
   virtual-COM (S, ops) or spooler-RAW transport (M, new dep likely).
2. Drawer UUIDs = **engine gap** (no JOIN, no DTO fields); frontend is
   faithful. Fix M with a live-vs-snapshot naming decision.
3. Footer = **DSN-only map delivered** (S restyle by owner task; gaps noted).

## Files

- [CREATED] `D:/Dev/Baraka UltraPOS/audit/AUD-041/REPORT.md` (this file)

## Git + state

`git status --short` shows ONLY `?? audit/AUD-041/` (paste in handoff).
No process started (read-only session: no cargo/npm/node invoked).
Live DB never opened. No scripts created (NO_LEFTOVERS clean).

STATE done. DONE: all three points traced to file:line with OS facts.
REMAINING: BRK-details + DSN-footer/printer-UX (unblocked by this).
NEXT READY: YES.
