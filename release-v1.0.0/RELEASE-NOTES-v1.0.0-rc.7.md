# Release Notes — Baraka UltraPOS v1.0.0-rc.7

- Release Candidate: **v1.0.0-rc.7**
- Date: 2026-10-15
- Git tag: `v1.0.0-rc.7` (annotated; `v1.0.0` remains prohibited by Charter §3.1)
- Baseline: commits `1d929be` (REL-055 forensic drill + CSV repair) and
  `b40e44e` (DSN-048 cockpit compaction + Data Exchange Hub)

## Verification metrics

- Rust workspace: **435 passed / 0 failed** (53 suites: 424 baseline + 11
  forensic-drill tests), `cargo fmt --check` clean,
  `cargo clippy --all-targets --all-features -D warnings` clean.
- Frontend: **526/526 Vitest** across 49 files (505 baseline + 21 new:
  deferred-lifecycle download contract, exchange-menu, cockpit pins),
  `eslint --max-warnings 0` clean, `tsc --noEmit` clean, `vite build` clean.
- Installer: NSIS `Baraka UltraPOS_1.0.0_x64-setup.exe`; SHA-256 recorded
  in `release-v1.0.0/SHA256SUMS.txt`; delivery archive
  `Baraka-UltraPOS-v1.0.0-rc.7-Delivery.zip` checksummed alongside.

## Highlights

1. **Hardened desktop CSV download** — template + catalog export now use
   a deferred-URL-lifecycle helper (30 s revocation window) so WebView2's
   asynchronous pickup never races a dead `blob:` URL; UTF-8 BOM kept so
   Excel opens Arabic text flawlessly; inline filename confirmation.
2. **Unified Data Exchange Hub** — one "Échanges & Données" menu (import
   CSV/TXT, 300-item starter pack, Excel CSV export, standard template)
   replaces scattered header buttons; Esc/outside-click close, FR/AR.
3. **Caisse cockpit compaction** — Topbar 56 px → 40 px, duplicate brand
   removed (16 px reclaimed for an extra POS tile row); all session, depot,
   and operator widgets preserved in order.
4. **Fast-sale tiles** — whole tile is the click target (no more "Modifier"
   distraction); live stock chips (`{qty} {unit}`, red on zero).
5. **Empty-cart operator HUD** — scan/click guidance plus
   `[F1] Recherche` · `[F2] Client` · `[F12] Encaisser` shortcut chips.
6. **Responsive catalogue grid** — fixed 11-column percentage layout,
   zero horizontal overflow at ≥ 1024 px; pill strip scrolls without a
   native scrollbar thumb.
7. **10-pillar forensic drill, 11/11 green** — till, pricing/margin guard,
   stock, credit, documents + returns, suppliers, reports, import,
   cash session, RBAC, settings/backup; engine rules pinned from observed
   behavior (timbre floor, exact-tender credit, terminal-scoped payments).
8. **ADR-090 filed** — Data Exchange Hub, fast-sale optimization, and
   responsive layout recorded ACCEPTED; ADR index advanced (next: ADR-091).

## Open gates (owner)

- Desktop re-verification of CSV save + exchange menu on real hardware
  (inline filename note makes success self-evident).
- At-rest encryption remains an open owner decision (unchanged).
