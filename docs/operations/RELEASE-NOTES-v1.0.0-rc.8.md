# Release Notes — Baraka UltraPOS v1.0.0-rc.8

- Release Candidate: **v1.0.0-rc.8**
- Date: 2026-10-16
- Git tag: `v1.0.0-rc.8` (annotated; `v1.0.0` remains prohibited by Charter §3.1)
- Baseline: commits `fc64044` (BAT-071 dual-width engine + ARC-017 boot
  package) and `4114ea6` (BAT-072 worker threading + settings UI + ADR-091)

## Verification metrics

- Rust workspace: **443 passed / 0 failed** (`cargo fmt --check` clean,
  `cargo clippy --all-targets --all-features -D warnings` clean), including
  **7 dual-width integration tests** in
  `crates/infrastructure/tests/dual_width_thermal_printing.rs` and
  **2 shaping regression guards** (`shaping_advances_are_physical_pixels`,
  `long_designation_wraps_on_both_canvases`).
- Frontend: **527/527 Vitest** across 49 files, `eslint --max-warnings 0`
  clean, `tsc --noEmit` clean, `vite build` clean.
- Installer: NSIS `Baraka UltraPOS_1.0.0_x64-setup.exe`; SHA-256 recorded
  in `release-v1.0.0/SHA256SUMS.txt`; delivery archive
  `Baraka-UltraPOS-v1.0.0-rc.8-Delivery.zip` checksummed alongside.

## Highlights

1. **Dual-width ESC/POS thermal receipt engine** — 80 mm standard
   (576 dots / 72-byte stride) and 58 mm compact (384 dots / 48-byte
   stride), calibrated per width with automatic layout reflow.
2. **Arabic font shaping advance fix** (`shaping.rs`) — removed the
   extraneous `/ 64.0` divisor (rustybuzz returns unscaled font design
   units, not 26.6 fractional points), restoring true physical-pixel
   metrics and Arabic line-wrapping on narrow paper.
3. **Dynamic paper-width persistence** — `printer.paper_width_mm` stored
   in SQLite and threaded live into the print worker; layout and Arabic
   text wrapping adjust automatically to the selected roll.
4. **Settings UI enhancement** — radio/segmented control for receipt
   paper width (58 mm / 80 mm) in Settings → Impression with instant
   persistence (FR/AR).
5. **ADR-091 filed** — dual-width thermal printing and shaping
   calibration recorded ACCEPTED (next: ADR-092).
6. **Offline handbook updated** — `GUIDE-UTILISATION.html` carries the
   rc.8 badge and a new bilingual Section 10 documenting thermal printer
   width configuration; fully self-contained, zero network dependencies.

## Open gates (owner)

- Desktop re-verification of 58 mm / 80 mm physical print output on real
  hardware (test-print diagnostic makes success self-evident).
- At-rest encryption remains an open owner decision (unchanged).
