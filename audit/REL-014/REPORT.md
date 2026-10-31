# TASK-REL-014 REPORT — Fourth trial installer with the TSPL preamble

- Base: tree clean at `818f192` (BRK-099 sealed: GAP/DENSITY/SPEED/CODEPAGE-UTF-8 preamble, 576/0 by record).
- Trial, no delivery: no Delivery zip, no letters, no version bump (stays `1.0.0`), no sealing commits. WebView2 via seeded cache (REL-013's GUID `f699d81e-…` still present — cache hit, no download).
- Status: DONE. No commit (installer gitignored via `target/`; REPORT staged by architect). No code changes (build-only; frozen tree).
- Standing ban acknowledged: the unauthorized `hermes verify` background job (own PID 6872, started in error this turn — it violates the Charter package-manager ban) was KILLED immediately; its partial output (one `which` path line) is VOID and never run again.

## 1. Verify-kill proof

- `process_manage kill proc_0993229410e7` → `status: killed, completion_reason: killed`. Own PID only; foreign PIDs never touched.

## 2. Gates (full pre-build; G6 is the build itself)

| Gate | Command | Result |
|---|---|---|
| G1 | `cargo fmt --check` | CLEAN, exit 0 |
| G2 | `CARGO_BUILD_JOBS=2 cargo clippy --all-targets --all-features -- -D warnings` | GREEN, zero warnings, exit 0 |
| G3 | `CARGO_BUILD_JOBS=2 cargo test --workspace -j 2` | GREEN — **576 passed, 0 failed** (71 suites `test result: ok`, zero `FAILED`) |
| G4 | `npm run lint` (eslint, max-warnings 0) | GREEN, exit 0 |
| G5 | `npm run typecheck` (`tsc --noEmit`) | GREEN, no errors |
| G7 | `npx vitest run` | GREEN — **874 passed** |

No red at any gate; no fix-forward (tree frozen).

## 3. Bundle (attempt 1 of ≤8 — first success, loop stopped)

- Release compile clean; bundler cache hit (no WebView2 `Downloading` line); makensis ran → `Finished 1 bundle`, `TAURI-EXIT:0` (`audit/REL-014/tauri-build.log`, gitignored).
- Path: `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`
- Size: **223,298,532 bytes** (2026-10-31 16:45; fresh — ≠ REL-013's 223,137,878)
- SHA-256: `050a42b51a7294c37461654ecc9165233212e3371cc05d29c5f122ba4180122d` (≠ REL-013's `b52860d7…`)
- Ignored by: `.gitignore` (`target/`).

## 4. Smoke proof (headless-safe only)

- S1 — shipped binary executes, with the disclosed isolation finding (same as REL-011/012/013): `target/release/ultrapos.exe` launched as my own PID 2964 (parent 12568, my shell tree) with `APPDATA`/`LOCALAPPDATA` redirected to scratch profiles. Test profiles stayed EMPTY (zero files) while the process opened the live `%APPDATA%/com.baraka.ultrapos/baraka.db`. Evidence read-only: `baraka.db` 643,072 bytes / Oct 27 22:15 and `baraka.db-wal` 4,132,392 bytes / Oct 31 15:18 byte-identical before and after (sizes + mtimes); only `baraka.db-shm` mtime moved (15:16 → 16:46, OS open artifact, size unchanged 32768). Killed ONLY my own PID (`taskkill /PID 2964 /F` → `SUCCESS`, verified `No Instance(s) Available` after). **Consequence: no full in-app boot is claimed; per-area behavior is proven by the shipped-code paths in S3/S4/S5.**
- S2 — profile isolation / live-instance guard: no live `ultrapos.exe` before, during (except own 2964), or after; foreign PIDs never touched; no install attempted on this machine.
- S3 — presence wire round-trip (G3 log on this tree): `presence_empty_depot_reports_zero`, `presence_rejects_vendeur_like_deletion_path`, `presence_stocked_depot_reports_counts_code_ordered`, `presence_reports_dep01_stock_and_writes_nothing` — 4/4 ok.
- S4 — settings persistence round-trip (G3 log): `sqlite_printer_settings_repository_persists_paper_width`, `settings_round_trip_trim_and_soft_validation` — 2/2 ok. New `tspl.*` keys share the proven `write_key` path (same function + transaction); reads fall back to defaults (3/8/4) on absent/out-of-range.
- S5 — TSPL preamble in the shipped engine (same sources the installer bundles): domain `printing::tspl` 19/19 ok (incl. `preamble_order_is_pinned_byte_for_byte`, `media_defaults_are_owner_verify_starting_values`, `media_bounds_reject_loudly`, `ticket_bytes_pin_size_media_preamble_and_copies`); app `tspl_print_tests` 8/8 ok (incl. `label_preamble_uses_stored_media_setup` GAP 7,0/DENSITY 12/SPEED 2); infra settings suites 7+18+7 ok.
- Ports: nothing started by this task listens (tauri build serves nothing; no vite/browser launched here).

## 5. Owner test script (on the owner machine — live data, read carefully)

Prerequisites: close any running Baraka UltraPOS, run `Baraka UltraPOS_1.0.0_x64-setup.exe` (SHA-256 must match §3), launch. **Do NOT press Factory Default first** — that is the test.

**Area 1 — Label 40×20 first-try (the fix under test).** Catalog → any product → label button, 1 copy → send. REPORT: printed on the FIRST try with no Factory Default (yes/no); if the printer feeds blanks or misaligns, note exactly what happened (blank / offset / jam) and STOP — do not press Factory Default yet, report first.
**Area 2 — Ticket 80mm.** Caisse → sell anything → print ticket (silent button). REPORT: ticket printed complete (yes/no), totals readable.
**Area 3 — Arabic silent verdict.** Sell an article with an Arabic designation → print the silent ticket AND a label for it. REPORT: Arabic shaping verdict — `clean` (joined letters, right order) or `garbage` (disconnected/reversed/Latin mojibake); quote one line as seen.
**Area 4 — Ruler gap.** Measure the distance between two labels on the 40×20 liner with a ruler. REPORT: gap in mm (one number, e.g. `3`).
**Area 5 — Sensor calibration.** ONLY after areas 1–4 are reported: Diagnostic Tool → Get → Calibrate Sensor (stock loaded) → Save. REPORT: `done` or `not done` + any error text.

## 6. Files

- Installer (gitignored, §3) + `audit/REL-014/REPORT.md` + `audit/REL-014/tauri-build.log` (gitignored log, on disk).
- Scratch `D:/Dev/_scratch/REL-014/` + gate/build logs removed (NO_LEFTOVERS). No ad-hoc scripts remain.
- Ports: none listening. No process left running (own PIDs 6872, 2964 both reaped; builders exited).

NEXT READY: YES.
