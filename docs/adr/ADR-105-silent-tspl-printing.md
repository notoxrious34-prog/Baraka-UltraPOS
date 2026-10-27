# ADR-105 — Silent TSPL printing for the single XP-410B (ticket 80mm + label 40×20)

Status: ACCEPTED. Date: 2026-10-27 (SESSION-ARC-033). Authority: Architect.

## Context

Owner verdict + locked answers (2026-10-27): discount excellent; ONE
XPrinter XP-410B prints BOTH tickets and labels; ticket via a button that
prints immediately with NO Windows dialog (not auto after sale); label
stock is **40×20mm** (supersedes the 39×20 DSN-083 calibration);
label = name + price + barcode. `window.print` (graphique path) always
shows the OS dialog — it can never be silent.

## Decision

Silent printing goes through the existing spooler-RAW transport
(BRK-089) with **native TSPL** bytes generated in the engine:

- Label 40×20mm (203 dpi → 320×160 dots): `SIZE`, `TEXT` (name, price),
  `BARCODE` Code128, `PRINT m,n`.
- Ticket 80mm continuous: TSPL page (`SIZE 80mm,0` gap-less) with `TEXT`
  lines mirroring the graphique ticket content model + `BARCODE` for the
  ticket number; cut via `PRINT 1,1` + cutter command where supported.
- Trigger: explicit buttons only (drawer/modal ticket button; catalog
  per-product label button with copy count). No auto-print, no dialog.
- Errors surface as verbatim codes (AUD-042 honesty applies).

No PDL beyond TSPL in v1; ESC/POS path untouched (serial receipt
printers); graphique `window.print` kept as fallback for non-spooler
setups.

## Consequences

- New engine renderer crate-adjacent module + silent IPC commands.
- Label geometry changes 312→320 dots wide (40mm); DSN-083 39×20
  calibration archived, not deleted.
- Owner acceptance: label scan + ticket read on the XP-410B.
