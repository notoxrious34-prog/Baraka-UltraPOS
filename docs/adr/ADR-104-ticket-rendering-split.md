# ADR-104 — Ticket Rendering Split: ESC/POS vs Graphique (label-class USB printers)

Status: ACCEPTED. Date: 2026-10-24 (SESSION-ARC-029). Authority: Architect (Charter v2.0 §1.2).

## Context

Owner hardware verdict (rc.11 + BRK-089): queue `Xprinter XP-410B` is exact,
Idle, Windows test page prints, graphics barcodes print — but our ESC/POS
RAW bytes vanish without a trace. The unit is a label printer (TSPL/label
mode) that does not speak ESC/POS on this path. Evidence: owner trial
2026-10-24 + audit/BRK-089{-C1}/.

## Decision

Two ticket-rendering modes, selected by setting `ticket_render_mode`
(default `escpos`, validated):

- `escpos`: unchanged engine path (serial COM or spooler RAW bytes) for
  true receipt printers.
- `graphique`: engine still creates the atomic print job, but the worker
  does NOT emit ESC/POS bytes; the job transitions to a new terminal
  status `FRONTEND_RENDERED` (audit event recorded). The 80mm ticket is
  rendered as HTML and printed through the OS graphics path (the same
  proven pipeline as barcode labels, DSN-083) — manually (reprint) and
  automatically after checkout.

No printer-specific PDL (TSPL/EPL/ZPL) is emitted in v1: graphics keeps
one renderer for every USB label printer.

## Consequences

- A new print-job terminal status may require a CHECK-constraint migration
  (additive, no data change) — authorized inside TASK-BRK-090 if needed.
- Real receipt printers (COM or ESC/POS-over-USB) keep working exactly as
  before; the owner flips one setting per shop.
- The silent-failure lesson stands: every mode must surface its true
  state (AUD-042 surfacing spec applies to both modes).
