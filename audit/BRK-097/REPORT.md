# TASK-BRK-097 REPORT — Silent TSPL engine

- ADR-105 (81cfbd3) ACCEPTED. Owner-locked: ONE XP-410B for both; button-triggered silent print (NO dialog, NO auto-print); label 40×20 mm (320×160 dots @203dpi — supersedes DSN-083 39×20); label = name + price + Code128; ticket = graphique content model in TSPL. Spooler-RAW transport exists (BRK-089); graphique `window.print` kept as fallback (untouched).
- Status: DONE. No commit (REPORT staged by architect). No new dependencies. Live DB never touched.

## 1. Geometry math + command table

- 203 dpi ≈ 8 dots/mm EXACT (`DOTS_PER_MM = 8`; true 7.992, 0.1% absorbed by printer calibration).
- Label: 40×8 = **320** × 20×8 = **160** → `SIZE 40 mm,20 mm`.
- Ticket: 80×8 = **640** wide → `SIZE 80 mm,H mm`, H = ceil(total_dots/8), cursor-computed.
- Label bytes: `SIZE` / `DIRECTION 1` / `CLS` / `TEXT 8,8,"2"` (name, 25 chars) / `TEXT 8,36,"3"` (price + ` DZD`, 19 chars) / `BARCODE 8,72,"128",56,1,0,2,2` / `PRINT 1,n` (1 set, n copies — never resends).
- Ticket bytes: `SIZE 80 mm,H mm` / `DIRECTION 1` / `CLS` / `TEXT` lines (store F3, address/phone F1, rules, number+date F2, sku F1, designation F2, `qty unit x pu = total` F1, five totals, payment, change) / centered `BARCODE x,y,"128",64,1,0,2,2,"<number>"` / footer F1 / `PRINT 1,1`.
- Font metrics (TSC TSPL reference, STATED): "1"=8×12, "2"=12×20, "3"=16×24. Field budgets derive from them (name (320−16)/12=25, price (320−16)/16=19, store (640−32)/16=38, F1 (640−32)/8=76, F2 (640−32)/12=50).
- Code128 density (STATED): 11 modules/char + 35 overhead, narrow=wide=2. Label budget 304 dots (10-char fit 290, 11-char 312 reject). Ticket budget 608, centered x=(640−w)/2.
- No `GAP` emitted (printer's calibrated media gap applies; DSN-098 owns media setup). No `CUT` emitted: TSPL core has no printer-agnostic cut command — physical cut relies on the printer's auto-cut configuration (handoff clause: STATED with reference, cut omitted by design, no flag).
- TEXT payloads pass through as UTF-8 bytes (never transliterated). Caveat STATED: built-in TSPL fonts do not shape Arabic and TSPL has no BiDi — Arabic names/captions need soft fonts (deferred to hardware verification). `"` → `'` and CR/LF → space in TEXT (command-injection safety); `"` excluded from Code128 payloads (unquotable in TSPL). Truncation marker is ASCII `...`. Barcodes are never truncated (reject `TsplBarcodeTooWide`). Empty unit renders without double space (graphique cells collapse whitespace — stated micro-deviation, values identical).
- Permission gates (STATED): label = `PosSell` (Vendeur prints shelf labels, cannot print tickets); ticket = `ReprintDocument` (same as reprint path).

## 2. Byte-assert lines

- Domain (`printing::tspl`, 9/9): `geometry_math_is_pinned` (8, 320, 160, 640); `label_bytes_are_exact` (exact 7-line `SIZE…PRINT 1,3` match); `label_copies_live_in_print_not_resends` (`PRINT 1,1`/`PRINT 1,99`, single BARCODE, 0/100 → `TsplCopiesOutOfRange`); `label_barcode_guard_rejects_non_ascii_and_quotes` (é, ✓, `"`, tab → `TsplBarcodeUnencodable`); `label_barcode_width_rejects_instead_of_shrinking` (290 fit / 312 reject); `label_text_truncates_and_sanitizes_without_silent_overflow` (`...` + `Vis 8' acier zingué`); `ticket_bytes_pin_size_gapless_barcode_and_copies` (`SIZE 80 mm,59 mm`, `BARCODE 142,348`, `PRINT 1,1`, no GAP/CUT); `ticket_totals_mirror_stored_values_verbatim` (`3 Pièce x 9.00 = 27.00`, no `10.00`, `VIS-001`, five totals); `ticket_arabic_captions_emit_without_destroying_bytes` (`المجموع: 27.00`).
- Use-case (`tspl_print_tests`, 7/7 over fakes): `label_spools_exact_bytes_and_reports_count` (1 submission, `bytes_sent == len`, `SIZE`/`TEXT`/`BARCODE`/`PRINT 1,3`); `label_rejects_non_spooler_transport_without_touching_the_queue` (serial + escpos → `TsplTransportRequiresSpooler`, queue empty); `ticket_rejects_non_spooler_transport_without_touching_the_queue` (same, serial); `ticket_transcribes_discounted_and_legacy_lines_verbatim` (net `3 Pièce x 9.00 = 27.00`, no `10.00`, legacy `5.00`, `Sous-total: 32.00`/`TOTAL: 32.00`/`Payé (CASH): 32.00`, `PRINT 1,1`); `ticket_gate_reprint_document_but_label_allows_vendeur` (Vendeur ticket → `Unauthorized`, label → 1 spool); `label_copies_and_price_guards_reject_before_spooling` (0 → CopiesOutOfRange, −1 → `TSPL_PRICE_MUST_BE_NON_NEGATIVE` passthrough, `VIS-é-001` → Unencodable, queue empty); `ticket_missing_document_surfaces_not_found`. Write-port fakes panic — passing tests prove zero writes.

## 3. IPC surface (wire only — DSN-098 owns buttons)

- `crates/commands/src/dto.rs` (+35): `TsplLabelRequest { session_token, name, price_centimes, barcode, copies }`, `TsplPrintResponse { bytes_sent }`, `TsplTicketRequest { session_token, document_id }`.
- `crates/commands/src/ipc.rs` (+82/−6 fmt): `print_tspl_label` (PosSell use-case, wire centimes via `parse_wire_int`, trimmed name/barcode), `print_tspl_ticket` (ReprintDocument use-case, `parse_wire_id` → `DocumentId`), `error_code` arms: `TsplTransportRequiresSpooler → TSPL_REQUIRES_SPOOLER`, `TsplBarcodeUnencodable → TSPL_BARCODE_UNENCODABLE`, `TsplBarcodeTooWide → TSPL_BARCODE_TOO_WIDE`, `TsplCopiesOutOfRange → TSPL_COPIES_OUT_OF_RANGE`.
- `src-tauri/src/lib.rs` (+2): both commands registered in `generate_handler!`.

## 4. Gates

| Gate | Command | Result |
|---|---|---|
| G1 | `cargo fmt --check` | CLEAN, exit 0 |
| G2 | `CARGO_BUILD_JOBS=2 cargo clippy --all-targets --all-features -- -D warnings` | GREEN, zero warnings, exit 0 (retry after 2× `identity_op` in the new test file — fixed to `10_000`) |
| G3 | `CARGO_BUILD_JOBS=2 cargo test --workspace -j 2` | GREEN — **71 suites, 565 passed, 0 failed, 7 ignored** (549 baseline + 9 renderer + 7 use-case) |
| G4–G7 | N/A — backend-only, no shared surface touched (no TS, no UI; IPC commands are new, nothing references them yet) | state: not run |

- Renderer 9/9 + use-case 7/7 also re-run standalone on final source: green.

## 5. Files (absolute)

- `D:/Dev/Baraka UltraPOS/crates/domain/src/printing/tspl.rs` [CREATED] — pure renderer + 9 pins.
- `D:/Dev/Baraka UltraPOS/crates/domain/src/printing/mod.rs` [MODIFIED] — `pub mod tspl` + re-exports.
- `D:/Dev/Baraka UltraPOS/crates/domain/src/errors.rs` [MODIFIED] — 4 Tspl variants.
- `D:/Dev/Baraka UltraPOS/crates/application/src/use_cases/tspl_printing.rs` [CREATED] — 2 use-cases (+`Debug` on outcome for tests).
- `D:/Dev/Baraka UltraPOS/crates/application/src/use_cases/mod.rs` [MODIFIED] — `pub mod tspl_printing`.
- `D:/Dev/Baraka UltraPOS/crates/application/tests/tspl_print_tests.rs` [CREATED] — 7 fake-port tests.
- `D:/Dev/Baraka UltraPOS/crates/commands/src/dto.rs` [MODIFIED] — 3 wire structs (CRLF preserved, 35 insertions).
- `D:/Dev/Baraka UltraPOS/crates/commands/src/ipc.rs` [MODIFIED] — 2 handlers + 4 code arms.
- `D:/Dev/Baraka UltraPOS/src-tauri/src/lib.rs` [MODIFIED] — 2 handler registrations.
- `D:/Dev/Baraka UltraPOS/audit/BRK-097/REPORT.md` [CREATED] — this file.

## 6. UNKNOWNs / deviations

- Renderer bytes frozen green except one intentional addition during C1: sku line per item (graphique parity) — HEIGHT pin moved 57→59, barcode y 328→348, recomputed by hand and green.
- TSPL reference assumptions (fonts, Code128 density, `SET CUTTER` absence, `DIRECTION 1`) are STATED, not probed — XP-410B emulation must confirm on hardware (DSN-098/owner trial).
- Arabic shaping/codepage caveat STATED (bytes pass through; soft fonts deferred).
- No TS bindings added — DSN-098 wires `print_tspl_label`/`print_tspl_ticket` via Tauri invoke directly.

NEXT READY: YES.
