# AUDIT-REPORT-3 — read-only UI audit (TASK-AUD-003 + C1)

Method: static Vite build + node:http server + headless Edge over CDP + canned
`window.__TAURI_INTERNALS__.invoke` stub (injected via
Page.addScriptToEvaluateOnNewDocument inside the capture script only).
LAYOUT ONLY — no Rust engine ran, no real database was opened.

## 1. Preflight (C0)

- Date: 01/10/2026, 11:28 local (Africa/Algiers). `ver`: Windows 10.0.19045.6466. `node -v`: v26.7.0.
- `git rev-parse HEAD`: `05f0f3975f7e2ccd201d4d1795abbf99696b5bb6` (matches expected baseline).
- `git status --short` (start and end): only `?? audit/AUDIT-REPORT-2.md`, `?? audit/measurements/`,
  `?? audit/screenshots/`, `?? docs/architecture/` (pre-existing untracked; tracked files unmodified).
- LIVE_BEFORE-C1 (metadata only): `%APPDATA%/com.baraka.ultrapos`: baraka.db 368640 B
  (28/09/2026 14:57:00), baraka.db-shm 32768, baraka.db-wal 4120032, printer-capture dir;
  `%LOCALAPPDATA%/com.baraka.ultrapos/EBWebView` profile tree. Matches session-1 values.
- Scratch: `D:/Dev/_scratch/aud003` existed with dist/, edge*, harness/, out/; :8137 listener PID 2484
  (own server); audit/screenshots + audit/measurements listed in §7.

## 2. Carried findings F1–F8 (re-verified)

F1 (REPRODUCED): Win 10.0.19045.6466; node v26.7.0; msedge.exe 154.0.4258.37 at
`C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`;
`target/release/ultrapos.exe` 14848000 B 30/09/2026 18:53:52;
installed `C:\Users\X1\AppData\Local\Baraka UltraPOS\ultrapos.exe` 14848000 B 30/09/2026 18:52:06.

F2 (REPRODUCED): AGENTS.md exists (3363 B), CONSTITUTION_OF_EXECUTOR.md exists (13720 B).
First 40 lines of AGENTS.md:
```
# Baraka UltraPOS — Executor Engineering Contract
## Authority
`CONSTITUTION_OF_EXECUTOR.md` is the binding executor constitution ...
The latest `Baraka UltraPOS — Master Sovereign Architecture & Product Design v1.0.0` is the authoritative ...
## Execution
- Execute exactly one `TASK-BRK-XXX` per session command. ... - Never modify `baraka.db`, `*.db-wal`, or `*.db-shm`.
- Never add dependencies unless the Task Brief explicitly authorizes them. ...
## Architecture Baseline
React/TypeScript → Typed Tauri IPC → Rust Commands → Rust Application → Rust Domain → Rust Infrastructure → SQLite/SQLCipher.
- Rust Money = i64 centimes newtype. ... - IPC monetary wire representation must be canonical decimal strings.
```
First 40 lines of CONSTITUTION_OF_EXECUTOR.md:
```
# CONSTITUTION OF THE EXECUTOR - Baraka UltraPOS
## Senior Principal Software Engineer - Opencode Desktop Agent
**Version:** 1.0.0 - GOLD EXECUTOR EDITION ... **Date:** 2026-09-25
### PREAMBLE ... You are the HAND. The Architect is the BRAIN. ...
### CHAPTER 1: IDENTITY AND ABSOLUTE OBEDIENCE
**Article 1.1 - Identity:** You are `EXECUTOR-BRK-01`. Your expertise: TypeScript (strict), React,
Tauri/Rust or Electron, SQLite (WAL, transactions, migrations), Drizzle ORM, Zustand, ...
**Article 1.2 - Chain of Command:** ... **Article 1.3 - Law of Verbatim Execution:** ...
```

