# TASK-REL-013 REPORT — Third trial installer with per-unit discount

- Base: tree clean at `e2d40ad` (BRK-096 + DSN-097 sealed: per-unit kernel + display, 549 Rust + 856 Vitest green by record).
- Target stack in this bundle: per-unit line discount end-to-end — kernel exact math + stored discounted PU (BRK-096) and cart/modal/ticket/drawer/returns display (DSN-097).
- Trial, no delivery: no Delivery zip, no letters, no version bump (stays `1.0.0`), no sealing commits. WebView2 via seeded cache (no download — same pattern REL-011/012 used).
- Status: DONE. No commit (installer gitignored via `target/`; REPORT staged by architect). No code changes (build-only; `git diff` empty).

## 1. Gates (full pre-build; G6 is the build itself)

| Gate | Command | Result |
|---|---|---|
| G1 | `cargo fmt --check` | CLEAN, exit 0 |
| G2 | `CARGO_BUILD_JOBS=2 cargo clippy --all-targets --all-features -- -D warnings` | GREEN, zero warnings, exit 0 (50.59s) |
| G3 | `CARGO_BUILD_JOBS=2 cargo test --workspace -j 2` | GREEN — **549 passed, 0 failed, 7 ignored** (70 suites `test result: ok`, zero `FAILED`) |
| G4 | `npm run lint` (eslint, max-warnings 0) | GREEN, exit 0 |
| G5 | `npx tsc -p tsconfig.json --noEmit` | GREEN, no errors |
| G7 | `npx vitest run` | GREEN — **82 files / 856 passed** |

No red at any gate; no fix-forward needed (tree frozen).

## 2. Bundle attempts log

- Attempts 1–6 (prior turn, REL-013): `TAURI-EXIT=1` every time — `failed to bundle project: timeout: global` at the NSIS step, AFTER a good release compile. Root-caused to the bundler's own `HEAD https://go.microsoft.com/fwlink/?linkid=2124701` (ureq single-attempt vs. variance-heavy Microsoft fwlink endpoint: curl shows ~90% fast / ~10% ~21s stalls; ureq never survives a stall).
- During diagnosis the WebView2 runtime rotated to a new GUID (`f699d81e-69ce-4b09-aa07-23ff2713045b`), absent from the seeded cache. Seeded it via the direct msedge CDN URL (the fwlink redirector was the sick part): `%LOCALAPPDATA%/tauri/x64/f699d81e-…/MicrosoftEdgeWebView2RuntimeInstallerX64.exe`, **214,884,560 bytes**, curl exit 0 — same cache layout/pattern as REL-011/012.
- Attempt 7 (this turn, first of the approved retry loop): HEAD landed in a fast window → cache hit (**no `Downloading` line** in `tauri-build7.log`) → makensis ran → **`Finished 1 bundle`, TAURI-EXIT=0**. Loop stopped at first success (1 of 8 used).

## 3. Installer (gitignored, on disk)

