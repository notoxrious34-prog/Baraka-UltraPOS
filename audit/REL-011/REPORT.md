# TASK-REL-011 REPORT — Trial installer with USB + graphique stack

- Base: tree clean at `225b747` (DSN-093 sealed, 797/797). Target stack in this bundle: BRK-089 (USB spooler RAW), BRK-090 (graphique mode + `FRONTEND_RENDERED`), DSN-091 (queue UI), DSN-093 (80mm ticket + honest banner).
- Trial, not delivery: no Delivery zip, no OPEN-GATES letters, no sealing commits, no version bump. WebView2 via previously seeded cache (no download delay).
- Status: DONE. No commit (installer gitignored via `target/`; REPORT staged by architect). No code changes (build-only). Live DB never touched.

## 1. Gates (full, pre-build)

| Gate | Command | Result |
|---|---|---|
| G1 | `cargo fmt --check` (`CARGO_BUILD_JOBS=2`) | CLEAN, exit 0 |
| G2 | `cargo clippy --all-targets --all-features -- -D warnings` | GREEN, zero warnings, exit 0 |
| G3 | `cargo test --workspace -j 2` | GREEN — **515 passed, 0 failed** |
| G4 | `npm run lint` (eslint, max-warnings 0) | GREEN, exit 0 |
| G5 | `npx tsc -p tsconfig.json --noEmit` | GREEN, no errors |
| G7 | `npx vitest run` | GREEN — **797 passed, 0 failed** (79 files) |

No red at any gate; no fix-forward needed (tree frozen).

## 2. Build lines

- `npx tauri build` (log `audit/REL-011/tauri-build.log`, gitignored by `*.log` convention) — full log on disk.
- Frontend rebuilt (`vite build`, ~6s); Rust release compiled; `ultrapos.exe` patched with NSIS bundle info; `makensis` produced the bundle with **no `Downloading` line** (seeded WebView2 bootstrapper reused).
- `TAURI-EXIT=0`, `Finished 1 bundle`.

## 3. Installer (gitignored, on disk)

- Path: `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`
- Size: **223,027,056 bytes** (2026-10-25 12:39; distinct from the rc.11-era 223,014,545-byte file it replaces)
- SHA-256: `11f356d26b85398e837eef1cd9907c1ee6e1195f74806e8dfd30287781a4b1a8`
- Ignored by: `.gitignore:14` (`target/`).

## 4. Smoke proof (headless-safe only)

- S1 — shipped binary executes: `target/release/ultrapos.exe` launched with redirected `APPDATA`/`LOCALAPPDATA` (scratch test profile, never the live dirs) → process ran early init and exited `APP-EXIT=0` (`D:/Dev/_scratch/REL-011/smoke-stdout.log`, scratch removed after capture).
- S2 — profile isolation: live dirs `%APPDATA%/com.baraka.ultrapos` and `%LOCALAPPDATA%/com.baraka.ultrapos` **do not exist on this box** (verified before and after — nothing created, nothing touched).
- S3 — live-instance guard (constraint discovered during smoke): PID 3368 = `D:\Programs\Baraka UltraPOS\ultrapos.exe` (foreign install, separate session) is RUNNING. The app's single-instance guard (`src-tauri/Cargo.toml:22`, `src-tauri/src/lib.rs:78-80`) means any launch hands off to it — so no full boot or install was attempted here (would have stolen focus / risked live state). The `APP-EXIT=0` handoff itself proves the fresh binary loads and runs through init. **Do not kill PID 3368; do not run the installer on this machine while it lives.**
- S4 — settings round-trip in test profile: proven by G3 (`spooler_transport_tests`: `ticket_render_mode` defaults/round-trip/None-keeps + invalid-mode rejection with zero writes) — same Rust code path the installer ships.
- S5 — graphique ticket HTML generation: proven by G7 (9 template pins executing the exact bundled TS: wire verbatim totals, DUPLICATA on/off, 80mm `@page`, RTL + LTR numbers, escaping).
- S6 — installer integrity: size + SHA-256 recorded above; NSIS self-verifies CRC at install time on the owner machine.
- Ports: no listener left behind (`:5173` free after capture vite was stopped in DSN-093-C1; `tauri build` serves nothing).

## 5. Owner test script (on the owner machine, with the owner's printer)

1. Close any running Baraka UltraPOS, then run `Baraka UltraPOS_1.0.0_x64-setup.exe` (SHA-256 must match §3) and launch.
2. Paramètres → Impression & Matériel → transport **USB direct**, queue = exact Windows name (e.g. `Xprinter XP-410B`) → Enregistrer.
3. Same tab → **Ticket graphique 80mm** → Enregistrer (note: driver paper size must be 80mm).
4. Test d'impression (FR): honest banner reads — `… sent …` = bytes reached the spooler; `… capture-only …` = switch device kind to physique; any other text (red) = the spooler call + OS code, report it verbatim.
5. Make a real sale (graphique mode): the 80mm ticket must print via the OS path automatically. If paper stays blank, report: (a) banner text from step 4, (b) whether Windows test page prints, (c) spooler queue state (paused/offline?).
6. Reprint path: Documents → ticket → **Imprimer le ticket** (graphique, DUPLICATA-marked) vs engine DUPLICATA (ESC/POS mode only).

## 6. Files

- Installer (gitignored, §3) + `audit/REL-011/REPORT.md` + `audit/REL-011/tauri-build.log` (gitignored log, on disk). `git status` shows only `?? audit/REL-011/` (REPORT; log invisible by `*.log` rule, same as REL-010).
- Scratch `D:/Dev/_scratch/REL-011/` removed (NO_LEFTOVERS). No ad-hoc scripts remain.

## 7. UNKNOWNs

- None blocking. Owner hardware trial pending (install + steps §5 on the owner machine with the XP-410B).

NEXT READY: YES