F3 (REPRODUCED): no `INSERT into tax_rules` anywhere (grep over crates/**, src/**, src-tauri/**,
migrations). DDL only at `crates/infrastructure/migrations/0001_baseline_schema.sql:260`.
Timbre at `crates/domain/src/sales/pricing.rs:150-169` (`TIMBRE_MIN_CENTIMES: i64 = 500` L150,
`TIMBRE_MAX_CENTIMES: i64 = 250_000` L152, ceil-to-100 of 1% + clamp, Ticket/Facture + cash only).
TVA is a per-line `rate_bp` parameter (`pricing.rs:26,59`); no default-rate constant.
Replay of all 13 migrations with node:sqlite on a scratch in-memory DB: `mig=13 tax=0 set=4`,
settings = printer.transport/SERIAL, printer.serial_port/"", printer.serial_baud/9600,
printer.paper_width_dots/576 (seeded by `0006_print_worker_columns.sql:14`). `device_id` is added only
at first app start (`terminal_context.rs:20-47`).

F4 (REPRODUCED): `src-tauri/src/lib.rs:68-74` resolves `app.path().app_data_dir()`; DB file
`baraka.db` (`connection.rs:31-32`, `default_db_file_name` L20-22). Re-grep for `std::env::var`,
`env::args`, `BARAKA_` in production code: only `BARAKA_DRILL_*` in `crates/infrastructure/tests/`.
REDIRECT_MODE = NONE confirmed.

F5 (REPRODUCED): locale `src/stores/pos-ui-store.ts:10-36` (zustand, default `ar-DZ`, toggle only,
no persistence). Dark theme exists: `tailwind.config.js:4` `darkMode: "class"`, `dark:` count = 558,
toggle `Topbar.tsx:25-31,101-107`. Views `app-view-store.ts` + NavRail `App.tsx:31-54`. Logout button
`Topbar.tsx:108-115` (red, no aria-label). Header `h-14` `Topbar.tsx:47`. Cart
`section.col-span-5` `CartPanel.tsx:19`. Six fixed buttons `PosShell.tsx:74-160`
(`fixed bottom-* end-4 z-40`, `fixed` count in file = 6). PosShell root `h-screen w-screen` L64.

F6 (REPRODUCED), verbatim window block (`src-tauri/tauri.conf.json:13-23`):
`{"title":"Baraka UltraPOS","width":1280,"height":800,"minWidth":1024,"minHeight":768,
"resizable":true,"fullscreen":false}`, `app.security.csp: null`. No decorations/center keys.

F7 (REPRODUCED) [INFERRED-FROM-CODE]: `migration.rs:101-146` — creates schema_migrations if missing,
loops over embedded MIGRATIONS 1..13 only, verifies recorded checksums, inserts missing rows. It never
lists or inspects rows outside 1..13, so a newer-version row would open silently. Checksum mismatch
returns `ChecksumMismatch` (L126-131) → `lib.rs:77-78` startup error, no window. Probes A/B BLOCKED
(REDIRECT_MODE=NONE); not attempted.

F8 (SUPERSEDED by C4): session-1 font stacks confirmed; platform-font evidence below.

## 3. Capture method and limits

- Option (a) (real app over CDP) BLOCKED: REDIRECT_MODE=NONE; launching any binary opens the live DB.
- Option (b) PROVEN: `vite build --outDir D:/Dev/_scratch/aud003/dist` (13 s, local vite, no install),
  node:http static server :8137, headless Edge `--window-size=1280,800` with scratch `--user-data-dir`,
  CDP via global fetch+WebSocket, stub injected before every load.
- Proof (00-proof-capture.png): `Runtime.evaluate` → title "Baraka UltraPOS", 1250x705, dpr 1,
  first-run Arabic body; PNG 17642 B, magic 89504e470d0a1a0a.
- Stub modes driven by sessionStorage STATE (`audSetup` 1/0, `audRich` 1/0): FIRST_RUN, LOGGED_OUT,
  LOGGED_IN (ADMIN, cash session closed — stub `cash_get_active_session` → null).
- LIMITS: layout only. Empty catalog/search/suppliers/reports (stubs return []); modal shells render but
  context calls return null (hence "Erreur interne du noyau" in conversion modal — STUB ARTIFACT, not an
  app bug). Native headless window 1250x705 is NOT the real Tauri window.

## 4. Measurement table + real-screen facts

Native row = headless default 1250x705 — NOT the real Tauri window. `h=true` on pos-fr rows is a
scrollbar-gutter artifact (deScrollW == innerWidth; clientW = inner − 15px v-scrollbar); no genuine
horizontal overflow anywhere (scrollW never exceeds innerWidth).

| screen | viewport | h | v | #fixed | fixed∩TOTAL | fixed∩cart | logout visible | topbar clipped |
|---|---|---|---|---|---|---|---|---|
| login fr+ar | native, V1..V5 (12 rows) | false | false | 0 | — | — | n/a (no header) | 0 |
| pos-fr empty | native, V1..V5 | true* | true | 6 | 0 | 6 (57984 px²) | true | 0 |
| pos-ar empty | native, V1, V2, V4 | false | true | 6 | 1 (تحويل 360 px²) | 6 (56304 px²) | FALSE (x=-3) | 1 |
| pos-fr 1 line | V1, V2 | true* | true | 6 | 0 | 6 | true | 0 (hero 120 000,00 DZD) |
| pos-fr 13 lines | V1, V2 | true* | true | 6 | 1 (Convertir 954 px²) | 6 | true | 0 (hero 1 292 722,19 DZD) |
| pos-ar 13 lines | V1, V2 | false | true | 6 | 1 (تحويل 2160 px²) | 6 | FALSE (x=-3) | 1 |

Raw JSONs: audit/measurements/login-{fr,ar}-*.json (12), pos-fr-*.json (6), pos-ar-*.json (4),
pos-1line-fr-*.json (2), pos-14lines-{fr,ar}-*.json (4, see naming note §7).

Real screen (DPI-aware PowerShell, System.Windows.Forms): physical Bounds 1920x1080, physical
WorkingArea 1920x1034, DPI 144 = 150%. Logical working area = 1280 x 689.
- tauri.conf minHeight 768 vs working 689 → window taller than available screen: YES (768 > 689).
- default 1280x800 vs 1280x689 → YES (800 > 689; height overflows by ~111 logical px).
- minWidth 1024 vs 1280 → NO (fits). Default width 1280 == working width exactly.

## 5. Console / stub-call summary (POS load, fr, native)

`audit/measurements/console-pos-fr.json`: count 0, byType {}, first10 [], exceptions [].
Zero console errors/warnings/exceptions from login submit through POS settle + 2 s.
`audit/measurements/stub-calls-pos-fr.json`: 5 calls in order —
check_first_run_status (keys []), auth_login (request), warehouses_list (session_token),
cash_get_active_session ×2 (session_token). Unknown (stub returned null): none.
So the POS issues exactly these engine commands on open: first-run probe (gate), login, warehouse list,
cash active-session ×2.

## 6. Fonts report

`audit/measurements/fonts-platform-fr.json`, `fonts-platform-ar.json`.
`document.fonts.size` = 0, zero @font-face rules — no webfonts bundled or loaded.
Computed stacks: body `Inter, Cairo, system-ui, "Segoe UI", Tahoma, sans-serif`
(src/index.css:17); `.font-arabic` `Cairo, Inter, ...` (index.css:30);
hero `.text-4xl` `"JetBrains Mono", ui-monospace, ...` via `.font-mono` (TotalsFooter.tsx:45);
tailwind `fontFamily.sans=[Inter,Cairo,...], arabic=[Cairo,Inter,...], mono=[JetBrains Mono,...]`
(tailwind.config.js:fontFamily block).
Rendered (CSS.getPlatformFontsForNode):
- brand/logo glyph (ب): Tahoma, 1 glyph (fr and ar).
- hero total: Consolas 8 glyphs (fr) / Courier New 8 glyphs (ar).
- header button: Inter 17 glyphs (fr); Inter 1 + Segoe UI 9 (ar). `.font-arabic` node: no platform fonts.
Verdicts — Cairo rendered: NO. JetBrains Mono rendered: NO. Inter rendered: YES (system font,
17 glyphs on fr header button). Noto Naskh Arabic rendered: NO (Arabic glyphs fall back to Tahoma).

## 7. Screenshot index

51 PNGs verified: magic 89504e470d0a1a0a, size > 0, IHDR dims read — 51/51 valid.
Full table: audit/screenshots/INDEX.md. Gallery: audit/screenshots/index.html (static, no scripts).
Renames applied (old → new): 22× `*-native.png → *-1250x705.png` (02, 03-dark, 04–12 fr+ar);
`03-pos-ar-light-native.png` DELETED (superseded by measured c1 file);
`03-pos-fr-light-native.png` (s1) DELETED (superseded by measured c1 file);
4× `13-pos-14lines-* → 13-pos-13lines-*` (evidence shows 13 cart rows: first product added twice,
qty 2, total 1 292 722,19 DZD — file names corrected to truth); session-1 mislabeled `01-login-*`
+ `login-*.json` deleted in C1 (they showed first-run).
Observations (vision-verified): 01-login fr clean TRUE login; 01-login ar TRUE login, RTL correct;
03-pos fr: duplicated cash-closed banner, depot clipped "DEP-01 — Dépôt Pr", 6 floating pills overlap
cart, totals values clipped; 03-pos 1024x600: logout visible, pills cover cart, h-scroll;
04-conversion: modal renders + stub-artifact error box; 13-pos: TOTAL 1 292 722,19 readable but
Convertir overlaps totals block, pills cover ~half the cart. Session-1 modal/view/dark shots kept,
not re-shot (same stub shapes).

## 8. TVA-rate origin (C8)

In production, the TVA rate of a sale line is determined by a hardcoded 0: both pricing call sites in
`crates/application/src/use_cases/sales.rs` build `PricedLineInput { ..., tva_rate_basis_points: 0, ... }`
(L94-104 with comment "Pure pricing (TVA baseline rate is 0 pending the tax engine)", and L674 second
path) — the frontend sends no rate (no tva field on the checkout request DTO; `tva_rate` fields exist
only on return-context *output* DTOs, `dto.rs:1844,1965`), no product column is read for it, and the
`tax_rules` table is never SELECTed by production code (zero references in application/commands/domain
outside DDL + the migration test table list). Conclusion: TVA is structurally 0 on every sale until a
tax engine is built; `tax_rules` is dead schema today.

## 9. C9 finalisation

- LIVE_AFTER-C1: appdata top-level identical except `baraka.db-shm` LastWriteTime 30/09 21:36:47 →
  01/10 11:29:06 (size unchanged 32768; db 368640 and wal 4120032 byte-identical, same timestamps);
  EBWebView tree same names, several LastWriteTime touched 01/10 ~11:29 (BrowserMetrics, Default,
  Last Version, Local State, Variations*). LIVE UNCHANGED BY OUR PROCESSES: YES — none of our processes
  can touch those paths (no Tauri backend ran; Edge used scratch profiles; server served scratch dist);
  timestamp-only movement with identical sizes is consistent with the owner's own app use during the
  session window. No content was opened, hashed, or copied — metadata only.
- `git status --short` / `git rev-parse HEAD` (05f0f39): tracked files unchanged (only pre-existing
  untracked `??` entries).
- Processes terminated: node server PID 2484 (Stop-Process, verified ours via CommandLine); Edge PIDs
  3636/13568/3540 already exited (Browser.close + SIGKILL in drivers). Verified: no process has scratch
  path in CommandLine; ports 8137/9223 free.
- NO_LEFTOVERS: deleted `D:/Dev/_scratch/aud003` entirely (dist/, edge*, edge2/3/4, edge-c1/,
  harness/{srv,cap,run,run2,run3,c1,idx}.mjs, out/ incl. status JSONs, fresh.db, pids). audit/ contains
  only: AUDIT-REPORT.md, AUDIT-REPORT-2.md, AUDIT-REPORT-3.md (this file), measurements/ (39 JSONs),
  screenshots/ (51 PNG + INDEX.md + index.html).

## 10. UNEXPECTED FINDINGS and UNKNOWNs

- AR topbar overflow: in every ar POS measurement the logout button sits at x=-3 (outside viewport,
  `logoutFullyVisible:false`) and one topbar child group starts at x=-3 (3 px clipped) — RTL overflow
  defect, present at all sizes including 1536 wide.
- Cash-closed banner text renders twice ("…vendre. .Veuillez…" — PosShellView.tsx:33 string plus store
  banner; vision-confirmed duplication, likely double render path).
- 13th product never added in C6 (clicked first card twice → qty 2); totals/TVA lines unreadable behind
  Convertir at 1024x600.
- UNKNOWN: real Tauri window chrome (decorations unknown — config has no decorations key; headless
  inner 1250x705 ≠ real window). UNKNOWN: dark-theme shots for screens other than POS (only POS dark
  captured). UNKNOWN: probe A/B observed behaviour (BLOCKED by design).

## 11. Appendix A — harness source (trimmed essentials)

```js
// server: node:http static dist on 127.0.0.1:8137
import http from "node:http"; import fs from "node:fs"; import path from "node:path";
const ROOT = "D:/Dev/_scratch/aud003/dist";
http.createServer((req, res) => {
  const u = decodeURIComponent(req.url.split("?")[0]);
  let f = path.normalize(path.join(ROOT, u === "/" ? "index.html" : u.slice(1)));
  if (!f.startsWith(path.normalize(ROOT))) { res.writeHead(403); res.end(); return; }
  fs.readFile(f, (e, d) => { if (e) { res.writeHead(404); res.end("nf"); return; }
    res.writeHead(200); res.end(d); });
}).listen(8137, "127.0.0.1");
// launch: spawn msedge --headless=new --remote-debugging-port=9223
//   --user-data-dir=<scratch>/edge-c1 --window-size=1280,800 <url>, detached+unref
// CDP client: fetch /json/list -> page target -> global WebSocket;
// send(method,params) with id map; Runtime.evaluate; Page.captureScreenshot;
// Emulation.setDeviceMetricsOverride/clearDeviceMetricsOverride;
// Runtime.consoleAPICalled + Log.entryAdded + exceptionThrown collectors.
// stub (addScriptToEvaluateOnNewDocument; STATE survives reload via sessionStorage):
window.__TAURI_INTERNALS__ = { invoke: (cmd, args) => {
  window.__audCalls.push({ cmd, keys: args ? Object.keys(args) : [] });
  const S = (k) => sessionStorage.getItem(k);
  if (cmd === "check_first_run_status") return Promise.resolve({ requires_setup: S("audSetup") === "1" });
  if (cmd === "auth_login") return Promise.resolve({ session_token: "stub", user_id: "u",
    username: args.request.username, role: "ADMIN", expires_at: new Date().toISOString() });
  if (cmd === "warehouses_list") return Promise.resolve({ warehouses:
    [{ id: "w1", code: "DEP-01", name: "Dépôt Principal", is_active: true }] });
  if (cmd === "cash_get_active_session" || cmd === "catalog_search" && S("audRich") !== "1")
    return Promise.resolve(cmd === "catalog_search" ? [] : null);
  if (cmd === "catalog_search") return Promise.resolve(RICH_14_PRODUCTS); // canonical-string money
  if (/^(customers|suppliers|reports|backups)_/.test(cmd) || cmd === "printer_get_settings")
    return Promise.resolve(CANNED[cmd]);
  console.warn("STUB_UNHANDLED:" + cmd); return Promise.resolve(null); } };
// measurement (Runtime.evaluate): inner/outer, DPR, screen, de/body scroll+client,
// hasScroll booleans, all position:fixed (tag, aria-label/text, rect), TOTAL-hero/cart/header/
// logout rects, fixed∩TOTAL/cart (bool+area), topbar children past edges, bodyTextHead (120ch);
// fonts: document.fonts, computed fontFamily, CSS.getPlatformFontsForNode per DOM.querySelector node.
// naming: NN-<screen>-<lang>-<theme>-<WxH>.png; JSONs mirror tags; every capture asserts its screen.
```
