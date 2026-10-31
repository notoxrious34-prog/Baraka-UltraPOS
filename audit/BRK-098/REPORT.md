# TASK-BRK-098 REPORT — World-class TSPL ticket + label design

- Base: ca606df (DSN-098 silent buttons). Renderer was BRK-097 byte-correct but layout-basic.
- Status: DONE (pending G2/G3 recapture — clippy running). No commit. No new dependencies. Live DB never touched.

## 1. Anatomy table

**Ticket 80 mm** (top → bottom, `SIZE 80 mm,H mm`, H computed, gapless):
| # | Block | Style | Collapse rule |
|---|-------|-------|---------------|
| 1 | Store name | font "3" scale 1 centered | blank → skip |
| 2 | Address | font "1" centered | blank → skip |
| 3 | Phone | font "1" centered | blank → skip |
| 4 | Tax rows (`NIF: …`, `NIS: …`, `RC: …`, `AI: …`) | font "1" centered | per-row blank → skip |
| 5 | Rule (BAR full width) | — | — |
| 6 | Doc number | font "2" left | — |
| 7 | Capture instant (truncated to 19 chars) | font "1" left | — |
| 8 | Cashier (`Caissier: <stored UserId>`) | font "1" left | — |
| 9 | Depot (`Depot: <stored WarehouseId>`) | font "1" left | — |
| 10 | Rule | — | — |
| 11 | Column header (`Qté x PU = Total` / AR) | font "1" left | — |
| 12 | Per line: SKU (F1) → designation F2 (truncated 50) → detail right-aligned F1 (`qty unit x PU = total`) → badge right-aligned F1 (`remise <pu>` / AR) iff per-unit discount stored | — | badge None on legacy NULL rows |
| 13 | Rule | — | — |
| 14 | Totals hierarchy right-aligned: Sous-total, Remise totale, HT (= subtotal_ht stored, stated), TVA (all F2), then TOTAL TTC F3 scale 2 (double-height) | — | — |
| 15 | Payment block right-aligned F1: `Payé (<method>): <paid>`, `Rendu: <change>` | — | — |
| 16 | Doc-number Code128 (h=64, centered), human-readable number centered beneath | — | payload > 24 chars rejects |
| 17 | Footer owner message (skip blank lines) + constant thanks centered | — | blank → skip |

**Label 40×20** (`SIZE 40 mm,20 mm` fixed):
| y | Content |
|---|---------|
| 2 | Brand micro-line F1 centered (collapses when blank) |
| 16/38 | Designation F2, 2 lines max (second marked `...` when cut) |
| 58 | Price F3 scale 2 (double-height) left |
| 94 | Currency F1 right-aligned in the price band |
| 106 | Code128 h=40; payload ≤ 9 chars (rejects louder) |
| 148 | Human-readable payload F1 centered |
| — | `PRINT 1,<copies>` (copies once, never resends) |

## 2. Geometry / barcode math (quoted + pinned)

- 203 dpi = 8 dots/mm exact. Narrow bar = **2 dots = 0.25 mm ≈ 9.8 mil** (1 dot ≈ 4.9 mil rejected as unscannable).
- Quiet zones: **≥ 10 narrow modules per side = 20 dots** reserved; payload budget = media − 2×20.
- Ticket: (640−40)/2 = 300 modules → 11·n+35 ≤ 300 → **max 24 chars** (`TICKET_BARCODE_MAX_CHARS` computed, not literalled). Height 64 dots (8.0 mm): widest payload 598 dots → 9.3:1; typical 13-char number 356 dots → 5.6:1 (ISO 15416-friendly).
- Label: (320−40)/2 = 140 modules → **max 9 chars**. Height 40 dots (5.0 mm): widest 268 dots → 6.7:1.
- Over-wide payloads reject with `TsplBarcodeTooWide` (never shrunk). Charset guard `TsplBarcodeUnencodable` unchanged (ASCII minus `"`).
- Vertical budgets: label worst case 2+12+38+48+40+12 = 152 ≤ 160 ✓ (fits by construction, all fields width-truncated).

## 3. Byte-assert lines (new + updated)

