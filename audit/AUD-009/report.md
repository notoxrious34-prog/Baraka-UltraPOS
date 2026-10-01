# AUD-009 — maximized-open proof, startup timeline, host profile (3 runs)

Header: TASK-AUD-009, 2026-10-01. HEAD `ecbef3635ac9292522077426b43a5d50bd702e95` (BRK-060).
Repo `D:/Dev/Baraka UltraPOS`, branch master, tree clean. Scratch `D:/Dev/_scratch/AUD-009/` (deleted, see §7).
Env: DPI 144 (150%), monitor 1920x1080 phys, work area [0,0,1920,1034] phys = 1280x689.3 logical.
Data dir isolated via session-only `BARAKA_DATA_DIR`; CDP port 9222 (runs 1-2), none (run 3).

## 0. Preflight — PASS
HEAD == ecbef363..., HEAD~1 == 69f6aebe (AUD-008), HEAD~2 == 938e93e (AUD-007). Tree clean.
`git diff 69f6aebe..HEAD` = tauri.conf.json (+1), window_config.rs (+28), ADR-066 (+7). Port free; no Baraka processes. Step 0.5 baselines FP_APPDATA f487a65b (10) / FP_LOCAL 9566ede0 (1599).

## 1. Source/config — PASS
Ordered setup (src-tauri/src/lib.rs): manage(stop) 77 → app_data_dir 84-87 → BARAKA_DATA_DIR resolve/prepare 91-96 → ensure_database_dir 97 → DB open 100-101 → **run_migrations 102-103** → device_id 107 → state 110-143 → manage 144 → spawn_print_worker 145-155 → **WebviewWindowBuilder::from_config 174** → (override) data_directory 180 → build 182-184 → set_title 187 → handlers+build+run 192-269. **DB open+migrations run BEFORE the window build.** No `.plugin()` calls; 0 tauri-plugin-* crates in Cargo.lock. No additional_browser_args / WEBVIEW2_* env in code (only builder.data_directory on override). No runtime file logging; only stderr on startup failure. Config: title, create:false, 1280x640, min 1024x600, center, **maximized:true**, resizable.

## 2. Build — PASS
`npm run tauri -- build --no-bundle` exit 0, 2m24s. exe `target/release/ultrapos.exe` 14873088 B,
SHA-256 `94480efd5b9e81b7952a05ec7a77a4fc5529ac149baf7051c35f19a7671ea71e` (≠ AUD-007 `82e5deed…`).
mtime newer than commit; `git status` clean after (only untracked audit/AUD-009/).

## 3. Host profile
Win10 Pro 10.0.19045; Intel i5-3450 4C/4T; 16 GB RAM; both disks SSD; C: 3.9 GB free, D: 40.5 GB;
power plan High performance; Defender RealTimeProtection Enabled; WebView2 runtime 154.0.4258.37 (HKLM; HKCU absent);
GPU GTX 1650; 1 monitor 1920x1080 phys = 1280x720 logical; DPI 144.

## 4-5. Runs (all timings are ms since launch T0; "win visible first state" from the sampler)
| metric | Run1 COLD (CDP) | Run2 WARM (CDP) | Run3 WARM (no CDP) |
| window created & visible | 405 | 466 | 359 |
| first-visible state | zoom=true (rect -7,-7,1288,697) | zoom=true | zoom=true |
| **FLASH test** | **NO FLASH** | **NO FLASH** | **NO FLASH** |
| title override [BARAKA_DATA_DIR] | 1073 | 866 | 817 |
| first webview2 child | 819 | 466 | 359 |
| TCP 9222 open | 819 | 866 | n/a (no CDP) |
| CDP pageTarget / content-ready | 6299 / **6317** | 4001 / **4028** | n/a (no CDP) |
| screenshot | 6424 (29863 B, saved) | 4134 | none |
| app CPU [0→window] | 140 ms | 62 ms | 78 ms |
| app CPU util window→content | 109/5498 = 2% | 156/3562 = 4% | n/a |
| app CPU idle (content→end) | 2% | 2% | 2% |
| sampler gaps | max 476 / median 42 ms | max 423 / median 42 | max 346 / median 41 |

