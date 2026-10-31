# TASK-DSN-100 REPORT — Temporary probe-mode selector UI (unlocks owner P1–P3)

## 1. FAIL-before

No selector existed: `grep -rni "probe|tspl"` over `src/` matched only
`print_tspl_*` silent paths + unrelated words (`probes` in App.tsx comments,
`tsplState` in CheckoutModal) — zero UI surface for `tspl_probe_mode`.
REL-015 trial 5 proved it: only `normal` was ever exercised.

## 2. Component lines

- `src/lib/ipc.ts`: `TsplProbeMode` union
  (`'normal' | 'no_codepage' | 'ascii_only' | 'font1_only'` — invalid
  spellings unconstructible, never sent); `toTsplProbeMode` read-fallback to
  `normal`; `PrinterSettings.tsplProbeMode`; `ipcGetPrinterSettings` parses
  `tspl_probe_mode`; `ipcUpdatePrinterSettings(..., renderMode?, probeMode?)`
  sends `tspl_probe_mode: probeMode ?? null` (None-keeps server-side).
- `src/features/settings/PrinterSettingsTab.tsx`: `probeMode` state, load/save
  wiring through the single Save, TEMPORARY block after the render-mode help —
  dashed amber border + `settings.tsplProbeBadge` pill + 2×2 segmented group
  (`role="group"`, `aria-pressed`) + per-mode note + ascii_only amber warning.
- `src/lib/i18n.ts`: 11 keys × FR + AR (title, badge, 4 labels, 4 notes, ascii
  warning naming Arabic explicitly: « l’arabe » / « العربية »).
- Display only: writes go through the validated setting (BRK-100); no engine
  changes; DSN owns this block, lock-in task removes it.

## 3. Pin lines (`src/features/settings/tspl-probe-selector.test.ts`, 8/8)

Badged dashed group + 4 loop-rendered buttons; 4 labels + 4 notes keyed in tab
and i18n; ascii warning gated on `ascii_only` with FR `l’arabe` + AR `العربية`;
facade read-parse + union-only send + tab save/load round-trip; no pictographs;
every key present in both locales. One self-caught fix: buttons render from
`PROBE_LABEL_KEYS` (loop var), pin asserts the map + `aria-pressed={probeMode
=== mode}` instead of per-mode literals.

## 4. Captures + asserts (DSN-099 chunked-injection method, IIFE harness)

- `audit/DSN-100/selector-fr-light.png` (82,961 B): dashed block +
  `DIAGNOSTIC TEMPORAIRE` + title; 4 buttons (`Normal`, `Sans CODEPAGE`,
  `ASCII seul` pressed, `Police 1 seule`); P2 note; beige warning naming
  l’arabe; no overlap/clipping.
- `audit/DSN-100/selector-ar.png` (54,466 B): RTL block + `تشخيص مؤقت` +
  `مسبار TSPL (تشخيص)`; 4 buttons RTL-ordered (`عادي`, `بدون CODEPAGE`,
  `ASCII فقط` pressed, `الخط 1 فقط`); warning naming العربية; dropdown arrow
  mirrored; bidi strings correct; no RTL breakage/overlap.
- Browser asserts: fr+ar sections present, both badges, 4 buttons per group,
  exactly 1 pressed (mock-stored `ascii_only`), both warnings, dashed block,
  **0 console errors**.

## 5. Gates (G1–G3 N/A — zero Rust: no `.rs` touched, engine frozen)

- G4 `npm run lint` EXIT 0 (max-warnings 0).
- G5 `npm run typecheck` EXIT 0 (11 new keys typecheck in both dicts).
- G6 `npm run build` EXIT 0 (vite 5.13 s).
- G7 `npx vitest run` EXIT 0 — **85 files, 882 passed (874 + 8 new), 0 failed**.

## 6. Status

- `git status --short`: `M` × 3 (PrinterSettingsTab, i18n, ipc) +
  `?? src/features/settings/tspl-probe-selector.test.ts` +
  `?? audit/DSN-100/`. Scratch harness/gen/config + `_scratch/DSN-100/` +
  build log removed (NO_LEFTOVERS); no other strays. DO NOT COMMIT honored.
- STATE: DONE. REMAINING: trial 6 build → owner P1–P3 8 cells → lock-in or
  BITMAP pivot. NEXT READY: YES.
