# Release Notes — Baraka UltraPOS v1.0.0-rc.9

- Release Candidate: **v1.0.0-rc.9**
- Date: 2026-10-17
- Git tag: `v1.0.0-rc.9` (annotated; `v1.0.0` remains prohibited by Charter §3.1)
- Baseline: commits `0c7ae33` (BRK-073 intelligent catalog search),
  `acdee5f` (DSN-049 scanner ergonomics) and `ef0270c` (REL-058
  hardware drill)

## Verification metrics

- Rust workspace: **453 passed / 0 failed** (`cargo fmt --check` clean,
  `cargo clippy --all-targets --all-features -D warnings` clean),
  including 7 catalog-search enhancement tests and the 300-item
  hardware drill (3 tests + 1 release-mode latency benchmark).
- Frontend: **537/537 Vitest** across 50 files (527 baseline + 10 new
  scanner-ergonomics tests), `eslint --max-warnings 0` clean,
  `tsc --noEmit` clean, `vite build` clean.
- Installer: NSIS `Baraka UltraPOS_1.0.0_x64-setup.exe`; SHA-256 recorded
  in `release-v1.0.0/SHA256SUMS.txt`; delivery archive
  `Baraka-UltraPOS-v1.0.0-rc.9-Delivery.zip` checksummed alongside.

## Highlights

1. **Intelligent catalog search** — infix token matching anywhere in the
   name (`douchette` finds `Pomme de douchette`), multi-term AND
   conjunction (`vis agglo 4x35`), Rank 0 exact barcode prioritization
   (product and unit codes), and Arabic orthographic tolerance via
   single-character wildcards (alef hamza forms, taa marbuta, yaa).
2. **POS cockpit scanner & ergonomics** — auto-focus wedge redirection
   (keystrokes landing outside inputs flow into the search bar with the
   first character preserved), instant Enter-to-cart barcode dispatch
   (add + clear + refocus), and WebView2 shortcut discipline including
   F11 fullscreen suppression.
3. **300-item hardware benchmark** — realistic bilingual quincaillerie
   dataset (Visserie, Plomberie, Outillage, Électricité, Peinture);
   release-mode p95 search latency **10.47 ms** (budget < 15 ms);
   two-item atomic cash sale verified (FINALIZED document, SALE_OUT
   movements, PENDING receipt print job, exact change arithmetic).
4. **Offline handbook updated** — `GUIDE-UTILISATION.html` carries the
   rc.9 badge and a new bilingual Section 11 on smart search and rapid
   barcode scanning; fully self-contained, zero network dependencies.

## Open gates (owner)

- Desktop re-verification of wedge-scanner hands-free flow and 58/80 mm
  physical print output on real hardware.
- At-rest encryption remains an open owner decision (unchanged).
