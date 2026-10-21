# TASK-BRK-079 REPORT — production worker proof + POS failure surface

## Phase A (engine): worker already runs in production — proof, no wiring added
- Spawn cited, all present in `src-tauri/src/lib.rs`: import `:36`
  (`spawn_print_worker, WorkerDeps`); stop-flag + handle cell `:71-77`;
  `spawn_print_worker(...)` inside setup `:157`; §34-35 shutdown protocol
  `:301` (stop-before-exit) + `:309-310` (bounded join).
- Checkout→POS link: `sales_checkout` returns `print_job_id` (`ipc.rs:1485`
  → `ipc.ts` `ReceiptWire`); chip polls `documents_get_print_status`
  (`ipc.rs:2800-2802`, `ipc.ts:931`) to terminal states.
- Terminal-state tests already green in-tree (`print_worker` 15/15, untouched):
  DONE/FAILED paths, `failed_print_leaves_accounting_byte_identical`
  (sale+ledger survive print death), `unconfigured_printer_is_bounded_and_never_spins`
  (unset-COM → bounded EXHAUSTED, never spins). No engine behavior added —
  FAIL-before N/A with reason (nothing to red-test; existing suite is the proof).

## Phase B (frontend): FAILED banner + chip hardened
- `src/components/pos/PrintStatusIndicator.tsx`: FAILED-only `role="alert"`
  guidance banner + `aria-live="polite"` root; reprint via `documents_reprint` untouched.
- `src/lib/i18n.ts`: `+1` key FR `"Vérifiez l'imprimante et les paramètres, puis réimprimez."`
  + AR `"تحقق من الطابعة والإعدادات، ثم أعد الطباعة."`
- Pins `src/components/pos/print-status-indicator.test.ts` 6/6: banner/role,
  aria-live, reprint wiring, FR key, AR key, `CheckoutModal.tsx:423` mount.
  Red→green demonstrated: 4 failed on stashed (unfixed) code, 6 passed restored.
- No POS→Settings navigation exists in-tree, so the "settings shortcut" is
  textual guidance (stated limitation).

## Captures (3/3 green, 0 console errors, noPageScroll)
- `pos-failed-1280x800-fr-light.png` (102714 B): banner "Vérifiez l'imprimante…",
  chip "Échec d'impression", "Réimprimer", live=true.
- `pos-failed-1280x800-ar-dark.png` (86054 B): RTL dark verified by vision —
  red `فشلت الطباعة` + guidance bar + `إعادة الطباعة`, no breakage.
- `pos-failed-1024x600-fr-light.png` (93685 B): same assertions pass.
- Flow per capture: login → Caisse → search "vis" → card click → F12 →
  tender 2000 → Valider → `sales_checkout` stub receipt → done screen →
  `documents_get_print_status` → FAILED (EXHAUSTED/PRINTER_OFFLINE semantics).
- Hook trace (`src/hooks/use-catalog-search.ts:39-53`): blank query fires
  `ipcSearchCatalog(token, "", 100, false)`; fixes applied to `capture.mjs`:
  bare-`[]` catalog array, flat cash session, `stock_get` single-object shape,
  `sales_preview_totals` stub (checkout prerequisite), AR theme label
  `المظهر`, AR product-name probe, span-click (card text lives in SPAN).

## Evidence copy (Step 0)
- `audit/BRK-076/fts80k-walltime.log` (197121 B): read-only copy of
  `$TMPDIR/fts80k.log` (24h-prune rescue). Headline: wall 16570.60s
  (~4h36m; sibling run 16925.37s), counts 80000/80000/170000, then FAIL at
  `fts_migration_tests.rs:473` `Port("catalog store failure")` — owned by
  BRK-076, not diagnosed here.

## Worktree / gates / cleanup
- G1-G7 + `git status` + cleanup proofs: see executor report (pasted below).
- `audit/BRK-079/`: REPORT.md + capture.mjs + capture-results.json + 3 PNGs.
  `diag.txt` deleted. Dev server killed. Scratch probe deleted. DO NOT COMMIT.

STATE: complete / DONE: worker proof + banner/chip/i18n + 6 pins + 3 PNGs +
REPORT + evidence copy / REMAINING: none.
