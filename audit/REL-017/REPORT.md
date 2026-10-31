# TASK-REL-017 REPORT — Seventh trial installer (BITMAP silent print)

## 1. Gate exits + counts (G6 is the build)

- G1 `cargo fmt --check`: EXIT 0 (`G1_FMT_OK`, no diff).
- G2 `cargo clippy --all-targets --all-features -- -D warnings`: EXIT 0
  (`Finished dev profile`, zero warnings).
- G3 `cargo test --workspace -j 2`: EXIT 0 — **71 suites, 587 passed,
  0 failed** (`passed: 587`, `failed: 0`, no FAILED/panicked lines).
- G4 `npm run lint`: EXIT 0 (`LINT_EXIT:0`, max-warnings 0).
- G5 `npm run typecheck` (`tsc --noEmit`): EXIT 0 (`TSC_EXIT:0`).
- G7 `npx vitest run`: **86 files passed, 889 tests passed**
  (`Test Files 86 passed (86)`, `Tests 889 passed (889)`).
- No red at any gate; no fix-forward (frozen tree, build-only).

## 2. Installer path + size + SHA-256

- Path: `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`
  (gitignored, DO NOT COMMIT).
- Size: **223,309,192 bytes**.
- SHA-256: `3f2892b3088a931d3c89d7d6f47b46dc41b0e81c3956ba20b9074383eb1a187d`
  (!= REL-016 `0aa4ae51...`, != REL-015 `9a35bb9b...`).
- G6 `npx tauri build`: EXIT 0, attempt 1 of <=8
  (`Finished 1 bundle at: ...nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`).
- No version bump (stays `1.0.0`), no Delivery packaging/letters,
  no new dependencies.

## 3. Smoke proof (S1 limits honest, byte-identity)

- S1 launch: binary started with `APPDATA`/`LOCALAPPDATA` redirected to
  `_scratch/REL-017/profile-*` (seeded empty profiles) — own child
  PID 14288 observed via wmic
  (`"D:\Dev\Baraka UltraPOS\target\release\ultrapos.exe" 14288`).
- Profiles stayed EMPTY (no db files created under scratch profiles —
  app boots without seeding owner data).
- Kill: `taskkill /PID 14288 /F` -> SUCCESS; re-check:
  `No Instance(s) Available.` No port left listening; no foreign PID
  touched (pre-launch count of `ultrapos.exe` was 0).
- Live DB byte-identity: `baraka.db` 643,072 B, mtime Oct 27 22:15
  unchanged before/after; `-shm` 32,768 B (mtime moves with OS page
  cache — same honest note as REL-016; `-wal` untouched). Live DB never
  opened or modified (read-only `ls` only).
- S5 fresh pins (post-build, unmodified tree):
  `bitmap_job_ignores_the_temporary_probe_switch ... ok`,
  `bitmap_job_is_byte_exact_on_a_two_byte_fixture ... ok`,
  `bitmap_job_carries_label_and_ticket_geometry ... ok`,
  `bitmap_job_refuses_bad_geometry_copies_and_blank_frames ... ok`
  (4 passed); `raw_bitmap_spools_validated_bytes_and_rejects_before_send
  ... ok` (1 passed).

## 4. Owner verdict script (BITMAP acceptance)

Rule: for EACH format, silent-print and record PRINT (paper moved, image
visible) vs BLANK (paper moved or not, nothing visible). Judge
print-vs-blank ONLY — not beauty.

1. Settings: confirm printer section TEMPORARY probe block reads
   **Normal** (bitmap path ignores it, but keep the known state).
2. Silent label: open a product with barcode, print the 40x20 label.
   EXPECTED: name + price + bars + one code line (see BRK-101
   `raster-label.png` reference). Record PRINT / BLANK.
3. Silent ticket: sell 2 lines + discount, print the 80 mm ticket.
   EXPECTED: header + lines + totals + barcode (see BRK-101
   `raster-ticket.png` reference). Record PRINT / BLANK.
4. Photograph: for each PRINT cell, one photo of the paper (whole
   label / whole ticket, flash off, straight-on); for each BLANK cell,
   one photo of the printer (paper state + LED) + note whether paper
   moved. Four photos max, named `bitmap-label-print|blank.jpg`,
   `bitmap-ticket-print|blank.jpg`.
5. Fallback-to-Normal note: if the owner previously flipped the probe
   switch during P1–P3, set it back to **Normal** and re-run cells 2–3
   once — bitmap bytes are probe-independent (pinned identical), so any
   difference between runs is printer state, not the switch.
6. Send the 4 cells (PRINT/BLANK x label/ticket) + photos. Verdict
   GREEN (any PRINT) -> lock-in (TEXT removal). All-BLANK -> emulation
   AUD (no more TSPL).

## 5. Leftovers / hygiene

- Deleted post-smoke: `_scratch/REL-017/` (profiles),
  `rel017-g3.log`, `rel017-smoke.log`, `rel017-tauri.log`.
  `_scratch/` back to pre-existing `AUD-005` + `TASK-DSN-012` only.
- `hermes verify` never run (standing ban — running it is a task
  failure). No console.log, no new deps, no version change.

## 6. Tree

- `git status --short` at seal: only `?? audit/REL-017/` (+ gitignored
  installer and gitignored `*.log` build log inside it).
- Base: `da3f03a` unchanged. DO NOT COMMIT (REPORT staged by
  architect).

STATE: DONE / REMAINING: owner BITMAP verdict (4 cells + photos) ->
lock-in or emulation AUD. NEXT READY: YES.
