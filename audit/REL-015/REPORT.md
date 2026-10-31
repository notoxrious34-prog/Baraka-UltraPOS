# TASK-REL-015 REPORT — Fifth trial installer with the TSPL probe switch

- Base: tree clean at `051d2d9` (BRK-100 sealed: `tspl_probe_mode`
  normal/no_codepage/ascii_only/font1_only, 582/0 by record, TEMPORARY-DIAGNOSTIC).
- Trial, no delivery: no Delivery zip, no letters, no version bump (stays `1.0.0`),
  no sealing commits. WebView2 via seeded cache (cache hit — no download).
- Status: DONE. No commit (installer gitignored via `target/`; REPORT staged by
  architect). No code changes (build-only; frozen tree).

## 1. Gates (full pre-build; G6 is the build itself)

| Gate | Command | Result |
|---|---|---|
| G1 | `cargo fmt --check` | CLEAN, exit 0 |
| G2 | `CARGO_BUILD_JOBS=2 cargo clippy --all-targets --all-features -- -D warnings` | GREEN, zero warnings, exit 0 |
| G3 | `CARGO_BUILD_JOBS=2 cargo test --workspace -j 2` | GREEN — **582 passed, 0 failed** (71 `test result: ok` suites, zero nonzero-`failed` lines), EXIT 0 |
| G4 | `npm run lint` (eslint, max-warnings 0) | GREEN, exit 0 |
| G5 | `npm run typecheck` (`tsc --noEmit`) | GREEN, no errors |
| G7 | `npx vitest run` | GREEN — **84 files, 874 tests passed** |

No red at any gate; no fix-forward (tree frozen).

## 2. Bundle (attempt 1 of ≤8 — first success, loop stopped)

- Release compile clean; bundler cache hit (no WebView2 `Downloading`); makensis
  ran → `Finished 1 bundle`, `TAURI-EXIT:0` (`audit/REL-015/tauri-build.log`, gitignored).
- Path: `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`
- Size: **223,305,302 bytes** (2026-10-31 18:33; fresh — ≠ REL-014's 223,298,532)
- SHA-256: `9a35bb9b9ea94362bed66fb60cc608ed81c13853e84c904649221f1e040bf2ed` (≠ REL-014's `050a42b5…`)
- Ignored by: `.gitignore` (`target/`).

## 3. Smoke proof (headless-safe only)

- S1 — shipped binary executes, with the disclosed isolation finding (same as
  REL-011→014): `target/release/ultrapos.exe` launched as my own PID 5296
  (parent 13988, my shell tree) with `APPDATA`/`LOCALAPPDATA` redirected to
  scratch profiles. Test profiles stayed EMPTY (zero files) while the process
  opened the live `%APPDATA%/com.baraka.ultrapos/baraka.db`. Evidence read-only:
  `baraka.db` 643,072 bytes / Oct 27 22:15 and `baraka.db-wal` 4,132,392 bytes /
  Oct 31 17:38 byte-identical before and after (sizes + mtimes); only
  `baraka.db-shm` mtime moved (18:16 → 18:34, OS open artifact, size unchanged
  32768). Killed ONLY my own PID (`taskkill /PID 5296 /F` → `SUCCESS`, verified
  `No Instance(s) Available` after). **Consequence: no full in-app boot is
  claimed; per-area behavior is proven by the shipped-code paths in S3/S4/S5.**
- S2 — profile isolation / live-instance guard: no live `ultrapos.exe` before,
  during (except own 5296), or after; foreign PIDs never touched; no install
  attempted on this machine.
- S3 — presence wire (G3 on this tree): 71/71 suites ok incl. the four presence
  round-trips; 0 failed.
- S4 — settings persistence (G3 on this tree): existing settings round-trips
  green; new `tspl.probe_mode` key shares the proven `write_key` path.
- S5 — probe switch in the shipped engine (same sources the installer bundles),
  fresh targeted runs pasted:
  `probe_ascii_only_folds_lossy_with_question_marks ... ok`,
  `probe_font1_only_unifies_every_text_face ... ok`,
  `probe_mode_parses_all_four_spellings_and_rejects_unknown ... ok`,
  `probe_no_codepage_drops_only_the_codepage_line ... ok`,
  `probe_normal_is_byte_identical_to_today ... ok`,
  `test result: ok. 24 passed; 0 failed` (domain tspl filter);
  `label_preamble_uses_stored_probe_mode ... ok`,
  `test result: ok. 1 passed; 0 failed` (app stored-mode pin).
- Ports: nothing started by this task listens (tauri build serves nothing).

## 4. Owner P1–P3 script — 8 cells (4 modes × label + ticket)

Prerequisites: close any running Baraka UltraPOS, run the §2 installer (SHA-256
must match), launch. **No Factory Default at any point — that is the test.**

**Flip step (read carefully — no selector UI in this build):** the validated
`printer_update_settings` IPC carries `tspl_probe_mode` (BRK-100 wire), but DSN
has NOT built the selector UI yet (BRK-100 SCOPE-OUT: DSN owns display). So the
owner CANNOT flip modes from the screen in this trial — each flip is one IPC
call with `{"tspl_probe_mode": "<mode>"}` executed on the owner machine
(architect provides the one-line Tauri invocation before the trial, or a DSN
selector follow-up). Default after install is `normal`. Invalid spellings are
rejected loudly, never applied silently.

Per mode (`normal` → `no_codepage` → `ascii_only` → `font1_only` → back to
`normal`), print ONE silent 40×20 label + ONE silent 80 mm ticket and record:

| Mode | Label cell | Ticket cell |
|---|---|---|
| `normal` (baseline) | PRINT / BLANK | PRINT / BLANK |
| `no_codepage` (P1) | PRINT / BLANK | PRINT / BLANK |
| `ascii_only` (P2) | PRINT / BLANK | PRINT / BLANK |
| `font1_only` (P3) | PRINT / BLANK | PRINT / BLANK |

Rules: judge **blank-vs-print ONLY** (complete + readable = PRINT; nothing /
garbage / feed-misalign = BLANK + one-line note). **Arabic note: `ascii_only`
mangles Arabic BY DESIGN (accents/Arabic → `?`, LOSSY diagnostic) — a mangled
but PRINTED page counts as PRINT; do not judge beauty in P2.** Report the 8
cells + which single P-mode (if any) turned BLANK→PRINT: that detail is the
rejector (lock-in drops it); all-red = raster-only emulation → BITMAP pivot
(AUD-047 §4); all-green = TEXT accepted → re-examine transport/state theories.

## 5. Files

- Installer (gitignored, §2) + `audit/REL-015/REPORT.md` +
  `audit/REL-015/tauri-build.log` (gitignored log, on disk).
- Scratch `D:/Dev/_scratch/REL-015/` + gate/build logs removed (NO_LEFTOVERS).
  No ad-hoc scripts remain. No process left running (own PID 5296 reaped;
  builders exited).

NEXT READY: YES (modulo the flip-invocation/DSN-selector dependency in §4).
