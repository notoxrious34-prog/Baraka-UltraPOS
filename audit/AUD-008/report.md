# AUD-008 — window min-size enforcement, self-maximize watch, startup timeline (3 runs)

Header: TASK-AUD-008 + TASK-AUD-008-C1, 2026-10-01. HEAD `938e93e5cb6b0bfd17cecfaa04ea207427f9a974`.
Repo `D:/Dev/Baraka UltraPOS`, branch master. Scratch was `D:/Dev/_scratch/AUD-008/` (deleted, see §6).
Env: Windows 10, DPI 144 (150%), monitor 1920x1080 phys, work area [0,0,1920,1034] phys
(= 1280x689.3 logical). App `target/release/ultrapos.exe` (AUD-007 build, source unchanged).
Data dir isolated via session-only `BARAKA_DATA_DIR`; CDP port 9222; `BARAKA_DATA_DIR` marker in title.

## 0. Preflight — PASS
Clean tree except untracked `audit/AUD-008/`; forbidden diff `58aa671..HEAD`
over crates/src/src-tauri/package.json/package-lock.json/Cargo.lock empty.
Run-3 instance alive at handoff (pid 1696, hwnd 1245926, CDP page target present).

## 1. Raw-file recomputation — PASS, discrepancies: none
| run | handle | tcp | pageTarget | content | shot | wv2 first | wv2 n | watch changes |
|---|---|---|---|---|---|---|---|---|
| 1 COLD | 115 | 709 | n/a (old harness) | 17571 | 17676 (29529 B) | 13789 | 6 | 0 |
| 2 WARM | 179 | 801 | 6743 | 6759 | 6859 (29529 B) | 5026 | 6 | 2 (fg only) |
| 3 WARM | 122 | 726 | 6333 | 6357 | 6445 (29529 B) | 4703 | 6 | 0 |
Run-2 watch: rect constant `[-11,14,1931,1030]`; toast "New notification" took
foreground +18887..+20025 ms only. Launch loop gap max 8 ms; CDP single-poll gap 0.
Caveat: run-1 COLD used the earlier harness (no pageTarget sub-milestone);
cold content time 17571 ms is single-source, lower confidence.
Warm first-paint ≈ 6.7 s, cold ≈ 17.5 s — real, sub-milestone instrumented.
Language facts (runs 1+2, fresh isolated profile, identical, nothing persists):
`lang="fr"`, `dir=""`, `navigator.language="en-GB"`, `languages=["en-GB","en","en-US"]`,
localStorage `{}`, sessionStorage no keys. First screen is Arabic RTL
("إنشاء حساب المدير" — create manager account) with a "Français" toggle button.
Self-maximize: **0 events in all 3 runs** (AUD-007 event not reproduced).

## 2. Window info — PASS
DPI 144. Outer `[-11,14,1931,1030]` phys = 1294.7x677.3 logical.
DWM visible frame `[-1,14,1921,1020]` phys (hr=0). Client origin (0,59) phys.
Client 1920x960 phys = **1280x640 logical**. zoomed=false, iconic=false.
Visible-frame vs work margins (phys): left −1, right −1, top +14, bottom +14
(logical −0.7/−0.7/+9.3/+9.3). NCD outer−client = [22,56] phys.

## 3. Programmatic probes — all NOT CLAMPED (win32 + CDP agree, stable >500 ms)
| probe | requested client | immediate | +500 ms | CDP iw×ih | verdict |
| A SetWindowPos | 800x450 | 800x450 | 800x450 | 800x450 | NOT CLAMPED |
| B MoveWindow | 800x450 | 800x450 | 800x450 | 800x450 | NOT CLAMPED |
| C SetWindowPos | 1000x580 | 1000x580 | 1000x580 | 1000x580 | NOT CLAMPED |
Probe D maximize: zoomed=true, outer `[-11,-11,1931,1045]`, DWM frame
`[0,0,1920,1034]` = work area exactly (margins 0/0/0/0), client 1280x666.7,
CDP 1280x667 dpr 1.5, `scrollWidth==clientWidth`, `scrollHeight==clientHeight`
(**no overflow, no scrollbars**). Screenshot `aud008-maximized-first-screen.png`:
first-run screen fills work area, no clipping. SW_RESTORE returned outer
`[-11,14,1931,1030]` (0 px diff), zoomed=false, client 1280x640 verified.
MINMAXINFO (from AUD-008): ncd [22,56], ptMinTrackSize [1558,956] phys
= **client [1024,600] logical** — cross-process SendMessage IS marshaled.

## 4. Real drag — control PASS, 4b/4c CLAMPED
4a control (BR→1150x620): grab (1919,1019) ht=17 → **[1150,620] PASS**,
anchor (−11,14) unchanged. (An earlier attempt with off-screen grab points
failed at [1280,644] — documented harness flaw, superseded.)
4b BR→800x450: outer `[-11,14,1547,970]` (1558x956 phys = ptMinTrackSize),
client **[1024,600] — CLAMPED**. 4c TL→800x450: grab (0,14) ht=13,
outer `[373,74,1931,1030]` (right/bottom unchanged), client **[1024,600] —
CLAMPED both axes**. All ≤1.1 s, cursor restored. Reset to 1280x640 verified.
Optional 30 s idle tail: NOT RUN (budget discipline).

## 5. Shutdown + isolation — PASS
WM_CLOSE → pid 1696 exited; no webview2 remains; only own shell matched
"AUD-008" cmdline; port 9222 FREE. FP_APPDATA_1 ==
`f487a65b08bb140e4a0186aee61102692afd11cf5d95492df5141e9cd2d098f1`
(10 entries) EQUAL; FP_LOCAL_1 ==
`9566ede057b84a9d470eb4d086a616b1569d301b4f6ae6d2d077f82c26bffda3`
(1599 entries) EQUAL.

## 6. Bundle + commit
`report.md`, `measurements.json`, `raw/` (run files, watch, results-c1,
appdata listings, local fingerprints only), `screenshots/` (run-1 + maximized),
`scripts/` (launch, maximize-watch, cdp-harness, window-ops, dwm-frame,
maximize-restore, drag-probe, cdp-size). One commit, exact subject.
No gates G1–G8 (no source change; forbidden-path diff empty).
REMAINING: none (optional idle tail omitted by design).

## Anomalies
- `lang="fr"` + empty `dir` while visible UI is Arabic RTL (unexplained; app fact).
- Maximized client height 666.7 < availHeight 690 (~23 logical px unaccounted).
- Run-1 cold content 17.5 s vs warm ~6.7 s (fresh WebView2 profile cost).
- AUD-007 self-maximize never reproduced; toast stole foreground briefly in run 2.
