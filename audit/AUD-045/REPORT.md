# TASK-AUD-045 REPORT — Factory-Default-gated printing (read-only audit)

## 1. Preamble quotes (emitted bytes, `crates/domain/src/printing/tspl.rs`)

Label 40×20 (`render_label`, lines 339-341; byte-pinned test line 691-693):

```
SIZE 40 mm,20 mm\r\nDIRECTION 1\r\nCLS\r\n
```

Ticket 80mm (`render_ticket`, lines 646-648; byte-pinned test line 844):

```
SIZE 80 mm,{H} mm\r\nDIRECTION 1\r\nCLS\r\n   (H = computed page height, e.g. 84/74)
```

Trailer: `PRINT 1,{copies}` (label, line 377) / `PRINT 1,1` (ticket, line 650).
Body between CLS and PRINT: `TEXT` / `BARCODE "128"` / rules only.

Absent commands (grep-verified): `GAP` occurs exactly 2× in the file — both non-emitted
(doc comment line 47 + test assert `!text.contains("GAP")`). Zero hits for `DENSITY`,
`SPEED`, `CODEPAGE`, `BLINE`, `SET CUTTER`, `OFFSET`, `SHIFT`, or any media-type select.
Deliberate per module docs + ADR-105: "the printer's calibrated media gap applies
(DSN-098 owns media setup)"; "no cut bytes emitted by design".

## 2. Missing-command table (self-describing gap-label job)

No TSPL reference is vendored in the repo (HANDOFF applies: standard stated from the
task, UNVERIFIED against a TSC/XPrinter programming manual on this machine):

| Command | Role | Our job | Required value source |
|---|---|---|---|
| `SIZE 40 mm,20 mm` | label pitch declaration | SENT | fixed geometry |
| `GAP m,n` (mm) | inter-label gap + offset | MISSING | measured from 40×20 stock (owner fact to collect, §5) |
| `DIRECTION 1` | forward feed | SENT | fixed |
| `DENSITY n` | heat (ribbon/DT stock) | MISSING | owner stock type |
| `SPEED n` | print speed | MISSING | conservative default |
| `CODEPAGE` | byte→glyph table for TEXT | MISSING | needed for Arabic path (§4) |
| `CLS` | image buffer clear | SENT | fixed |
| media sensor select (Gap vs Black-mark vs Continuous) | which sensor threshold applies | MISSING | NVRAM today; Gap for die-cut 40×20 |

Ticket (continuous 80mm) is legitimately gapless — `SIZE`-only preamble is correct there;
its failure mode on blank NVRAM is the same family (unit/density/speed unknown).

## 3. Mechanism (labeled)

PROVEN (code + delivery evidence): our bytes reach the printer intact — modal confirms
"1 étiquette(s) envoyée(s) — 217 o" — and carry no media-describing state beyond
SIZE/DIRECTION. Printer NVRAM is blank (Standby, ALL config fields blank, Get never
pressed, Unit=inch, sensor never calibrated). A TSPL printer positions each label using
NVRAM media state (gap pitch/threshold, sensor select); with none stored, the job
misfeeds/prints blank despite correct bytes.

INFERENCE (marked): Diagnostic Tool V1.039b "Factory Default" rewrites NVRAM to factory
values — including a default gap/threshold and sensor select that happens to match
(or tolerate) the loaded 40×20 stock — which is why exactly one manual reset unblocks
subsequent jobs until NVRAM drifts/is cleared again. The precise reset field list of
V1.039b was not probed (no hardware access in this audit).

## 4. Arabic verdict + fallback shape

VERDICT (stated in module docs lines 50-54, consistent with built-in-font TSPL):
`TEXT` payloads pass through as verbatim UTF-8 bytes; TSC-class built-in fonts ("1"/"2"/"3")
do NOT shape Arabic and have no BiDi — Arabic names/captions will print as unshaped,
logical-order glyphs (disconnected letters, LTR order). Bytes are never destroyed
(no transliteration), so nothing is lost — only rendered wrong.

BITMAP fallback shape (no implementation): replace `TEXT` for Arabic runs with the
`BITMAP x,y,width,height,mode,<raster>` command carrying pre-rasterized, shaped+ordered
glyphs (shaping done host-side); keep ASCII runs on built-in fonts. Decision point for
trial 4: one Arabic label print decides built-in vs BITMAP path.

## 5. Owner procedure + gap fact to collect

In Diagnostic Tool V1.039b, printer USB001 XprinterXP-410B, WITH 40×20 stock loaded:
1. `Get` — read current NVRAM (record: media type, density, speed, sensor values).
2. Set media type to Gap/Label with gap; `Calibrate Sensor` (auto gap measurement).
3. `Save`/`Write` to NVRAM; print self-test page; confirm one label feeds exactly one pitch.
4. Collect and report back the measured gap in mm (distance between two labels on the
   40×20 liner — commonly 2–3 mm; OWNER-PROVIDED FACT, do not assume) + density/speed
   values that print dark without ribbon bleed.

## 6. Fix spec (commands + order + values; NO code)

Label preamble becomes self-describing, in this order:
`SIZE 40 mm,20 mm` → `GAP {gap},{offset}` (gap = owner-measured mm; rule:
default 3 mm ONLY until the measured value is stored — measured value wins once
collected, never silently) → `DIRECTION 1` → `DENSITY {n}` → `SPEED {n}` →
(`CODEPAGE` per §4 decision) → `CLS` → body → `PRINT 1,n`.
Ticket preamble unchanged in shape (`SIZE 80 mm,H` + `DIRECTION 1` + `CLS`),
plus `DENSITY`/`SPEED` once owner values are known. No `CUT` (by design, ADR-105).

## 7. Evidence status

- `git status --short`: only `?? audit/AUD-045/` (this REPORT + nothing else). NOTHING modified.
- STATE: DONE. REMAINING: owner calibration (§5) → robustness fix → trial 4. NEXT READY: YES.
