# TASK-AUD-047 REPORT — TSPL TEXT-mode rejection vs BITMAP pivot (read-only)

## 1. Command table (emitted bytes quoted verbatim, compatibility assessed)

Emitters (`crates/domain/src/printing/tspl.rs`):

| Emitted line | Verbatim form | TSPL2 reality | Status |
|---|---|---|---|
| TEXT | `TEXT {x},{y},"1"\|"2"\|"3",0,{s},{s},"{…}"` (push_text:403-407) | Built-in fonts "1"–"5" + rotation/x/y-scale params are documented TSPL2 core | KNOWN syntax; **font-file existence on XP-410B-class firmware UNVERIFIED** |
| BARCODE | `BARCODE {x},{y},"128",{40\|64},1,0,2,2,"{…}"` (479, 714) | Code128 + narrow/wide/height params are documented TSPL2 core | KNOWN syntax; **on-device 128 rendering UNVERIFIED** (never printed) |
| CODEPAGE | `CODEPAGE UTF-8` (const:102, emitted:179) | TSC documents CODEPAGE with **numeric** ids; name-form `UTF-8` acceptance is firmware-dependent | **UNVERIFIED — most suspicious line (BRK-099-added, never trial-printed)** |
| Preamble | SIZE/GAP/DENSITY/SPEED/DIRECTION/CLS | All documented core setup commands | KNOWN; `GAP 0,0` acceptance UNVERIFIED |
| Payload bytes | UTF-8 (French accents multibyte + Arabic) | TSPL2 TEXT expects single-byte codepage bytes unless UTF-8 mode active | **Parser behavior on multibyte UTF-8 UNVERIFIED** |
| BITMAP (not emitted) | `BITMAP x,y,width,height,mode,data` | Documented TSPL2 core, mode 0 = overwrite | KNOWN (pivot relies on this) |

No firmware facts invented: every XP-410B-specific behavior above is UNVERIFIED + owner-probeable (Tool font-list read; self-test/config page listing emulation + codepage + sensor).

## 2. Matrix-kill table (E1–E4 decisive)

| Theory | Verdict | Killer |
|---|---|---|
| USB/pipe/cable dead; printer hardware dead | DEAD | E1 (driver test-page prints) |
| Sleep/wake (printer asleep for our jobs) | DEAD | E2 (silent-after-driver NO — just proven awake, still ignores us) |
| USB suspend | DEAD | E4 (suspend-disabled NO) |
| Blank-NVRAM media state as SOLE cause | DEAD as sole cause | E3 (same state: driver raster YES, our TSPL NO) |
| Spooler parks our job (never completes) | DEAD as sole cause | AUD-046 (handshake completes; jobs vanish) + E3 (vanish + zero LED) |
| **SURVIVOR A: TSPL TEXT-stream rejection** (font missing / CODEPAGE-line parse abort / UTF-8 bytes choke) | ALIVE | Consistent with E1–E4: driver raster bypasses the TEXT parser entirely |
| **SURVIVOR B: emulation expects raster on this path** (V3 GDI→raster; RAW TEXT unsupported in this firmware build) | ALIVE | Same consistency; Seagull path never emits TEXT |

LANGUAGE verdict stands: driver raster accepted, our TSPL TEXT stream ignored.

## 3. Get reinterpretation (INFERENCE-labeled)

The Tool opens USB directly (outside the spooler) and speaks bidirectionally. INFERENCE candidates for what unblocks printing where driver jobs don't need it: (a) soft parser reset clearing a latched error from a rejected TEXT job; (b) emulation/mode select issued by the Tool session; (c) status read clearing a held condition. Driver jobs don't need the ritual because the Seagull monitor protocol handshakes status itself, while our RAW fire-and-forget never asks. All three unprobed — no Tool protocol source available.

## 4. BITMAP-route design

Pipeline (built-in APIs only, zero new dependencies): existing graphique print HTML
(ticket: DSN-099 mirror, block-parity with TSPL anatomy; label: 320×160 preview mockup)
→ offscreen render at 203 dpi scale (label 320×160 px, ticket 640×H px) via
SVG-`foreignObject` + `canvas.drawImage` (platform built-ins: SVGSerializer, Image,
Canvas2D, ImageData) → threshold → 1-bit pack → TSPL wrapper
(`SIZE…/GAP…/DENSITY…/SPEED…/CLS` + `BITMAP x,y,wBytes,h,0,data` + `PRINT`) → send.

Seam: `PrinterTransportPort::send_raster(bytes)` is UNTOUCHED (bytes in, RAW out —
the BITMAP stream is just bytes). Minimal thin addition: one IPC command
(e.g. `print_raw_bitmap`) accepting pre-rasterized bytes + copies → existing transport;
TSPL wrapper built by a tiny pure helper (string concat, no dep). Frontend owns
rasterization (only it has canvas); engine keeps preamble/validation discipline.

Payload math (203 dpi = 8 dots/mm): label 320×160 → 40 B/row × 160 = **6,400 B**;
ticket 640×672 (84 mm) → 80 B/row × 672 = **53,760 B (~52.5 KiB)**. USB-trivial
(milliseconds of bulk transfer; spooler routinely handles MB jobs).

Risk classes that disappear (with why): Arabic shaping → browser text engine shapes
before rasterization (no printer font involved); Code128 → existing SVG generator
pixels (no on-device 128 needed); quiet zones → CSS padding becomes white pixels;
CODEPAGE/font-file/parser doubts → ZERO TEXT commands emitted; emulation doubt →
E1 PROVES this printer prints raster. Remaining risks: preamble lines must still parse
(SIZE/GAP — if the emulation rejects even those, fall back to driver-path printing);
threshold quality; ticket height computation.

## 5. Recommendation (TEXT probe batch first, BITMAP pivot ready)

The audit proves NO single byte error — all three TEXT suspects are UNVERIFIED, so a
blind TEXT "fix" is guessing. Recommend a discriminating probe batch (≤3 owner trials):
P1 drop the `CODEPAGE UTF-8` line only (most suspicious, newest); P2 ASCII-only content
(transliterated — isolates UTF-8 bytes vs font files); P3 `TEXT` with font "1" only.
Each outcome names the cause (P1-green = CODEPAGE syntax; P2-green = UTF-8 bytes;
P3-green = font files; all-red = SURVIVOR B confirmed → pivot).

| Route | Effort | Risk | Pays off when |
|---|---|---|---|
| TEXT probe batch (P1–P3) | Tiny (byte-line toggles + existing pins) | Low (reversible; trial-cheap) | Any single TEXT detail is the rejector |
| BITMAP pivot | Medium (canvas rasterizer + thin command + pins; no new deps) | Low–medium (inherits proven graphique layout + E1-proven raster acceptance) | SURVIVOR B, or probes all red |

## 6. Evidence status

- `git status --short`: only `?? audit/AUD-047/` (this REPORT). NOTHING modified. No builds/tests run (no cargo/npm). Live DB never touched.
- STATE: DONE. REMAINING: owner P1–P3 probes (or direct BITMAP pivot order) → fix route. NEXT READY: YES.
