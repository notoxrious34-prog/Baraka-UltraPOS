# TASK-BRK-100 REPORT — TSPL diagnostic probe switch (TEMPORARY, owner-flippable)

- TEMPORARY-DIAGNOSTIC: the `tspl_probe_mode` setting + all four render paths are
  marked for REMOVAL in the lock-in task (code comments + this REPORT). The
  winner (one mode, or BITMAP on all-red) replaces the switch — the switch
  itself never ships as a user feature. DSN owns any mode-selector UI; this
  task exposes the wire only (no `.ts`/`.tsx` changes).
- ascii_only is LOSSY by design (accents/Arabic → `?`); REPORT + code state it.

## 1. Mode table + byte diffs (vs Normal = today's bytes)

| Mode (wire spelling) | P | Byte diff vs Normal | Discriminates |
|---|---|---|---|
| `normal` (default) | — | none (regression-pinned identical) | baseline |
| `no_codepage` | P1 | preamble loses `CODEPAGE UTF-8` (6 lines: SIZE/GAP/DENSITY/SPEED/DIRECTION/CLS) | CODEPAGE-syntax rejector |
| `ascii_only` | P2 | every TEXT payload folded non-ASCII→`?` (`café Üمر`→`caf? ???`); pure-ASCII output | multibyte-bytes rejector |
| `font1_only` | P3 | every `TEXT` face forced `"1"` (layout widths unchanged — may shift, probe asks print-vs-ignore) | font-file rejector |

Unknown spellings → `TSPL_PROBE_MODE_UNKNOWN` (parse + write path LOUD; read path
defaults `normal` for absent keys, same posture as numeric media keys — only a
hand-edited DB can hold garbage, and `media_from_settings` fails such a print
loudly instead of probing silently).

Seam (no new commands): validated `tspl_probe_mode` rides the EXISTING settings
surface — `PrinterSettings.tspl_probe_mode`, port `update_tspl_media_settings`
+`Option<String>`, ADMIN-gated `UpdateTsplMediaSettingsUseCase` (extended, not
new), DTO `PrinterSettingsDto` +`String` / `UpdatePrinterSettingsRequest`
+`Option<String>` (`#[serde(default)]` — old frontend payloads still
deserialize), key `tspl.probe_mode` in the existing `settings` table (zero
migrations). Renderer stays pure (`TsplProbeMode` param in, bytes out).

## 2. Test lines

- `probe_mode_parses_all_four_spellings_and_rejects_unknown` — 4 spellings +
  `bogus`/`""` → `TSPL_PROBE_MODE_UNKNOWN`.
- `probe_no_codepage_drops_only_the_codepage_line` — 6-line preamble, no CODEPAGE.
- `probe_ascii_only_folds_lossy_with_question_marks` — fold unit + spooled-label
  ASCII assertion (one self-caught expectation miscount fixed: `Ü`+`مر` = 3 `?`).
- `probe_font1_only_unifies_every_text_face` — all TEXT lines `,"1",0,`.
- `probe_normal_is_byte_identical_to_today` — explicit Normal == default bytes
  (plus unchanged `label_bytes_are_exact`).
- `label_preamble_uses_stored_probe_mode` (app) — stored NoCodepage rides
  spooled bytes (`…SPEED 4\r\nDIRECTION 1\r\nCLS`).

## 3. Gates

- G1 `cargo fmt --check` EXIT 0 (after `cargo fmt --all`).
- G2 `cargo clippy --all-targets --all-features -- -D warnings` EXIT 0.
- G3 `cargo test --workspace -j 2` EXIT 0 — **71 suites, 582 passed (576+6), 0 failed**.
- G5 `npm run typecheck` EXIT 0 (shared wire touched — DTO gained fields; zero
  `.ts` changes, backward-compatible). G4/G6/G7 N/A backend-only (G6 = next
  trial build; state reason).

## 4. Owner P1–P3 script (single trial build, no reinstalls between flips)

Admin flips the mode (settings surface), then per mode prints ONE silent label +
ONE silent ticket, reporting per mode `PRINTED` (complete, readable) or `BLANK`
(nothing / garbage) — with NO Factory Default between modes:
P1 `no_codepage` → P2 `ascii_only` → P3 `font1_only` → back to `normal`.
Report the 4×2 table. Read: exactly one P-green = that detail is the rejector
(lock-in drops it); all-red = SURVIVOR B confirmed → BITMAP pivot (AUD-047 §4);
all-green = TEXT accepted → re-examine transport/state theories.

## 5. Status

- `git status --short`: `M` × 9 (ports, printing use-case, tspl_printing,
  tspl_print_tests, dto 5+/0-, ipc, printing/mod, tspl, sqlite_printer_settings)
  + `?? audit/BRK-100/`. No strays (scratch log removed). DO NOT COMMIT honored.
- STATE: DONE. REMAINING: owner P1–P3 (single trial) → lock-in or BITMAP pivot.
  NEXT READY: YES.
