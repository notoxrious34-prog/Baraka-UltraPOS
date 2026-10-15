# Release Notes — Baraka UltraPOS v1.0.0-rc.6

- Release Candidate: **v1.0.0-rc.6**
- Date: 2026-10-14
- Git tag: `v1.0.0-rc.6` (annotated; `v1.0.0` remains prohibited by Charter §3.1)
- Baseline: commit `a1c81fa` (BRK-069) + rc.6 packaging commit

## Verification metrics

- Rust workspace: **422 passed / 0 failed** (52 suites), `cargo fmt --check`
  clean, `cargo clippy --workspace --all-targets --all-features -D warnings`
  clean — including the `ultrapos` harness with the single-instance plugin.
- Frontend: **505/505 Vitest** across 46 files (incl. `starter-pack.test.ts`
  3/3), `eslint --max-warnings 0` clean, `tsc --noEmit` clean,
  `vite build` clean.
- Installer: NSIS `Baraka UltraPOS_1.0.0_x64-setup.exe` built with
  `npm run tauri build` (exit 0, offline WebView2 bundled); SHA-256 recorded
  in `release-v1.0.0/SHA256SUMS-rc.6.txt`.

## Highlights

1. **Single-instance window protection** — a second launch focuses,
   unminimizes, and shows the running POS instead of starting a rival
   process (no more SQLite lock contention or printer-port conflicts).
2. **300-item quincaillerie starter catalog** — authentic dual-language
   Algerian hardware dataset across all 7 starter categories, 77%
   checksum-verified EAN-13 barcodes, 15–40% commercial margins.
3. **One-click catalog loading** — `✦ Pack Quincaillerie (300)` in the
   catalog toolbar preloads the atomic import modal: preview, validate,
   import in under 30 seconds.
4. **Full official branding** — gold Baraka identity across shell, login,
   About tab, and Windows app icons (rc.5 baseline retained).
