# Release Notes — Baraka UltraPOS v1.0.0-rc.5

- Release Candidate: **v1.0.0-rc.5**
- Date: 2026-10-14
- Git tag: `v1.0.0-rc.5` (annotated; `v1.0.0` remains prohibited by Charter §3.1)
- Baseline: commit `8c32f4c` + rc.5 packaging commit

## Verification metrics

- Rust workspace: **422 passed / 0 failed** (52 suites), `cargo fmt --check`
  clean, `cargo clippy --workspace --all-targets --all-features -D warnings`
  clean — including `sale_preview_commit_parity` (3/3) and `sale_atomic`
  (19/19).
- Frontend: **502/502 Vitest** across 45 files, `eslint --max-warnings 0`
  clean, `tsc --noEmit` clean, `vite build` clean.
- Installer: NSIS `Baraka UltraPOS_1.0.0_x64-setup.exe` built with
  `npm run tauri build` (exit 0, offline WebView2 bundled); SHA-256 recorded
  in `release-v1.0.0/SHA256SUMS-rc.5.txt`.

## The 5 pillars + official brand

1. **Taxonomy & starter pack (DSN-044)** — 7 standard hardware categories,
   pill-strip filtering, category manager, starter taxonomy seeded.
2. **Phonetic & hardware search (BRK-066)** — Arabic/French dialect-tolerant
   normalization (72-entry dictionary), infix + barcode search.
3. **Multi-unit sales cockpit (DSN-046)** — F3 unit cycling
   (pièce/boîte/sac/kg), base-quantity equivalence badges, barcode wedges.
4. **Artisan & wholesale tier pricing + Margin Guard (BRK-068 / DSN-047,
   ADR-087)** — per-customer tier (Détail / Artisan / Gros), automatic cart
   re-pricing, per-unit wholesale price, integer-exact margin with pulsing
   `⚠️ Vente à perte / بيع بالخسارة` below-cost alert, tier badges in F2 and
   the POS customer chip.
5. **Atomic CSV catalog import (BRK-067 / DSN-045)** — `;`-first parser
   (BOM/quotes/CRLF-safe), 4-stage preview modal, skip-on-duplicate-SKU.
6. **Official "Baraka" identity** — gold 3D logo de-matted with zero fringe,
   embedded in TopBar, NavRail, login, About tab, and Windows app icons
   (multi-resolution ICO + full PNG set).

## Offline handbook

`release-v1.0.0/GUIDE-UTILISATION.html` gains the §7
"Version 1.0.0-rc.5 — Nouveautés" section (French + Arabic, inline CSS only,
zero network calls).
