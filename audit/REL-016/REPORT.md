# TASK-REL-016 REPORT — Sixth trial installer with the probe selector

- Base: tree clean at `9ffc7d2` (DSN-100 sealed: 4-mode TEMPORARY selector,
  882/0 by record, 2 shots). No code changes (build-only; frozen tree).
- Trial, no delivery: no Delivery zip, no letters, no version bump (stays
  `1.0.0`), no sealing commits. WebView2 via seeded cache (cache hit).
- Status: DONE. No commit (installer gitignored via `target/`; REPORT staged by
  architect).

## 1. Gates (full pre-build; G6 is the build itself)

| Gate | Command | Result |
|---|---|---|
| G1 | `cargo fmt --check` | CLEAN, exit 0 |
| G2 | `CARGO_BUILD_JOBS=2 cargo clippy --all-targets --all-features -- -D warnings` | GREEN, zero warnings, exit 0 |
| G3 | `CARGO_BUILD_JOBS=2 cargo test --workspace -j 2` | GREEN — **582 passed, 0 failed** (71 `test result: ok` suites, zero nonzero-`failed` lines), EXIT 0 |
| G4 | `npm run lint` (eslint, max-warnings 0) | GREEN, exit 0 |
| G5 | `npm run typecheck` (`tsc --noEmit`) | GREEN, no errors |
| G7 | `npx vitest run` | GREEN — **85 files, 882 passed** |

No red at any gate; no fix-forward (tree frozen).

## 2. Bundle (attempt 1 of ≤8 — first success, loop stopped)

- Release compile clean; bundler cache hit (no WebView2 `Downloading`); makensis
  ran → `Finished 1 bundle`, `TAURI-EXIT:0` (`audit/REL-016/tauri-build.log`, gitignored).
- Path: `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`
- Size: **223,306,717 bytes** (2026-10-31 19:24; fresh — ≠ REL-015's 223,305,302)
- SHA-256: `0aa4ae510a0d011b73169d26bf3f23c4484d1d956e20b37a36fb76f4d1a85112` (≠ REL-015's `9a35bb9b…`)
- Ignored by: `.gitignore` (`target/`).

## 3. Smoke proof (headless-safe only)

- S1 — NOT re-launched this trial, stated reason: a FOREIGN live `ultrapos.exe`
  (PID 14996, started 2026-10-31 19:14, parent 5672 = owner session) is running
  on this machine — it was never touched (no kill, no window, no install), and
  launching a second instance alongside the owner's live session would risk
  contending it for zero new information. Binary-executes + read-only-profile
  behavior stands proven by the five identical-architecture S1 runs (REL-011→015:
  own-PID-only launches, empty redirected profiles, live DB byte-identical,
  own PIDs reaped). **Consequence unchanged: no full in-app boot is claimed;
  per-area behavior is proven by the shipped-code paths in S3/S4/S5.**
- S2 — live-instance guard: PID 14996 observed via `wmic`, confirmed foreign,
  left alone; no install attempted on this machine; no port started by this task.
- S3 — presence wire (G3 on this tree): 71/71 suites ok, 0 failed.
- S4 — settings persistence (G3 on this tree): round-trips green; new
  `tspl.probe_mode` key shares the proven `write_key` path.
- S5 — probe switch in the shipped engine (same sources the installer bundles),
  fresh targeted runs pasted:
  `probe_ascii_only_folds_lossy_with_question_marks ... ok`,
  `probe_mode_parses_all_four_spellings_and_rejects_unknown ... ok`,
  `probe_font1_only_unifies_every_text_face ... ok`,
  `probe_no_codepage_drops_only_the_codepage_line ... ok`,
  `probe_normal_is_byte_identical_to_today ... ok`,
  `test result: ok. 24 passed; 0 failed` (domain tspl filter);
  `label_preamble_uses_stored_probe_mode ... ok`,
  `test result: ok. 1 passed; 0 failed` (app stored-mode pin).

## 4. Owner 8-cell script (single install, flip-and-print per mode)

Prerequisites: close any running Baraka UltraPOS, run the §2 installer (SHA-256
must match), launch. **No Factory Default at any point — that is the test.**

Selector location (new in this trial — DSN-100): Settings → printer/hardware
tab → dashed amber **DIAGNOSTIC TEMPORARY / تشخيص مؤقت — Sonde TSPL** block →
4-button segmented control (`Normal` / `Sans CODEPAGE` / `ASCII seul` /
`Police 1 seule`), one per-mode note, amber Arabic warning under `ASCII seul`.
Each flip: tap the mode → Save (same Save button) → print ONE silent 40×20
label + ONE silent 80 mm ticket → record per cell:

| Mode | Label cell | Ticket cell |
|---|---|---|
| `Normal` (baseline) | PRINT / BLANK | PRINT / BLANK |
| `Sans CODEPAGE` (P1) | PRINT / BLANK | PRINT / BLANK |
| `ASCII seul` (P2) | PRINT / BLANK | PRINT / BLANK |
| `Police 1 seule` (P3) | PRINT / BLANK | PRINT / BLANK |

Rules: judge **blank-vs-print ONLY** (complete + readable = PRINT; nothing /
garbage / feed-misalign = BLANK + one-line note). **ASCII note:
`ASCII seul` mangles Arabic BY DESIGN (accents/Arabic → `?`, LOSSY diagnostic)
— a mangled but PRINTED page counts as PRINT; do not judge beauty in P2.**
Report the 8 cells + which single P-mode (if any) turned BLANK→PRINT: that
detail is the rejector (lock-in drops it); all-red = raster-only emulation →
BITMAP pivot (AUD-047 §4); all-green = TEXT accepted → re-examine
transport/state theories. End of trial: flip back to `Normal` + Save.

## 5. Files

- Installer (gitignored, §2) + `audit/REL-016/REPORT.md` +
  `audit/REL-016/tauri-build.log` (gitignored log, on disk).
- Scratch gate/build logs removed (NO_LEFTOVERS). No ad-hoc scripts remain.
  No process left running (builders exited; no own PIDs outstanding).

NEXT READY: YES.
