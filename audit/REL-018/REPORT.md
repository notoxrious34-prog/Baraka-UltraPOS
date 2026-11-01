# REL-018 — Eighth trial installer (vendor-mirrored preamble)

## 1. Gates (all green, exact tree `53bc0d0`)

- G1 `cargo fmt --check` — EXIT 0.
- G2 `cargo clippy --all-targets --all-features -- -D warnings` — EXIT 0.
- G3 `cargo test --workspace -j 2` — EXIT 0, 71 suites, 587 passed, 0 failed.
- G4 `npm run lint` — EXIT 0. G5 `npm run typecheck` — EXIT 0.
- G7 `npx vitest run` — EXIT 0, 86 files, 889 passed.
- No red, no fix-forward. Frontend untouched (zero `.ts` changes).

## 2. Installer

- `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`
- 223,307,551 bytes — SHA-256 `173594db…c8635b` (≠ REL-017's `3f2892b3…`).
- G6 attempt 1 of ≤8, EXIT 0 (makensis stage reached and completed).

## 3. Smoke (honest S1 limits)

- Silent install to scratch: EXIT 0 (`ultrapos.exe` + `uninstall.exe` present).
- Own child PID 16928 launched with redirected APPDATA/LOCALAPPDATA (scratch
  profiles, stayed empty of live data) → `taskkill /F` SUCCESS → verified gone
  (`No tasks ... match`). No full boot claimed.
- Live DB byte-identical by metadata (read-only listing, never opened):
  db 774144 + wal 4132392 unchanged before/after; only `-shm` mtime moved —
  owner's own live session is active (foreign), so no attribution claimed.
- S5 fresh pins: domain `printing::` 36/36, app `tspl_print_tests` 10/10.

## 4. Owner verdict script (trial 8 — vendor preamble)

1. Install, open Settings → printer queue unchanged (`Xprinter XP-410B`).
2. Silent label (40×20) → PRINT or BLANK? Photograph the result.
3. Silent ticket (80 mm) → PRINT or BLANK? Photograph the result.
4. Rule: judge PRINT-vs-BLANK only — `é` may render as two glyphs under the
   printer default codepage (cosmetic, next step if print works).
5. 4 photos rule: `bitmap-<format>-<print|blank>.jpg` + note any LED/motion.
6. Fallback: flip probe mode back to Normal when done (selector still present).

## 5. Tree

- `git status --short`: only `?? audit/REL-018/` (+ gitignored installer).
- Scratch cleaned: `t8-smoke/`, `t8-profile/`, `g4-t8.log`, `g6-t8.log` removed.
- STATE: DONE / REMAINING: owner verdict → lock-in (TEXT removal + probe
  switch removal) or deeper investigation. NEXT READY: YES.
