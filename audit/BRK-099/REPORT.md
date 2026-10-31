# TASK-BRK-099 REPORT — Self-describing TSPL preamble (GAP + DENSITY + SPEED + CODEPAGE)

## 1. Preamble table + order pin

Every silent job now emits, byte-for-byte pinned (`preamble_order_is_pinned_byte_for_byte`):

```
SIZE 40 mm,20 mm | SIZE 80 mm,{H} mm
GAP {gap},0                      (label: stored 0..=10; ticket: forced 0 = declared continuous)
DENSITY {n}                      (stored 0..=15)
SPEED {s}                        (stored 1..=6 in/s)
CODEPAGE UTF-8                   (fixed; see §2)
DIRECTION 1
CLS
```

Order: SIZE → GAP → DENSITY → SPEED → CODEPAGE → DIRECTION → CLS.
Label exact bytes updated (`SIZE…GAP 3,0/DENSITY 8/SPEED 4/CODEPAGE UTF-8/…`);
ticket starts `SIZE 80 mm,84 mm\r\nGAP 0,0\r\nDENSITY 8\r\nSPEED 4\r\nCODEPAGE UTF-8\r\nDIRECTION 1\r\nCLS\r\n`
(`ticket_bytes_pin_size_media_preamble_and_copies`). No reference contradicted
the order (reference UNVERIFIED per AUD-045; order follows the task-stated standard).

## 2. Setting bounds + defaults + CODEPAGE choice

`TsplMediaSetup { gap_mm: u32, density: u8, speed: u8 }`, `Default` = 3/8/4.
Validators (codes quoted, loud reject — never clamps):
- `validate_tspl_gap_mm`: 0..=10 → `TSPL_GAP_MM_OUT_OF_RANGE` (11 rejected, pinned).
- `validate_tspl_density`: 0..=15 → `TSPL_DENSITY_OUT_OF_RANGE` (16 rejected, pinned).
- `validate_tspl_speed`: 1..=6 → `TSPL_SPEED_OUT_OF_RANGE` (0 and 7 rejected, pinned).
- Renderer re-validates every job (`push_media_preamble`); stored-value reads
  fall back to defaults on out-of-range (never trusted blindly).

Persisted in the key-value `settings` table — zero schema changes, zero migrations:
`tspl.label_gap_mm` / `tspl.print_density` / `tspl.print_speed`. Surface: `PrinterSettings`
+3 fields, port `update_tspl_media_settings` (existing `update_printer_settings`
untouched — no caller churn), `UpdateTsplMediaSettingsUseCase` (ADMIN-gated),
DTO `PrinterSettingsDto` +3 fields, `UpdatePrinterSettingsRequest` +3 `Option`
(serde-defaulted: old frontend payloads still deserialize; zero `.ts` changes).

CODEPAGE fixed `UTF-8` — NOT 437: our TEXT bytes ARE UTF-8 (French accents are
multibyte; CP437 would render them as mojibake); UTF-8 keeps French correct and
Arabic codepoint-accurate (unshaped per AUD-045; trial 4 decides BITMAP).
Printer acceptance of the `CODEPAGE UTF-8` line is OWNER-VERIFY — fallback is
dropping the line if the XP-410B rejects it.

OWNER-VERIFY list: gap default 3 mm is a STARTING VALUE (ruler against the 40×20
liner; measured value wins — pinned by `media_defaults_are_owner_verify_starting_values`);
sensor calibration (Diagnostic Tool procedure, AUD-045 §5); density/speed trial
(8/4 defaults, adjust for ribbon/bleed); CODEPAGE acceptance on hardware.

## 3. Test lines

- `preamble_order_is_pinned_byte_for_byte` — first 7 lines equal the table above.
- `media_defaults_are_owner_verify_starting_values` — default() == 3/8/4 + validates.
- `media_bounds_reject_loudly` — gap 11 / density 16 / speed 0,7 reject with quoted
  codes; renderer rejects gap-11 bytes; edges (0/15/6, 10/0/1) validate.
- `label_bytes_are_exact` — full label bytes incl. new preamble.
- `ticket_bytes_pin_size_media_preamble_and_copies` — GAP 0,0 no-gap form + no CUT.
- `label_preamble_uses_stored_media_setup` (app) — stored (7,12,2) rides spooled bytes.

## 4. Gates

- G1 `cargo fmt --check` EXIT 0 (one clippy-driven fmt fix applied to a test assert).
- G2 `cargo clippy --all-targets --all-features -- -D warnings` EXIT 0 (one
  `manual_range_contains` fix in `validate_tspl_speed`).
- G3 `cargo test --workspace -j 2` EXIT 0 — 71 suites, 576 passed (572 + 4 new), 0 failed.
- G4–G7 N/A — backend-only: zero `.ts`/`.tsx`/`.css` changes; IPC wire is
  backward-compatible (new response fields ignored by old frontend, new request
  fields `Option` + serde-defaulted).

## 5. Status

- `git status --short`: `M` ports.rs, printing.rs (app use-case), tspl_printing.rs,
  tspl_print_tests.rs, dto.rs, ipc.rs, printing/mod.rs, tspl.rs,
  sqlite_printer_settings.rs + `?? audit/BRK-099/`. No strays (scratch log removed).
  One line-ending incident on dto.rs repaired (reconstructed from HEAD: 15+/0-).
  DO NOT COMMIT honored.
- STATE: DONE. REMAINING: owner ruler-gap + calibration + density/speed trial (trial 4).
  NEXT READY: YES.
