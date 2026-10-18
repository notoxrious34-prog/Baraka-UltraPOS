# TASK-DSN-067 — Universal Instant Search Completion: REPORT

HEAD at start: `586042d`, clean. CRLF: `grep -c $'\r'` = 0 on all touched files
before and after (LF-only, zero drift).
Real paths came from `audit/AUD-024/REPORT.md` (task's guessed `src/features/*/<Modal>`
paths do not exist; actuals are `src/components/pos/modals/*` + purchase screen).

## 1. UPGRADED COMPONENTS

- Transfer picker — `src/components/pos/modals/WarehouseTransferModal.tsx`
  (AUD-024 §2: `:391-409`). Manual `runPickerSearch` (Enter + mislabeled
  `addItemToTransfer` search button) replaced.
- Purchase search — `src/features/purchases/PurchaseOrderFormScreen.tsx`
  (`ProductSearchInput`, AUD-024 §2: `:72-91`). Manual `runProductSearch`
  (Enter + `sup.search` button) replaced.
- Return lookup — `src/components/pos/modals/ReturnModal.tsx:156-179` +
  `src/stores/return-modal-store.ts:searchAndLoadDocument` (AUD-024 §2).
  Manual Enter + `returns.search` button replaced.
- New shared `src/lib/debounced-search.ts`: `createDebouncedSearchRunner`
  (`search`/`flush`/`reset`/`dispose`/`isLatest`, `shouldSearch` gate, default
  non-empty). Transfer/purchase 175ms; return 200ms + `length >= 2` gate.
  Store adds monotonic `searchTicket` (ticket taken before `searchDocuments`;
  stale responses/error/flag-clearing ignored when superseded).

Diff pattern (each component, DSN-064 shape):
```ts
const searchRef = useRef<...>(null);
if (searchRef.current === null) {
  const runner = createDebouncedSearchRunner({ delayMs: 175,
    onSearch: (firedQuery, ticket) => {
      void ipcSearchCatalog(token, firedQuery, 12)
        .then((found) => { if (runner.isLatest(ticket)) setResults(found); })
        .catch(() => { if (runner.isLatest(ticket)) setResults([]); });
    }});
  searchRef.current = runner;
}
useEffect(() => { searchRef.current?.search(query); if (query.trim() === "") setResults([]); }, [query]);
useEffect(() => () => { searchRef.current?.dispose(); }, []);
```
Enter → `search()+flush()` (immediate; return: `reset()` + direct
`searchAndLoadDocument`, barcode-safe, bypasses the ≥2 gate). Escape → `reset()`
+ clear query/results (return: Escape closes per global contract — unchanged).
Manual search buttons removed in all three (result-row add buttons untouched).
Transfer busy line uses `text-ink-muted` (NOT slate-500 — no new contrast debt).

## 2. FAIL-THEN-PASS

4 files, 15 tests: `src/lib/debounced-search.test.ts` (5 fake-timer: latest-only
fire, flush, reset/empty-gate, ≥2 gate, stale-ticket) + 3 source-contract suites.
BEFORE: 9 failed (lib module absent; manual triggers present). AFTER: 15/15.
(One test-expectation fix during GREEN: `isLatest` is evaluated at resolve time,
so the stale test asserts staleness after the second fire, not during the first.)

## 3. GATES

1. fmt 0. 2. clippy Finished dev, 0. 3. cargo test 463/0 (unchanged).
4. lint 0. 5. typecheck 0. 6. build 5.35s, 0.
7. vitest 65 files / **625 passed** (610 + 15 new).

## 4. VISUAL (1280×720, stubbed invoke, no Enter pressed anywhere)

- `dsn067-1-transfer-instant.png`: typed "Cab" → 4 CAB-00 body mentions (2 stub
  products) — selector note: transfer renders results as div/span rows with a
  separate add button, so button-hit counting (purchase pattern) read 0 while
  the list HAD rendered; body-mention count is the valid probe here.
- `dsn067-2-purchase-instant.png`: typed "Cab" → 2 candidate buttons.
- `dsn067-3-return-instant.png`: typed "FAC" → context `FAC-2026-001` loaded.
  Variance: no "candidate list" exists by design — the modal direct-loads the
  first match into context (unchanged semantics, now debounced).
`VISUAL_DONE instant-verified`.

## 5. FILES

- [CREATED] `D:/Dev/Baraka UltraPOS/src/lib/debounced-search.ts`
- [CREATED] `D:/Dev/Baraka UltraPOS/src/lib/debounced-search.test.ts`
- [MODIFIED] `D:/Dev/Baraka UltraPOS/src/components/pos/modals/WarehouseTransferModal.tsx`
- [MODIFIED] `D:/Dev/Baraka UltraPOS/src/features/purchases/PurchaseOrderFormScreen.tsx`
- [MODIFIED] `D:/Dev/Baraka UltraPOS/src/components/pos/modals/ReturnModal.tsx`
- [MODIFIED] `D:/Dev/Baraka UltraPOS/src/stores/return-modal-store.ts`
- [CREATED] 3 migration contract test files (transfer/purchase/return)
- [CREATED] `D:/Dev/Baraka UltraPOS/audit/DSN-067/REPORT.md` + 3 screenshots
- Ad-hoc `node_modules/.dsn067-visual.mjs`, `.dsn067-dbg.mjs`, profiles — DELETED.

STATE complete / DONE all 3 pickers + gates + visuals / REMAINING none.
NEXT READY: YES