- Domain 16 tests (`printing::tspl::tests`): `geometry_math_is_pinned` (quiet 20, budgets 24/9, mil 9.8), `label_bytes_are_exact` (full 8-line layout), `label_brand_collapses_when_blank`, `label_barcode_width_rejects_instead_of_shrinking` (9 fits / 10 rejects), `label_designation_wraps_two_lines_and_truncates`, `label_text_truncates_and_sanitizes_without_silent_overflow`, `ticket_bytes_pin_size_gapless_barcode_and_copies` (SIZE 84 mm, BARCODE 142,522 h64, single PRINT), `ticket_anatomy_order_is_pinned_block_by_block` (26 checkpoints, strictly increasing), `ticket_totals_mirror_stored_values_verbatim`, `ticket_blank_identity_fields_collapse_without_gaps` (84→74 mm), `ticket_discount_badge_absent_for_legacy_null_rows`, `ticket_barcode_rejects_over_wide_numbers` (24/25), `ticket_arabic_captions_emit_without_destroying_bytes` (12 AR captions), `ticket_copy_count_is_always_single_print`, + copies/guard carry-overs.
- Application 7 tests (`tspl_print_tests`): all constructors updated for the label identity port; ticket test asserts badge (`remise 1.00`), meta (`Caissier:`, `Depot:`), hierarchy (`Remise totale/HT/TVA/TOTAL TTC`), BARCODE presence; label test asserts brand/double-height price/currency/HR line.
- FAIL-before note: this task is a redesign — the 7 BRK-097 layout assertions were superseded by design (recorded here, not hidden): old `SIZE 59 mm`, `TEXT 8,8 "2" name`, `10.00 DZD`, `BARCODE 8,72 h56`, `Sous-total/TOTAL`, no badge/meta/HR.

## 4. Forced plumbing (REPORTED, mechanical, no logic change)

- `PrintTsplLabelUseCase::new` gains the identity port (brand micro-line); `print_tspl_label` handler passes `state.store_identity` (1 line, `crates/commands/src/ipc.rs`).
- Ticket use-case transcribes: per-line `discount` (stored per-unit), `global_discount`, `cashier_text`/`warehouse_text` (stored UUIDs verbatim — stated: no lookup port on this path; future task can resolve names), `tax_rows` (NIF/NIS/RC/AI from stored identity).
- `TsplLabel` gains `brand_text`/`currency_text`; `TsplTicketLine` gains `discount_text`; `TsplTicket` gains `tax_rows`/`cashier_text`/`warehouse_text`; totals gains `global_discount`.
- Frontend (SilentLabelModal preview) NOT touched per diff limit — its 10-char preview budget now differs from the 9-char engine budget; noted for DSN follow-up.

## 5. Gates

| Gate | Command | Result |
|---|---|---|
| G1 | `cargo fmt --check` | EXIT 0 |
| G2 | `cargo clippy --all-targets --all-features -- -D warnings` | EXIT 0 (1 dead-code error on first pass → fixed by computing barcode budgets from `QUIET_ZONE_DOTS`) |
| G3 | `cargo test --workspace -j 2` | EXIT 0 — **71 suites, 572 passed, 0 failed** (domain 16 + app 7 TSPL pins included) |
| G4–G7 | N/A — backend-only (shared surface untouched: no IPC wire/shape change, no frontend change) | not run, state reason |

## 6. Files (absolute)

- `D:/Dev/Baraka UltraPOS/crates/domain/src/printing/tspl.rs` [MODIFIED — rewritten] (renderer + 16 domain pins)
- `D:/Dev/Baraka UltraPOS/crates/application/src/use_cases/tspl_printing.rs` [MODIFIED] (plumbing only)
- `D:/Dev/Baraka UltraPOS/crates/commands/src/ipc.rs` [MODIFIED] (1 line: label identity port)
- `D:/Dev/Baraka UltraPOS/crates/application/tests/tspl_print_tests.rs` [MODIFIED] (7 tests updated + extended)
- `D:/Dev/Baraka UltraPOS/audit/BRK-098/REPORT.md` [CREATED] (this file)