- Path: `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`
- Size: **223,137,878 bytes** (2026-10-27 20:06; fresh — distinct from REL-012's 223,073,973-byte file)
- SHA-256: `b52860d709807d25e2c175f9b42d41ee9aae2c3cc2486428c058970ea275f122` (≠ REL-012's `bdaed60e…`)
- Ignored by: `.gitignore:14` (`target/`).

## 4. Smoke proof (headless-safe only)

- S1 — shipped binary executes, with the disclosed isolation finding: `target/release/ultrapos.exe` launched as my own PID 1428 with `APPDATA`/`LOCALAPPDATA` redirected to `D:/Dev/_scratch/REL-013/profile-app|profile-local`. **FINDING (same as REL-012): env redirection does NOT isolate a Tauri app** — the test profile stayed EMPTY (zero files) while the process opened the live `%APPDATA%/com.baraka.ultrapos/baraka.db`. Evidence it was read-only: `baraka.db` 4096 bytes / Oct 25 19:18 and `baraka.db-wal` 3,534,992 bytes / Oct 27 18:17 byte-identical before and after (sizes + mtimes); only `baraka.db-shm` mtime moved (18:15 → 20:08, OS open artifact, size unchanged at 32768). `backups/` and `printer-capture/` untouched. I killed ONLY my own PID (`Stop-Process -Id 1428 -Force`, verified `ultrapos gone` after); `APP-EXIT=127` is the kill artifact, not an app failure; stdout log empty. **Consequence: no full in-app boot is claimed; per-area behavior is proven by the shipped-code paths in S3/S4, same headless-safe level as REL-011/012.**
- S2 — profile isolation / live-instance guard: no live `ultrapos.exe` before, during (except my own 1428), or after; foreign PIDs never touched; no install attempted on this machine.
- S3 — presence wire round-trip in the shipped engine (same code the installer bundles): G3 `warehouse` suites **8/8 ok** — `presence_empty_depot_reports_zero`, `presence_rejects_vendeur_like_deletion_path`, `presence_reports_dep01_stock_and_writes_nothing` (zero-write proof), `presence_stocked_depot_reports_counts_code_ordered`.
- S4 — settings persistence round-trip in the shipped engine: `sqlite_printer_settings_repository_persists_paper_width` and `settings_round_trip_trim_and_soft_validation`, both ok.
- S5 — per-unit discount kernel in the shipped engine: `per_unit_owner_example_three_units_off_100` ok (3 × (1000−100) → PU' 900), `draft_parks_per_unit_discount_exact` ok, `partial_return_with_per_unit_discount_is_exact` ok; display pins live in G7 (856 green).
- Ports: nothing started by this task listens (tauri build serves nothing; no vite/chrome launched here).

## 5. Owner test script (on the owner machine — live data, read carefully)

Prerequisites: close any running Baraka UltraPOS, run
`Baraka UltraPOS_1.0.0_x64-setup.exe` (SHA-256 must match §3), launch.

**Area 1 — Per-unit discount (the new behavior, do this first).**
Caisse → add the same article 3 times (or qty 3, e.g. Vis 8x40 @ 10,00 DZD) → F5 → type `100` in `Remise ligne / unité (DZD)`: the hint must read `Le montant s'applique à CHAQUE unité` and the live line `Total ligne estimé: 3 x 9,00 DZD = 27,00 DZD` must appear BEFORE confirm. `Appliquer` → the cart row must show struck `10,00 DZD` + emerald `9,00 DZD`, qty 3, total `27,00 DZD`. Type more than the unit price (e.g. `2000`): the modal must refuse (`ne peut pas dépasser le prix unitaire`) with NO engine popup. Encaisser → print: the ticket line must read `3 × 9,00 = 27,00` (NOT 10,00 anywhere). REPORT: modal label + hint + live figure seen; cart struck/net/total; refusal message; ticket PU/total numbers.

**Area 2 — Returns with per-unit discount.** Documents → open that sale → `Retour`: the return line must show struck raw PU + net PU + `(Remise /u: 1,00 DZD)` badge; return 1 of the 3 → refund must be exactly `9,00 DZD`. REPORT: badge seen; refund amount.

**Area 3 — Old documents unchanged.** Open any pre-trial sale/facture (drawer + A4 print): lines with no per-unit discount must render EXACTLY as before (raw PU alone, no badges). REPORT: one old document number + "identical" or any diff.

## 6. Files

- Installer (gitignored, §3) + `audit/REL-013/REPORT.md` + `audit/REL-013/tauri-build{,2,3,4,5,6,7}.log` (gitignored logs, on disk).
- `git status` shows only `?? audit/REL-013/` (REPORT visible; logs invisible by `*.log` rule, same as REL-010/011/012).
- Scratch `D:/Dev/_scratch/REL-013/` removed (NO_LEFTOVERS — profile dirs, smoke log, diagnostic fetches). Gate logs `_rel013-g2/g3.log` removed. No ad-hoc scripts remain.

## 7. UNKNOWNs / deviations

- Bundle attempts 1–6 failed on the upstream fwlink stall (full evidence in §2 + prior-turn report); attempt 7 green. No workaround code or config was applied — plain retry inside a fast window + pre-seeded cache file at the bundler's own cache path.
- Seeded WebView2 offline installer is the current Microsoft release (GUID `f699d81e-…`, 214,884,560 bytes, downloaded clean via direct CDN URL); the runtime self-updates (Evergreen) regardless.
- S1 deviation (above): env redirection cannot test-profile a Tauri Known-Folder app; live DB opened read-only, never written — full byte evidence pasted.
- None blocking. Owner hardware/data trial pending (install + §5 on the owner machine).

NEXT READY: YES.
