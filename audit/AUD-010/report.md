# AUD-010 — frontend startup breakdown (navigation, paint, resources, child CPU, 4 runs)

Header: TASK-AUD-010, 2026-10-01. HEAD `8abe3c66e2d94765e1ac058f6fc8e67f3faea2df`, tree clean.
Repo `D:/Dev/Baraka UltraPOS`. Scratch `D:/Dev/_scratch/AUD-010/` (deleted after bundle). Exe `target/release/ultrapos.exe` SHA-256 `94480efd5b…` (unchanged, MATCH). Env: DPI 144, 1920x1080 phys = 1280x720 log, work [0,0,1920,1034], WebView2 154.0.4258.37. Data accounted only to raw/ + scratch + audit/AUD-010/.

## 0. Preflight — PASS
HEAD `8abe3c66e2d94765e1ac058f6fc8e67f3faea2df`; log -6 in raw/results.jsonl (AUD-009 order resolved: **`43e895b6` = 29-file bundle, `8abe3c6` = BOM fix**). Tree clean. `git diff ecbef36..HEAD` touches ONLY `audit/AUD-009/` (29 files, +1564). Forbidden paths diff empty. exe exists 14,873,088 B / SHA-256 `94480efd5b9e81b7952a05ec7a77a4fc5529ac149baf7051c35f19a7671ea71e` / mtime 2026-10-01T16:31:06Z. No AUD-0 ultrapos/msedgewebview2 procs; BARAKA_DATA_DIR/WEBVIEW2_* clear. Baselines: APPDATA f487a65b (10), LOCAL 9566ede0 (1599).
0.6 BOM/validity scan (report only): AUD-007 1 json clean; **AUD-008 11 files, 6 with UTF-8 BOM that fail strict JSON.parse** (`raw/run{1,2,3}-{launch,watch}.json`); AUD-009 13 files clean. Offenders listed in raw/bom-scan.txt.

## 1. Child CPU from AUD-009 raw (no launch)
The committed AUD-009 samples carry per-pid `cpuMs` but **no `--type` role flag** → per-role attribution UNKNOWN there. From the downsampled samples (full files were deleted with scratch): child total CPU (all WebView2 children summed) `[win->content]` ≈ run1 1888 ms / 5.9 s = 32%, run2 1592 ms / 3.6 s = 45%; child CPU `[0->win]` ≈ 0 (children spawn at the window). First-sample bound: run1/run2 first sample at 405 ms / 466 ms → "window/child first-seen" is an **upper bound**, not an event time. Step 3 records roles properly.

## 2. Source + config (no launch)
2.1 Boot path (execution order, src): `index.html:2 <html lang="fr">`, `:11 module script /src/main.tsx`; `vite.config.ts` single entry, build.target es2022, no code-splitting/manualChunks; `main.tsx:12 ReactDOM.createRoot(...).render(<App/>)`; `App.tsx` → `session===null` → `UnauthenticatedGate` (107-108); UnauthenticatedGate initial render = loading div with `requiresSetup===null` (84-89); `useEffect` :67 calls `ipcCheckFirstRunStatus()`; `lib/ipc.ts:117` → `call("check_first_run_status")` :122 → `invoke` (PRE_AUTH). On resolve → `FirstRunSetupScreen` (form+inputs, :45-81; **no mount IPC**, nothing on render except reading store). **Only IPC before first-run-form render = `check_first_run_status`** (count 1). Store hydration sync (Zustand getState, no await); i18n = pure `t(locale,key)` tables, no async load.
**Language decision**: default locale = **`"ar-DZ"`** (`src/stores/pos-ui-store.ts:28`); `App.tsx:103-105` maps locale → `dir={"ar-DZ"?"rtl":"ltr"}` / `lang={"ar-DZ"?"ar":"fr"}` / font-arabic. The static document keeps `index.html lang="fr"`, no dir → `documentElement.lang="fr" dir=""` while the rendered content div is `dir=rtl lang=ar` (Arabic RTL). This **explains** the observed `lang=fr + dir=""` + Arabic RTL screen. ADR-069 "default French" is NOT reflected by the actual default (`ar-DZ`); recorded as fact (not fixed).
2.2 Fonts/external: **no** `@font-face`, `<link>`, `url(http)`, `fonts.gstatic/googleapis`, or `wss://` anywhere in `src/` or `index.html` (0 matches; runtime `document.fonts` = loaded, size 0, no families → system fonts only). dist grep: only self-references.
2.3 CSP: `app.security.csp` = **absent** (`null`). `frontendDist = ../dist`, `devUrl = http://localhost:5173`.
2.4 dist: 3 files, **513,338 B total**; single JS `assets/index-o949h9zE.js` **486,063 B** (no code-splitting), css 26,759 B, index.html 516 B. Largest 5 = those 3.

## 3. Runs (1 COLD + 3 WARM; all ms since T0)
Clock alignment: `timeOrigin` (page) vs process `t0EpochMs` → offset = A segment; harness-measured pageTarget ≈ ws-open ±10 ms; alignment uncertainty ≈ ±60 ms (harness wall vs page timeOrigin drift).
`form-found` is a **poll-upper-bound** (CDP page target only exists from ~3-5 s, so the form is already present at first poll).

