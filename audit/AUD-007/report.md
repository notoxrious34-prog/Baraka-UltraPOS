# AUD-007 — First real run on an isolated data directory

Commit scope: `audit/AUD-007/` only (this report, measurements.json, screenshots/, scripts/).
No source change. HEAD at run time: `58aa6718ac4e0a05cbfd2f1f08c08f31549dc8da`, tree clean.

## 0. Preflight

- HEAD `58aa6718ac4e0a05cbfd2f1f08c08f31549dc8da`, `git status` clean (`## master`).
- Run protocol (docs/DEV_RUN_PROTOCOL.md) + ADR-076 in <= 10 lines:
  1. Full app runs via `npm run tauri dev` (dev) or a release build with embedded frontend.
  2. Fresh DB shows the First-Run Setup Wizard (no seed user); `bootstrap_first_admin` creates the admin.
  3. Production DB is `%APPDATA%\com.baraka.ultrapos\baraka.db` (ADR-049).
  4. ADR-076: `BARAKA_DATA_DIR` (absolute, no `..`, outside the default dir) redirects the data dir.
  5. Invalid override aborts startup; default dir is never a fallback.
  6. In override mode the window title carries `[BARAKA_DATA_DIR]` and the WebView2 profile goes inside the override dir.
  7. Launch used here: release build `--no-bundle` + env-only `BARAKA_DATA_DIR` + `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`.
- No baraka/msedgewebview2/tauri/ultrapos process running; `BARAKA_DATA_DIR` unset (User/Machine/Process);
  port 9222 free; disk free D: 41.1 GB, C: 4.0 GB.
- `node v26.7.0`, `rustc 1.98.1`, Windows `Microsoft Windows NT 10.0.19045.0`,
  `NVIDIA GeForce GTX 1650`, 1920x1080.

## 1. Live-data metadata baseline (names/sizes/timestamps only, never contents)

- `%APPDATA%\com.baraka.ultrapos`: 10 entries.
  `\baraka.db | 368640`, `\baraka.db-shm | 32768`, `\baraka.db-wal | 4120032`,
  7x `printer-capture\test-*.escpos` (26515/30979 bytes).
  FP_APPDATA_0 = `f487a65b08bb140e4a0186aee61102692afd11cf5d95492df5141e9cd2d098f1`
- `%LOCALAPPDATA%\com.baraka.ultrapos`: 1599 entries (WebView2 EBWebView profile of the live app).
  FP_LOCAL_0 = `9566ede057b84a9d470eb4d086a616b1569d301b4f6ae6d2d077f82c26bffda3`

## 2. Build (`npm run tauri -- build --no-bundle`, CARGO_BUILD_JOBS=2)

- Start 2026-10-01T14:35:48.928Z, end 2026-10-01T14:40:23.081Z (~4.6 min), exit 0.
- Last log lines: `Compiling ultrapos`, `Compiling baraka-infrastructure`,
  `Finished release profile in 3m 18s`, `Built application at: D:\Dev\Baraka UltraPOS\target\release\ultrapos.exe`, `BUILD_EXIT:0`.
- Exe: `D:/Dev/Baraka UltraPOS/target/release/ultrapos.exe`, 14873088 bytes,
  SHA-256 `82e5deed4a45d1d0f9ec1c1e68471a06a8df742dc86300e19fd5d7f75e9f7839`, mtime 2026-10-01T14:39:26Z.
