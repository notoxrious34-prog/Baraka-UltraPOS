# TASK-DSN-054 visual-acceptance assertions

Method: vite dev server on 127.0.0.1:5173 (this task only), headless Chrome via CDP,
Tauri `invoke` stub answering the exact wire shapes the frontend expects.
Data: 100% synthetic mocks (2 fake customers, empty supplier search, forced error).
No live database, no real customer data. Live paths (%APPDATA%/com.baraka.ultrapos)
never touched: the browser has no Tauri backend at all.

## dsn054-customers-empty-S-fr-light.png
- state=customers-empty tier=S lang=fr theme=light dpr=1 scope=<SECTION> darkClass=false docDir=ltr
- no-h-scroll: PASS (scrollWidth=1024 clientWidth=1024)
- no-page-v-scroll: PASS (scrollHeight=650 clientHeight=650; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 9.9:1 PASS (fg=rgb(51, 65, 85) bg=rgb(248, 250, 252))
- cta: direction=ltr centered=true (btnCx=712.0 scopeCx=712.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=ltr)

## dsn054-customers-empty-S-fr-dark.png
- state=customers-empty tier=S lang=fr theme=dark dpr=1 scope=<SECTION> darkClass=true docDir=ltr
- no-h-scroll: PASS (scrollWidth=1024 clientWidth=1024)
- no-page-v-scroll: PASS (scrollHeight=650 clientHeight=650; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 1.05:1 FAIL (fg=rgb(255, 255, 255) bg=rgb(248, 250, 252))
- body contrast: 1.18:1 FAIL (fg=rgb(226, 232, 240) bg=rgb(248, 250, 252))
- cta: direction=ltr centered=true (btnCx=712.0 scopeCx=712.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=ltr)

## dsn054-customers-empty-M-fr-light.png
- state=customers-empty tier=M lang=fr theme=light dpr=1 scope=<SECTION> darkClass=false docDir=ltr
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 9.9:1 PASS (fg=rgb(51, 65, 85) bg=rgb(248, 250, 252))
- cta: direction=ltr centered=true (btnCx=840.0 scopeCx=840.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=ltr)

## dsn054-customers-empty-M-fr-dark.png
- state=customers-empty tier=M lang=fr theme=dark dpr=1 scope=<SECTION> darkClass=true docDir=ltr
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 1.05:1 FAIL (fg=rgb(255, 255, 255) bg=rgb(248, 250, 252))
- body contrast: 1.18:1 FAIL (fg=rgb(226, 232, 240) bg=rgb(248, 250, 252))
- cta: direction=ltr centered=true (btnCx=840.0 scopeCx=840.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=ltr)

## dsn054-customers-empty-L-fr-light.png
- state=customers-empty tier=L lang=fr theme=light dpr=1 scope=<SECTION> darkClass=false docDir=ltr
- no-h-scroll: PASS (scrollWidth=1920 clientWidth=1920)
- no-page-v-scroll: PASS (scrollHeight=1080 clientHeight=1080; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 9.9:1 PASS (fg=rgb(51, 65, 85) bg=rgb(248, 250, 252))
- cta: direction=ltr centered=true (btnCx=1160.0 scopeCx=1160.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=ltr)

## dsn054-customers-empty-L-fr-dark.png
- state=customers-empty tier=L lang=fr theme=dark dpr=1 scope=<SECTION> darkClass=true docDir=ltr
- no-h-scroll: PASS (scrollWidth=1920 clientWidth=1920)
- no-page-v-scroll: PASS (scrollHeight=1080 clientHeight=1080; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 1.05:1 FAIL (fg=rgb(255, 255, 255) bg=rgb(248, 250, 252))
- body contrast: 1.18:1 FAIL (fg=rgb(226, 232, 240) bg=rgb(248, 250, 252))
- cta: direction=ltr centered=true (btnCx=1160.0 scopeCx=1160.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=ltr)

## dsn054-customers-empty-S-ar-light.png
- state=customers-empty tier=S lang=ar theme=light dpr=1 scope=<SECTION> darkClass=false docDir=rtl
- no-h-scroll: PASS (scrollWidth=1024 clientWidth=1024)
- no-page-v-scroll: PASS (scrollHeight=650 clientHeight=650; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 9.9:1 PASS (fg=rgb(51, 65, 85) bg=rgb(248, 250, 252))
- cta: direction=rtl centered=true (btnCx=312.0 scopeCx=312.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=rtl)

## dsn054-customers-empty-S-ar-dark.png
- state=customers-empty tier=S lang=ar theme=dark dpr=1 scope=<SECTION> darkClass=true docDir=rtl
- no-h-scroll: PASS (scrollWidth=1024 clientWidth=1024)
- no-page-v-scroll: PASS (scrollHeight=650 clientHeight=650; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 1.05:1 FAIL (fg=rgb(255, 255, 255) bg=rgb(248, 250, 252))
- body contrast: 1.18:1 FAIL (fg=rgb(226, 232, 240) bg=rgb(248, 250, 252))
- cta: direction=rtl centered=true (btnCx=312.0 scopeCx=312.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=rtl)

## dsn054-customers-empty-M-ar-light.png
- state=customers-empty tier=M lang=ar theme=light dpr=1 scope=<SECTION> darkClass=false docDir=rtl
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 9.9:1 PASS (fg=rgb(51, 65, 85) bg=rgb(248, 250, 252))
- cta: direction=rtl centered=true (btnCx=440.0 scopeCx=440.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=rtl)

## dsn054-customers-empty-M-ar-dark.png
- state=customers-empty tier=M lang=ar theme=dark dpr=1 scope=<SECTION> darkClass=true docDir=rtl
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 1.05:1 FAIL (fg=rgb(255, 255, 255) bg=rgb(248, 250, 252))
- body contrast: 1.18:1 FAIL (fg=rgb(226, 232, 240) bg=rgb(248, 250, 252))
- cta: direction=rtl centered=true (btnCx=440.0 scopeCx=440.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=rtl)

## dsn054-customers-empty-L-ar-light.png
- state=customers-empty tier=L lang=ar theme=light dpr=1 scope=<SECTION> darkClass=false docDir=rtl
- no-h-scroll: PASS (scrollWidth=1920 clientWidth=1920)
- no-page-v-scroll: PASS (scrollHeight=1080 clientHeight=1080; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 9.9:1 PASS (fg=rgb(51, 65, 85) bg=rgb(248, 250, 252))
- cta: direction=rtl centered=true (btnCx=760.0 scopeCx=760.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=rtl)

## dsn054-customers-empty-L-ar-dark.png
- state=customers-empty tier=L lang=ar theme=dark dpr=1 scope=<SECTION> darkClass=true docDir=rtl
- no-h-scroll: PASS (scrollWidth=1920 clientWidth=1920)
- no-page-v-scroll: PASS (scrollHeight=1080 clientHeight=1080; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 1.05:1 FAIL (fg=rgb(255, 255, 255) bg=rgb(248, 250, 252))
- body contrast: 1.18:1 FAIL (fg=rgb(226, 232, 240) bg=rgb(248, 250, 252))
- cta: direction=rtl centered=true (btnCx=760.0 scopeCx=760.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=rtl)

## dsn054-customers-empty-L-fr-light-dpr2.png
- state=customers-empty tier=L lang=fr theme=light dpr=2 scope=<SECTION> darkClass=false docDir=ltr
- no-h-scroll: PASS (scrollWidth=1920 clientWidth=1920)
- no-page-v-scroll: PASS (scrollHeight=1080 clientHeight=1080; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 9.9:1 PASS (fg=rgb(51, 65, 85) bg=rgb(248, 250, 252))
- cta: direction=ltr centered=true (btnCx=1160.0 scopeCx=1160.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=ltr)

## dsn054-customers-selected-M-fr-light.png
- state=customers-selected tier=M lang=fr theme=light dpr=1 scope=<SECTION> darkClass=false docDir=ltr
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digit "0550123456": bdi[dir=ltr]=false fontVariantNumeric="" FAIL
- digit "1 250,50 DZD": bdi[dir=ltr]=true fontVariantNumeric="tabular-nums" PASS
- digit "0550987654": bdi[dir=ltr]=false fontVariantNumeric="" FAIL
- digit "0,00 DZD": bdi[dir=ltr]=true fontVariantNumeric="tabular-nums" PASS
- body contrast: 8.31:1 PASS (fg=rgb(15, 23, 42) bg=rgba(245, 158, 11, 0.1))
- cta: direction=ltr centered=true (btnCx=199.5 scopeCx=200.0)

## dsn054-customers-selected-M-ar-light.png
- state=customers-selected tier=M lang=ar theme=light dpr=1 scope=<SECTION> darkClass=false docDir=rtl
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digit "0550123456": bdi[dir=ltr]=false fontVariantNumeric="" FAIL
- digit "1 250,50 د.ج": bdi[dir=ltr]=true fontVariantNumeric="tabular-nums" PASS
- digit "0550987654": bdi[dir=ltr]=false fontVariantNumeric="" FAIL
- digit "0,00 د.ج": bdi[dir=ltr]=true fontVariantNumeric="tabular-nums" PASS
- body contrast: 8.31:1 PASS (fg=rgb(15, 23, 42) bg=rgba(245, 158, 11, 0.1))
- cta: direction=rtl centered=true (btnCx=1080.5 scopeCx=1080.0)

## dsn054-suppliers-nomatch-M-fr-light.png
- state=suppliers-nomatch tier=M lang=fr theme=light dpr=1 scope=<SECTION> darkClass=false docDir=ltr
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digit "Ciment CPJ 45": bdi[dir=ltr]=true fontVariantNumeric="tabular-nums" PASS
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 9.9:1 PASS (fg=rgb(51, 65, 85) bg=rgb(248, 250, 252))
- cta: direction=ltr centered=false (btnCx=1042.0 scopeCx=640.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=ltr)

## dsn054-suppliers-nomatch-M-fr-dark.png
- state=suppliers-nomatch tier=M lang=fr theme=dark dpr=1 scope=<SECTION> darkClass=true docDir=ltr
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digit "Ciment CPJ 45": bdi[dir=ltr]=true fontVariantNumeric="tabular-nums" PASS
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 1.18:1 FAIL (fg=rgb(226, 232, 240) bg=rgb(248, 250, 252))
- cta: direction=ltr centered=false (btnCx=1042.0 scopeCx=640.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=ltr)

## dsn054-suppliers-nomatch-M-ar-light.png
- state=suppliers-nomatch tier=M lang=ar theme=light dpr=1 scope=<SECTION> darkClass=false docDir=rtl
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digit "Ciment CPJ 45": bdi[dir=ltr]=true fontVariantNumeric="tabular-nums" PASS
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 9.9:1 PASS (fg=rgb(51, 65, 85) bg=rgb(248, 250, 252))
- cta: direction=rtl centered=false (btnCx=139.8 scopeCx=640.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=rtl)

## dsn054-suppliers-nomatch-M-ar-dark.png
- state=suppliers-nomatch tier=M lang=ar theme=dark dpr=1 scope=<SECTION> darkClass=true docDir=rtl
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digit "Ciment CPJ 45": bdi[dir=ltr]=true fontVariantNumeric="tabular-nums" PASS
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 1.18:1 FAIL (fg=rgb(226, 232, 240) bg=rgb(248, 250, 252))
- cta: direction=rtl centered=false (btnCx=139.8 scopeCx=640.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=rtl)

## dsn054-suppliers-error-M-fr-light.png
- state=suppliers-error tier=M lang=fr theme=light dpr=1 scope=<SECTION> darkClass=false docDir=ltr
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 5.91:1 PASS (fg=rgb(185, 28, 28) bg=rgb(254, 242, 242))
- cta: direction=ltr centered=false (btnCx=1042.0 scopeCx=640.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=ltr)

## dsn054-suppliers-error-M-ar-light.png
- state=suppliers-error tier=M lang=ar theme=light dpr=1 scope=<SECTION> darkClass=false docDir=rtl
- no-h-scroll: PASS (scrollWidth=1280 clientWidth=1280)
- no-page-v-scroll: PASS (scrollHeight=720 clientHeight=720; inner regions may scroll)
- position:fixed in modified subtree: 0 (required 0) PASS
- emoji (U+1F300-U+1FAFF, U+2600-U+27BF) in modified subtree: false (required false) PASS
- digits: no digit-bearing text present in this state.
- title contrast: 17.06:1 PASS (fg=rgb(15, 23, 42) bg=rgb(248, 250, 252))
- body contrast: 5.91:1 PASS (fg=rgb(185, 28, 28) bg=rgb(254, 242, 242))
- cta: direction=rtl centered=false (btnCx=139.8 scopeCx=640.0)
- icon: centered=true (symmetric flex-col layout, direction-agnostic; document dir=rtl)

## Contrast summary (light theme)
- customers-empty (all tiers, fr+ar): title 17.06:1 PASS, body 9.90:1 PASS.
- customers-selected (M, fr+ar, light): body 8.31:1 PASS.
- suppliers-nomatch (M, fr+ar, light): title 17.06:1 PASS, body 9.90:1 PASS.
- suppliers-error (M, fr+ar, light): title 17.06:1 PASS, body 5.91:1 PASS.

## NEWLY DISCOVERED DEFECT (dark theme; reported per EDGE CASE, NOT patched)
All *-dark captures: teaching/no-match body text measures 1.05-1.18:1 (FAIL).
Root cause: src/index.css sets body{background:#f8fafc;color:#0f172a} with NO dark
variant, so base surfaces stay light in dark mode; the TASK-DSN-054 additions
(dark:text-white / dark:text-slate-200 in CustomerDossierPane teaching panel and
SupplierListScreen no-match panel) render near-white text on the undarkened surface.
Pre-existing pattern (old copy used theme-agnostic text-slate-500 and stayed readable);
the error panel is unaffected (it paints its own dark:bg-red-900/30 surface).
Remediation needs a design decision (darken surfaces vs drop dark: text) — left for Architect.

## Pre-existing digit-wrapping note (not introduced by this task, not patched)
customers-selected shots list phone numbers inside a bare <bdi> (no dir attr, no
tabular-nums) within a dir="ltr" <p> — from CustomerListPane.tsx (SCOPE-OUT, untouched).
Balance amounts in the same shots ARE bdi[dir="ltr"].tabular-nums (PASS).
The query interpolation introduced by this task IS bdi[dir="ltr"].tabular-nums (PASS).