**Maximized geometry (Win32 + CDP, run1-2):** DWM visible frame [0,0,1920,1034] = **work area (margins 0/0/0/0)**.
Client 1280x666.7 logical; CDP iw=1280 ih=667, ow=1280, oh=668, dpr=1.5. **No overflow**: document scrollWidth==clientWidth, scrollHeight==clientHeight (no scrollbars). Client origin (0,34) phys →
**non-client top in maximized = 34 −(−11) = 45 phys = 30 logical**. availH(690)−clientH(667)=22.7 logical ≈ clientOrigin y 22.7 logical → **"title bar = 23px gap" hypothesis SUPPORTED**.

**Restore (run2):** SW_RESTORE → outer [-11,14,1931,1030], client **1280x640**, zoomed=false, CDP iw=1280 ih=640;
horizontal gaps 0/0 (fills width), vertical top 59 phys / bottom 15 phys (not vertically centered). SW_MAXIMIZE back → DWM frame [0,0,1920,1034] (work area), client 1280x666.7, no overflow.

**Idle self-maximize watch:** no window rect/zoom/iconic changes during idle in any run → no self-maximize.

**Filesystem alignment (run1 cold, relative to T0):** baraka.db created +108 ms (WAL written +172 ms, 729 KB) →
**before the window (+405 ms)**; webview2 profile files ~+535 ms; children +819 ms.

**Gap analysis — where do ~5 s between window-created and content-ready go?**
Measured facts: window visible (zoomed) and webview2 children exist by ~0.4-0.8 s; CDP content-ready at ~4.0 s (warm) / ~6.3 s (cold); the app process uses only ~2-4% CPU in that interval (working set/threads/handles flat), then idle.
Ranked hypotheses:
- **H1 (SUPPORTED): the interval is wait, not compute.** App CPU util window→content is 2-4% (109 ms / 156 ms), so the app is blocked/idle, not crunching. Evidence: CPU deltas above.
- **H2 (SUPPORTED): the delay is WebView2 engine + frontend boot.** Children spawn at ~0.4-0.8 s but the page/document become ready ~3-6 s later; the app process does no work meanwhile, so the time goes to WebView2 profile/renderer init and first render + the 474 kB frontend bundle. Evidence: child 0.82 s → content 6.32 s (cold), flat app CPU/ws.
- **H3 (NOT SUPPORTED as cause of the ~5 s): the earlier "~4.7 s gap before any webview2 child" is a sampler artifact.** The AUD-008 CIM-based child poll reported first-child ~4.7 s; the present toolhelp sampler sees children at 0.36-0.82 s across all runs. Evidence: run1-3 child first-seen here.
Debugging-flag effect: window-created 405/466/359 ms (cold/CDP, warm/CDP, warm/no-CDP) are within ~100 ms → the `--remote-debugging-port` flag does not materially delay window creation; cold-vs-warm content (6.3 s vs 4.0 s) is the profile effect, not the flag.

## 6. Shutdown + isolation — PASS
Each run closed via WM_CLOSE; no app/WebView2 process with "AUD-009" in cmdline remains (only the agent's own shell matched); port 9222 free. FP_APPDATA_1 == f487a65b (10) == baseline; FP_LOCAL_1 == 9566ede0 (1599) == baseline. EQUAL both.

## 7. Bundle + commit
report.md, measurements.json, raw/ (clean run1-cdp, run2-cdp, run3 summaries, events, downsampled samples, results.jsonl, fs-aligned data, build log tail, meta listings incl. full APPDATA, LOCAL count+fingerprint only), screenshots/aud009-open-first-screen.png, scripts/. One commit, exact subject. No gates G1-G8 (no source change; forbidden-path diff empty). Scratch deleted (see Step 7 log).

## Anomalies
- `lang="fr"`, `dir=""` while visible UI is Arabic RTL (frontend fact, unchanged from AUD-008).
- Process-name strings read back as mojibake from toolhelp (`PE32.szExeFile` CharSet mismatch) — pids are authoritative and correct.
- Sampler runs at ~42 ms/sample (not 20 ms); it cannot sustain 20 ms but reports the true gap.
- run1's first CDP content-ready is an upper bound (harness attach), but sub-ms sampled window/child/CPU data is clean; run2/3 CDP timing is clean.
- Restored window is not vertically centered on the work area (top 59 / bottom 15 phys) despite `center:true` (maximized restore position retained).