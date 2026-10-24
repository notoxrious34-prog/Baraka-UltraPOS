# TASK-DSN-090 REPORT — Shell restyle (nav + session strip + success popup + printer guidance)

Owner direction implemented; banner replaced by a small centered popup.
No wire/query/DTO, checkout, modal, transport, keybinding, or config changes.
No commit.

## (1) FAIL-before (quoted)

- Banner element (`PosShell.tsx:56-65`, pre-edit): full-width
  `bg-emerald-700` button rendering the success text; set from
  `CheckoutModal.tsx:300` (`setBanner(...)`), plus 5 more producer sites
  (`RecordPaymentModal:253`, `ParkedSalesModal:148,192`,
  `use-pos-keyboard.ts:126,130,190,194`, `quick-products-store.ts:198`).
- Clipped cash menu: `CashSessionIndicator.tsx:138` (`absolute end-0`
  under a strip-start pill → opens off-screen; RTL worse).
- Cryptic "C" toggle: `Topbar.tsx:141` (`{dark ? "☀" : "☾"}` — the
  crescent reads as "C"); text-only nav pills (`App.tsx` `entry()`),
  short brand `Baraka`.
- Emoji-as-icons: `OperationsMenu.tsx:100-107`
  (`conversion: "⇄"`, `return: "↩"`, `"warehouse-transfer": "⇄"`,
  `"stock-adjust": "⚖"`, `backup: "🛡"`, `identity: "🖨"`).

## (2) Component lines (post-edit)

- `src/components/icons.tsx` [CREATED, 19 icons]: inline SVG set, 1.75px
  stroke, currentColor, no emoji. Strip: Cash, Depot, Operations
  (reserved for menu restyle), User, Clock, Lang, Moon, Sun, Logout
  (`flipRtl`), ChevronDown. Nav: Pos, Catalog, Stock, Customers,
  Documents, Suppliers (`flipRtl`), Reports. C1 additions (max +3 used):
  `ShieldIcon` (backup), `PrinterIcon` (identity), `ChevronDownIcon`
  (menu toggles, identical in RTL). Settings gear redrawn mid-task:
  tangent teeth + hub (first pass read as sun — risked confusion with the
  theme Sun icon; re-shot reads as cog, vision-confirmed).
- `src/App.tsx`: brand `Baraka UltraPOS`; `entry()` takes
  `Icon` — all 9 pills icon+text, one pill language.
- `src/components/pos/Topbar.tsx` (piecewise rewrite): 3 zones —
  zone 1 cash+depot (never hidden; `DepotIcon`, select `title=` full name,
  label collapses ≤1100px), zone 2 center ops+alerts (`OperationsMenu`,
  offline pill, parked badge; labels collapse ≤1024px), zone 3 session
  group (`UserIcon`, `ClockIcon` + `<bdi dir=ltr>`, `LangIcon` pill,
  `MoonIcon`/`SunIcon` toggle, `LogoutIcon` + label). Flex mirrors in RTL;
  digits LTR-isolated.
- `src/components/pos/CashSessionIndicator.tsx`: menu `end-0` → `start-0`
  (in-viewport both dirs); toggle `▾` → `ChevronDownIcon`.
- `src/stores/pos-ui-store.ts`: `banner/setBanner` → `notice/notify/
  dismissNotice` (`ShellNotice {title, detail?}`); `openModal` clears
  notice (never stacked under a modal).
- `src/components/pos/NoticePopup.tsx` [CREATED]: `fixed top-24 z-40`
  (below modal z-50; moved from `bottom-20` after the first shot overlapped
  `TOTAL TTC`), `role=status aria-live=polite`, 4 s auto-dismiss from
  visibility, manual close. Coordination (stated, not blind): in-modal
  success stays while open; popup fires only on modal close
  (`finishSale` with `done`: `checkout.success + number · total ·
  print.pending`), so never two success messages simultaneously.
  RecordPayment/Parked/keyboard/quick-product producers `notify()` at the
  same points (popup appears after their modal closes via the modal gate).
- `src/features/settings/PrinterSettingsTab.tsx` + `src/lib/i18n.ts`:
  text-only COM guidance box (4 keys FR/AR: `printerComGuideTitle/Com/
  Usb/Transport` — Device Manager path, USB→virtual-COM note, transport-
  unavailable meaning). No logic change.
- `src/components/pos/OperationsMenu.tsx`: `ITEM_ICON` strings → icon
  components; trigger `▾` → `ChevronDownIcon`.

## (3) Pin lines (all green)

`src/components/pos/shell-restyle.test.tsx` 11/11: icon set present
(1.75/currentColor), brand + 9 nav icons, SVG moon/sun (no `"☀"`/`"☾"`),
menu `start-0` (no `end-0`), banner gone (shell+store+checkout), checkout
routes through `notify` on close, popup gated + timed + live region,
8 guidance keys FR/AR, store notify→dismiss + openModal-clears,
6 ops-icon mappings + 5 glyph absences.

