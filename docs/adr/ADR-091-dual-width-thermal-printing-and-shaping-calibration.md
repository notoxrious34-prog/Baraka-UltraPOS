# ADR-091: Dual-Width (58mm/80mm) ESC/POS Thermal Pipeline and Font-Shaping Calibration

Status: ACCEPTED
Date: 2026-10-16
Session: TASK-BRK-070 / TASK-BAT-071 / TASK-BAT-072 (dual-width calibration, worker threading, settings UI)

## Context

Charter v2.0 §3.4 recorded Engineering Debt #6: `PAPER_WIDTH_DOTS=576`
calibrated for 80mm, 58mm uncalibrated. Compact 58mm thermal printers
(203 DPI, 384 printable dots, ~32 columns) are widespread in small Algerian
retail; the raster pipeline hardcoded a 576-dot surface, so 58mm hardware
clipped ~33% of every receipt. Exact geometry is mandatory: 80mm = 576 dots
at a 72-byte (`0x48`) GS v 0 stride, 58mm = 384 dots at a 48-byte (`0x30`)
stride.

While calibrating, an integration probe exposed a deeper pre-existing defect
in `crates/infrastructure/src/printing/shaping.rs:137` (present since
baseline `05f0f39`): `shape_run` computed `scale = px_size / upem / 64.0`.
rustybuzz 0.18 reports advances in FONT UNITS, not 26.6 fixed point
(measured: `x_advance` 636/253/360/387/456 on a 1000-upem Noto face; 1443
for Latin `A` on a 2048-upem Inter face — both ≈0.6em). The extra `/64.0`
compressed every line ~64× (63 Latin chars at 24px measured 11.53px;
100 Arabic chars 15.69px): text rasterized as an ink blob and line wrapping
could never trigger (4×400 Arabic chars still rendered 58mm == 80mm).
Evidence: `crates/infrastructure/tests/dual_width_thermal_printing.rs`
guards `shaping_advances_are_physical_pixels` and
`long_designation_wraps_on_both_canvases`, shown FAIL-before
(`0.183 px/char`, `58mm must wrap (550) above 80mm (550)`) then green.

## Decision

1. **Dynamic `PaperWidth` domain enum.** `crates/domain/src/printing/paper.rs`:
   `Width80mm` (576 dots / 42 cols, default) vs `Width58mm` (384 dots /
   32 cols); `from_mm` (≤58 → 58mm), `as_mm`, strict
   `validate_paper_width_mm` (only 58|80, else
   `DomainError::InvalidPaperWidthMillimeters`). No floats anywhere.
2. **Width-parameterized raster core.** `render_canvas`, `render_debt_canvas`,
   `render_lines`, `build_lines`, identity/footer builders all take
   `PaperWidth`; canvas width, margins, LTR/RTL origins, rules derive from
   `dot_width()`. `layout_lines` wraps on whitespace by re-shaping word
   groups (Arabic ligatures never split); oversize single words emit alone,
   never panic.
3. **Per-job worker resolution.** `PrintWorker` (`worker_thread.rs`
   `WorkerDeps.printer: Arc<dyn PrinterSettingsPort>`) resolves
   `PaperWidth::from_mm` from persisted `printer.paper_width_mm` on every
   job via `process_one_job_with_identity`; settings failure falls back to
   80mm without felling the job. `ReceiptRendererPort` gains defaulted
   `render_with_paper` / `render_debt_receipt_with_paper` (fakes keep
   compiling); `EscPosRasterizer` overrides with the width-aware canvas.
   Legacy `process_one_job` renders the 80mm default explicitly.
4. **Settings UI toggle.** `PrinterSettingsTab` exposes a bilingual paper
   selector (`settings.paperWidth80` = `80 mm (Standard / عريض)`,
   `settings.paperWidth58` = `58 mm (Compact / مدمج)`), persisted through
   `printer_update_settings.paper_width_mm` (58|80); `None` keeps stored
   width for older callers.
5. **Shaping calibration.** `scale = px_size / upem` (the `/64.0` removed).
   All print suites pass unchanged; no byte goldens existed to re-baseline.

## Consequences

Receipts print with bit-exact strides on both stocked formats, long Arabic
designations wrap instead of clipping, and merchants switch hardware from
Settings without restart. Deferred: nothing blocking; live per-job selection
is the production path as of this ADR.

Validation gate: dual-width suite 7/7 green (widths, strides, wrap,
settings round-trip, RBAC, zero-write rejection) + `print_worker` 15/15
incl. `worker_honors_configured_58mm_paper_width` (48-byte stride through
the real worker path). Owner gate: print one real 58mm and one real 80mm
ticket and confirm legibility.