- `git status` after build: clean except untracked `audit/AUD-007/` (this task's own new dir).

## 3. Launch (BARAKA_DATA_DIR=D:\Dev\_scratch\AUD-007\data, port 9222)

- PID 10232, T0 epoch ms 1790865638339, env vars unset from the session right after Start-Process.
- Milestones ms since T0 (approximate, dev machine, 50 ms polling; NOT a contract, NOT POS-ready):
  (a) MainWindowHandle 550; (b) page target listed <= 7010 (present at first successful CDP poll);
  (c) readyState complete 7010; (d) #root children 7010 (count 1); (e) body text 7010 (len 138).
- First-screen text verbatim (first 200 chars, Arabic preserved):
  `ب / إنشاء حساب المدير / التشغيل الأول — أنشئ حساب مدير هذه الطرفية / اسم المستخدم /
  كلمة المرور / تأكيد كلمة المرور / إنشاء الحساب (Entrée) / Français`
- CDP method proven: global fetch + WebSocket to 127.0.0.1:9222 (scripts/cdp.mjs).

## 4. Isolation proof

- Win32 window title: `Baraka UltraPOS [BARAKA_DATA_DIR]` (marker present). document.title: `Baraka UltraPOS`.
- Scratch data dir holds: `baraka.db` 4096 (fresh), `baraka.db-shm` 32768, `baraka.db-wal` 729272,
  `webview2/` 258 files / 31910430 bytes.
- 6/6 msedgewebview2 processes (PIDs 5684,2412,15292,9100,14008,1084, all under PID 10232)
  with `--user-data-dir=D:\Dev\_scratch\AUD-007\data\webview2\EBWebView`; 0 without the isolated path.
- Memory idle at first screen (not a contract): app 34.0 MB; total app+descendants 349.7 MB
  (5684: 114, 2412: 10.1, 15292: 74.0, 9100: 35.4, 14008: 18.4, 1084: 63.7).

## 5. Window measurement (DPI 144, per-monitor v2, factor physical*96/dpi)

Default/restored state, isZoomed false:
- outer physical [-11,14,1931,1030] (1942x1016) -> logical pos (-7.3, 9.3), size 1294.7x677.3
- client physical 1920x960 at (0,59) -> logical 1280.0x640.0 at (0, 39.3) — exactly the configured size
- visible frame physical [-1,14,1921,1020] -> logical pos (-0.7, 9.3), size 1281.3x670.7
- monitor [0,0,1920,1080] -> 1280x720; work [0,0,1920,1034] -> 1280x689.3
- CDP: inner 1280x640, outer 1280x641, DPR 1.5, screen 1280x720, avail 1280x690,
  doc client/scroll 1280x640/1280x640, body font `Inter, Cairo, system-ui, "Segoe UI", Tahoma, sans-serif`,
  document.fonts 0 loaded.
- Fit verdicts: (i) FAIL strict — visible frame 1 physical px outside work area left (-1) and right (1921);
  top/bottom inside; GetWindowRect shadow reaches -11/+1931. (ii) PASS — 677.3 <= 689.3.
  (iii) bottom margin 9.3 logical px visible (14 px; GetWindowRect 2.7). (iv) PASS — client diff 0.0/0.0.
  (v) PASS — left -1 px vs right +1 px, symmetric within 2 px tolerance (centered, 1 px symmetric overhang).
- S1 `screenshots/aud007-default-first-screen.png` (29529 bytes, IHDR 1920x960 = 1280x640 @1.5x — exact match).
  Visual: complete Arabic-RTL first-run admin-creation screen (logo ب, 3 empty fields, orange
  `إنشاء الحساب (Entrée)`, `Français` toggle); no clipping, no scrollbar.
- ANOMALY: between the first info/metrics reads and S1, the window became maximized by itself
  (isZoomed true, frame == work area, client 1920x1000 -> inner 1280x667). Cause UNKNOWN — read-only grep
  over `src` and `src-tauri/src` finds no maximize/fullscreen/set_size call (only generated Tauri schemas).
  Restored via ShowWindow(SW_RESTORE); restore bounds intact; all numbers above re-verified after restore.
  The first S1 capture (1920x1001, transitional) was discarded and re-taken at the true default.

## 6. Minimum-size probe (SetWindowPos client 800x450 logical, own window only)

- Result: client physical [1200,675] = logical [800,450]; CDP inner 800x450.
  minWidth: FAIL (800 < 1024, not clamped). minHeight: FAIL (450 < 600, not clamped).
  (Programmatic SetWindowPos path; interactive-drag enforcement not tested by this task.)
- At 800x450: scroll == client on both axes, 0 elements beyond viewport — first-run screen complete, unclipped.
- S2 `screenshots/aud007-min-size-first-screen.png` (25295 bytes, IHDR 1200x675 = 800x450 @1.5x — exact match).
- Restored to 1280x640 logical client; CDP innerWidth/innerHeight 1280x640 verified.

## 7. Shutdown

- WM_CLOSE posted; app exited after 703 ms. taskkill NOT needed.
- Leftover check: all 7 PIDs gone; no process with AUD-007 in its command line except the querying
  shells themselves; port 9222 free again.
- Final data dir: baraka.db 4096, -shm 32768, -wal 729272, webview2/ 258 files.

## 8. Isolation re-check

- FP_APPDATA_1 = FP_APPDATA_0 (`f487a65b…098f1`), 10 entries — EQUAL (full listings byte-identical).
- FP_LOCAL_1 = FP_LOCAL_0 (`9566ede0…6bffda`), 1599 entries — EQUAL (full listings byte-identical).

## 9. Bundle and commit

- audit/AUD-007/: report.md, measurements.json, screenshots/ (2 PNG), scripts/ (meta-list.ps1,
  launch.ps1, window-ops.ps1, cdp.mjs). Text files LF, UTF-8 no BOM.
- build.log (26 lines, BUILD_EXIT:0, quoted in §2 above) was scratch-only; scratch deleted per 9.2.
- No source change: forbidden-path diff empty (see commit evidence in chat report).

## Method notes / conflicts

- Bash (not cmd) shell: `< /dev/null` instead of `< NUL`; `ls`-missing instead of Test-Path for deletion proof.
- `$env:` expansion was mangled by the bash layer, so live-dir paths were resolved in Node and passed
  as literals to meta-list.ps1 (metadata only, unchanged semantics).
- No task-vs-constitution conflicts. No forms submitted, no installer, no persistent env changes.