## (4) PNGs + rect asserts (`audit/DSN-090/`, `capture.mjs` + CDP vs vite)

- `shell-1280x800-fr-light.png`: strip in-viewport
  (`STRIP-OK l:0,t:40,r:1280,b:80`), brand ok, no banner element,
  pre-clean ok; cash menu `MENU-OK {l:34,t:78,r:210,b:146}` then closed;
  end-to-end checkout (stubbed catalog/preview/checkout) → popup
  `POPUP-OK 450,96,830,158` (`Vente validée / T-2026-0099 · 30,25 DZD ·
  Impression en attente`), no TOTAL/footer overlap; `dismissed-ok`
  after 4.8 s; `noPageScroll`.
- `shell-1280x800-ar-dark.png`: `STRIP-OK` + `logout-ok`
  (تسجيل الخروج) + `noPageScroll`, `dir=rtl`, vision-verified mirror
  (logout arrow mirrors, digits LTR), no UUID.
- `shell-1024x600-fr-light.png` (light forced): `STRIP-OK r:1024`,
  `kept-cash-depot-logout`, `collapsed-ok {depot:0,offline:0,ops:0}`,
  `noPageScroll`.
- `capture-results.json`: CAPTURE GREEN 13/13, 0 console errors.
- Honest fixes en route: menu needs a paint tick between click and rect
  (React batching); cards need `div.cursor-pointer` wait (palette paints
  first); done button is `Vente suivante (Entrée)`; `dir` lives on the
  shell div; `bg-emerald-700` is legitimately reused (Cash pill, CreditHud)
  so the banner assert targets bare-`Vente validée` buttons outside
  `[role=status]` instead.

## (5) Gates

| Gate | Result |
|---|---|
| G4 `npm run lint` | EXIT 0 |
| G5 `npm run typecheck` | EXIT 0 |
| Targeted Vitest | 11/11 |
| G6 `npm run build` | EXIT 0 (5.59 s) |
| G7 full Vitest | 76 files / 768 tests green |
| G1–G3 | N/A — zero Rust (state reason) |

## (6) Emoji proof + FUTURE-SWEEP (untouched, SCOPE-OUT)

Scoped grep (all touch files, pictographs + symbols + VS16): EMPTY after
the chevron purge (only hit was a `→` in a keyboard comment, reworded).
Repo-wide genuine emoji remain OUTSIDE this scope — recorded, not fixed:
`CartTable.tsx:169 ⚠️`, `BackupModal.tsx:465 💾`, `CatalogModule.tsx:641
🏷️ / :653 ✏️`, `CategoryManageModal.tsx:275 🔧 / :428 ✏️`,
`DataExchangeMenu.tsx:103 📥 / :114 📦 / :126 📤 / :137 📄`, plus text
symbols `✓✗↺↻←→↑↓✕` in BackupModal/ModalShell/StockAdjustModal/
StoreIdentityModal/KeyFooter/CartTable. Proposed follow-up: one task
per surface, SVG-or-text per case.

## (7) Footprint

[MODIFIED] icons-consumer set: `icons.tsx` (+Shield/Printer/ChevronDown),
`App.tsx`, `Topbar.tsx`, `CashSessionIndicator.tsx` (+1 import line, C2),
`OperationsMenu.tsx`, `pos-ui-store.ts`, `NoticePopup.tsx` (top-24),
`CheckoutModal.tsx`, `RecordPaymentModal.tsx`, `ParkedSalesModal.tsx`,
`use-pos-keyboard.ts`, `quick-products-store.ts`, `i18n.ts` (+8 keys),
`PrinterSettingsTab.tsx`, `shell-restyle.test.tsx` (+mapping pin).
[CREATED] `audit/DSN-090/REPORT.md`, `capture.mjs`,
`capture-results.json`, `capture.log`, `vite.log`, 3 PNGs.
No commit. Live DB never opened. No strays (sweep §8).

## (8) NO_LEFTOVERS + port proof

Vite (own PID, started for captures) stopped; `:5173` verified free
(`Get-NetTCPConnection -LocalPort 5173` → no rows); Chrome headless
killed by the capture script; `dist/` is gitignored build output;
no `zz_*` probes; audit dir holds only REPORT + capture artifacts +
vite.log + 3 PNGs.

## STATE / DONE / REMAINING

STATE done. DONE: restyle + popup + guidance + swap + 3 vision-checked
shots + all frontend gates green. REMAINING: owner visual acceptance;
FUTURE-SWEEP emoji task; USB-transport sequencing. NEXT READY: YES.