| metric | Run1 COLD | Run2 WARM | Run3 WARM | Run4 WARM |
| process → nav start (A) | **1019** | **779** | **737** | **684** |
| A CPU (all roles) | 748 ms / 73% | ~0 | ~0 | ~0 |
| nav → FCP (B) | 384 | 304 | 392 | 332 |
| B CPU / util | 594 ms / 155% | 1154 ms / **380%** | 1251 ms / **319%** | 1030 ms / **310%** |
| FCP (msT0) | 1403 | 1083 | 1129 | 1016 |
| first-paint | 1367 | 1083 | 1129 | 1016 |
| LCP | 1535 | 1119 | 1129 | 1016 |
| load (DOMContentLoaded) msT0 | 1353 | 994 | 951 | 898 |
| exe CPU total (0→form) | 31 ms | 15 ms | ~32 ms (B) | 16 ms (B) |
| form-found (upper) | 3383 | 5147 | 3390 | 3093 |
| C = form−FCP (upper) | 1980 | 4064 | 2261 | 2077 |
| LoAF (JS bundle frame) | 0 (late attach) | **251 ms** | **344 ms** | **294 ms** |
| longtask | 0 | 0 | 0 | 0 |
| TCP (CDP) open | 1169 | 1139 | 1002 | 917 |

Per-role CPU, phase B (nav→FCP), ms: run2 {**browser 375**, gpu 328, utility 233, renderer 203, exe 15}; run3 {**browser 485**, gpu 375, utility 187, renderer 156, exe 32}; run4 {**browser 422**, gpu 281, utility 171, renderer 140, exe 16}. Exe is idle everywhere (≤32 ms over seconds).
Per-role CPU totals 0→form run1 {exe 31, browser 781, gpu 546, renderer 218, utility 171} — **browser process dominates**.

Resource waterfall (identical all runs): 4 resources, **513,338 B decoded**; the single JS bundle (486,063 B, d≈14-15 ms transfer) + CSS (26,759 B) + IPC `check_first_run_status` (fetch, d 3-7 ms) + favicon. **startedAfterFCP = 0**; **external (non-local) origin = 0** — all four are the app's own `http://tauri.localhost/` origin (the resource-classifier labels them "other" because the protocol is http, but there is no cross-domain request). No CDN/fonts/external fetch possible at boot.

Bottleneck (facts): the one uncoded-split 486 KB bundle executes in a **single long animation frame of 251-344 ms** (LoAF script attribution = `index-o949h9zE.js` only) that sits between nav start and FCP, across renderer+gpu+utility+browser at 310-380% combined util; the WebView2 **browser process** is the largest single CPU contributor in both phase A (cold boot 531 ms) and phase B. Exe idle.

Warm spread (total to form-found, upper bounds): **min 3093, median 3390, max 5147 ms** (run2's 5147 is a CDP page-target boot outlier: ~5.1 s vs ~3.1-3.4 s).

## 4. Floor measurement — NOT RUN (UNKNOWN)
WebView2 runtime standalone `C:\Program Files (x86)\Microsoft\EdgeWebView\Application\154.0.4258.37\msedgewebview2.exe` with `--user-data-dir`(AUD-010 path) `--remote-debugging-port=9223` → **exposed no page target** on 9223 with both a `data:` URL and a `file://` URL (retry limit reached). Floor = **UNKNOWN**. All floor-profile processes cleaned; ports 9222/9223 free.

## 5. Shutdown + isolation — PASS
Each run closed WM_CLOSE → taskkill fallback only if cmdline had AUD-010. After last: no process with "AUD-010" in cmdline except this session's own shell; ports 9222/9223 FREE. FP_APPDATA_1 = f487a65b (10) == Step 0.5; FP_LOCAL_1 = 9566ede0 (1599) == Step 0.5. EQUAL both (no live-data change).

## 6. Bundle + commit
report.md, measurements.json (NO BOM), raw/ (per-run breakdown+miles+events+results.jsonl+roles+downsampled samples+BOM scan+floor+local-fingerprint+appdata-listing), scripts/ (startup-timeline.ps1 w/ role enrichment, startup-breakdown.mjs, cdp-floor.mjs). Samples >200 KB → downsampled "every 5th" still 635 KB > 200 KB, kept **every 20th** (as AUD-009) and said so. One commit, exact subject. Scratch deleted (failing ls). No gates (task forbids). No amend/push/tag.

## Anomalies / deviations
- resource `origin` classifier labels the 4 app resources "other" (http protocol); after review all are app-origin (tauri.localhost) → external = 0 (reported as such).
- LoAF/longtask buffered observers attach only at CDP page-target time (~3-5 s), after load; run1 cold recorded 0 LoAF (buffer/late), warm runs captured 1 LoAF each (251/344/294 ms).
- `check_first_run_status` appears as a resource (initiator fetch) so IPC round-trip is visible in resource timing (3-7 ms) — the bridge itself is not a bottleneck.
- Older prepared full-file listing referenced in AUD-009 (data1-filesystem-aligned); only summary numbers were logged here.
- `performance.getEntriesByType('mark')` unavailable in the app (no user marks; marks array empty).

## REMAINING
None for this task. (Subsequent architectural work: consider code-splitting the 486 KB bundle and/or reducing its main-thread LoAF; verify ADR-069 French-default intent vs actual `ar-DZ` default.)