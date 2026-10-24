# TASK-DSN-091 REPORT — Printer settings UI for transport selection

STATE DONE (sealed). No commit. Live DB never opened.

## (0) Wire note (disclosed deviation, no engine edits)

BRK-089 `0913214` shipped the Rust wire (`transport` + `queue_name`,
serde-defaulted) but NOT the `src/lib/ipc.ts` facade. The tab could not
save the selection without the pass-through, so the facade was extended
(+18/-1, defaults keep the old 3-arg call shape). The alternative —
bypassing the facade with raw `call<>` in the component — was rejected
as the worse pattern. Zero Rust/production-engine changes.

## (1) FAIL-before (quoted)

Tab (pre-change `PrinterSettingsTab.tsx:1-219`): serial-only form —
`serialPort`, `serialBaud`, `paperWidthMm` state only;
`ipcUpdatePrinterSettings(token, serialPort, serialBaud, paperWidthMm)`.
No "transport"/"queue" except key reference
`:161 {t(locale, "settings.printerComGuideTransport")}`.
Facade: `PrinterSettings` 4 fields, update takes 4 args —
`grep -i transport|queue ipc.ts` hit only unrelated comments.
Guidance text denied the feature:
"l'impression USB directe n'est pas prise en charge" /
"غير مدعومة".

## (2) Component lines

- `src/lib/ipc.ts`: `PrinterTransport` ('serial'|'spooler') + 2 fields;
  get maps `transport === 'spooler' ? … : 'serial'`,
  `queueName ?? ''` (stub-proof); update sends `transport` +
  `queue_name`.
- `src/features/settings/PrinterSettingsTab.tsx`: `transport`/`queueName`
  state (independent — switching never clears); `role="group"` segmented
  control (`aria-pressed`, `SettingsIcon` = Série COM port params,
  `PrinterIcon` = USB printer); serial → port+baud grid, spooler →
  queue input (`placeholder` example, help line, `<bdi dir="ltr">`);
  paper width always visible; empty-spooler save → inline
  `printerQueueRequired`, no IPC; test-print buttons untouched (route
  server-side).
- `src/lib/i18n.ts`: +7 keys × FR/AR (Title, Serial, Spooler, QueueName,
  QueuePlaceholder "Ex. Xprinter XP-410B", QueueHelp, QueueRequired);
  rewrote `printerComGuideUsb/Transport` (USB-direct path documented,
  virtual-COM kept as alternative).
- Pins: `src/features/settings/printer-transport.test.ts` (9 its).

## (3) Pin lines

Segmented pressed-states (`Série COM=true,USB direct=false` default);
icon reuse + pictograph code-point scan (regex tripped
`no-misleading-character-class` — rewritten, disclosed); conditional
visibility (COM gated serial, queue gated spooler); paper width before
guide box; LTR + no `setSerialPort("")`/`setQueueName("")`;
guard-before-IPC ordering + required message; facade request/response
shape; 7 keys × 2 locales; stale-denial strings gone. 9/9 GREEN.

## (4) PNG names + asserts (`capture-results.json` GREEN 9/9, 0 console)

- `printer-1280x800-fr-light.png`: serial pressed default, COM1 in
  input, queue hidden → `USB direct` pressed, queue visible →
  empty-save blocked (no IPC) → filled `Xprinter XP-410B`, payload
  `{transport:spooler, queue_name:Xprinter XP-410B}` →
  `RECT-OK:transport-group {240,56,1264,98}`,
  `RECT-OK:queue-input {240,134,1264,168}`, noPageScroll. Vision: USB
  selected, queue filled, updated guide, saved-notice, no defects.
- `printer-1280x800-ar-dark.png`: Caisse detour (theme/lang live in POS
  Topbar only — theme-first), `سلسلة COM=true,USB مباشر=false`,
  COM1+9600 visible, queue hidden, `rtl-ok` (app-root `div[lang]`,
  App.tsx:234 — `documentElement.dir` assert was wrong, fixed),
  `RECT-OK {16,56,1040,98}` mirrored, noPageScroll. Vision: RTL mirror,
  serial selected, no defects, Latin tech tokens intact.
- Capture honesty fixes: snake_case identity stub (settings view crashed
  on camelCase stub — stub bug, not app), U+2019 apostrophes in asserts,
  input-value (not innerText) for COM visibility.

## (5) Gate table

| Gate | Result |
|---|---|
| G4 `npm run lint` | EXIT 0 (after test-only pictograph-scan fix) |
| G5 `npm run typecheck` | EXIT 0 |
| G6 `npm run build` | EXIT 0, 5.57s |
| G7 `npx vitest run` | EXIT 0 — 77 files / 777 tests passed, 0 failed (76/768 + pins file) |
| G1–G3 | N/A — zero Rust, state reason |

## (6) Cleanup + status

NO_LEFTOVERS: `zz_probe.mjs` deleted during the task; no `zz_*`/probe
files remain; `audit/DSN-091/` holds only REPORT + capture.mjs +
results + 2 PNGs + vite.log.
Vite :5173: holder is PID 13924 (pre-existing repo vite, NOT started
this session — must not kill); my own `npm run dev` attempt exited on
port conflict. Port is serving current tree (captures GREEN against it).
DO NOT COMMIT honored.

git status --short (expected):
M `src/features/settings/PrinterSettingsTab.tsx`, `src/lib/ipc.ts`,
`src/lib/i18n.ts`; ?? `src/features/settings/printer-transport.test.ts`,
`audit/DSN-091/`. No other paths touched.

## HARDWARE-PENDING + STATE/DONE/REMAINING

HARDWARE-PENDING: stubs only — live save to the XP-410B queue is the
owner USB test (unblocked next).
STATE: DONE. DONE: both locales GREEN 9/9 + visions, G4–G7 green with
counts, REPORT written, cleanup proven, diff limited to scope.
REMAINING: none. NEXT READY: YES.
