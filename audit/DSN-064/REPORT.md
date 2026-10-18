# TASK-DSN-064 — Row Adjustment Blocker, Instant Search, RTL Isolation: REPORT

## 1. ROOT CAUSE (Issue 1)

`StockModule.openAdjust(row)` (`src/features/stock/StockModule.tsx:129-141`) prefilled
the adjust store with `warehouseId/productId/productName/currentStockMicro` but NO
`unitId`/`unitName` (a `LowStockRow` carries no units array). The store's
`openModal` copied the prefill verbatim, and the modal's gate
(`StockAdjustModal.tsx:97-102`, `canSubmit = … && s.unitId !== null && …`) stayed
false forever — confirm button permanently disabled. The global-button path worked
only because the manual picker (`pickProduct`) resolves the unit from
`product.units` (default-selling → base → first).

## 2. DIFFS

**Fix 1 — store unit resolution** (`src/stores/stock-adjust-modal-store.ts`):
new `resolvePrefillUnit(productId)` (called from `openModal` when prefill has a
product but no unit): `ipcGetProduct` (`catalog_get_product`) → same precedence as
the picker → stale-safe set (rechecks `productId`/`unitId` after await; no-op when
a unit is already set; `CATALOG_PRODUCT_HAS_NO_UNITS` error when empty).
```ts
const unit =
  product.units.find((candidate) => candidate.is_default_selling) ??
  product.units.find((candidate) => candidate.is_base_unit) ??
  product.units[0];
…
if (state.productId !== null && state.unitId === null) {
  void resolvePrefillUnit(state.productId);
}
```
No `StockModule.tsx` change needed — the row already passes every other field.

**Fix 2 — instant picker search** (`StockAdjustModal.tsx`): new 175ms debounced
`useEffect` on `[pickerQuery, session, modalProductId]` with timer cleanup + ticket
guard against stale responses; manual "Search" button + wrapper div REMOVED (Enter
kept as explicit fallback). Variance: the StockModule table bar was ALREADY instant
(`StockOverviewPane.tsx:62-67` client-side `useMemo` filter, TASK-DSN-050, tested) —
no change there; only the modal picker was manual.

**Fix 3 — RTL isolation** (`StockAdjustModal.tsx:454`): transition preview wrapped:
```tsx
<span className="font-mono">
  <bdi dir="ltr" className="tabular-nums">
    {fmtQty(parseMicro(s.currentStockMicro))} → {fmtQty(newStock)}
  </bdi>
</span>
```
(matches the file's own barcode `<bdi dir="ltr">` precedent, line ~326).

**Variances vs task text**: reasons enum is French (`INVENTAIRE`, not `INVENTORY`) —
test uses the real `StockAdjustReason`; new test lives at
`src/features/stock/stock-adjust-row-prefill.test.ts` per scope (store + modal-source
contracts, no DOM lib in repo — follows the mocked-`invoke` store-test precedent).

## 3. FAIL-THEN-PASS (Regression Law)

New file `src/features/stock/stock-adjust-row-prefill.test.ts` (4 tests: unit
resolution from unit-less prefill; qty 500 + INVENTAIRE → submit true with
`unit_id: "u-def"`, `quantity_micro: "5000000"`, `direction: "INCREASE"`;
175ms-debounce source contract; `<bdi dir="ltr" className="tabular-nums">` contract).
- BEFORE fix: `Tests 4 failed (4)` (unitId stays null → submit false; no setTimeout/bdi in modal).
- AFTER fix: `✓ stock-adjust-row-prefill.test.ts (4 tests)`, full suite `604 passed`.

## 4. CRLF PROOF

`grep -c $'\r'` on all touched files before AND after: `0` each
(`StockModule.tsx` untouched; store, modal, new test LF-only). `file` reports
UTF-8/ASCII, no CRLF. Zero drift.

## 5. QUALITY GATES (all green)

1. `cargo fmt --check` → exit 0, no diff.
2. `CARGO_BUILD_JOBS=2 cargo clippy --all-targets --all-features -- -D warnings` → `Finished dev profile`, exit 0.
3. `CARGO_BUILD_JOBS=2 cargo test --workspace -j 2` → `TOTAL passed=463 failed=0` (unchanged — frontend-only).
4. `npm run lint` → exit 0. 5. `npm run typecheck` → exit 0.
6. `npm run build` → `✓ built in 5.22s`, exit 0.
7. `npx vitest run` → `604 passed` (baseline 600 + 4 new), exit 0.

## 6. VISUAL ACCEPTANCE (1280×720, stubbed invoke, CDP)

- `audit/DSN-064/screenshots/dsn064-stock-adjust-row-fr.png` — row "Ajuster" →
  modal prefilled, qty 500, `CONFIRM_FR=ENABLED` (poll on disabled flag).
- `audit/DSN-064/screenshots/dsn064-stock-adjust-row-ar.png` — same in ar-DZ,
  `CONFIRM_AR=ENABLED`, 7× `bdi[dir="ltr"]` in modal; inspected render: preview bar
  reads `-1 → 499` LTR with `فائض / زيادة (+500)`; confirm solid orange (enabled).
- `audit/DSN-064/screenshots/dsn064-stock-search-instant.png` — global adjust →
  typed "Ciment" → `INSTANT_RESULTS=yes` with no Enter/button (175ms debounce path).

## 7. FILES

- [MODIFIED] `D:/Dev/Baraka UltraPOS/src/stores/stock-adjust-modal-store.ts`
- [MODIFIED] `D:/Dev/Baraka UltraPOS/src/components/pos/modals/StockAdjustModal.tsx`
- [CREATED] `D:/Dev/Baraka UltraPOS/src/features/stock/stock-adjust-row-prefill.test.ts`
- [CREATED] `D:/Dev/Baraka UltraPOS/audit/DSN-064/screenshots/dsn064-stock-adjust-row-fr.png`
- [CREATED] `D:/Dev/Baraka UltraPOS/audit/DSN-064/screenshots/dsn064-stock-adjust-row-ar.png`
- [CREATED] `D:/Dev/Baraka UltraPOS/audit/DSN-064/screenshots/dsn064-stock-search-instant.png`
- [CREATED] `D:/Dev/Baraka UltraPOS/audit/DSN-064/REPORT.md`
- Ad-hoc `node_modules/.dsn064-visual.mjs` + chrome profile — DELETED (NO_LEFTOVERS).

STATE complete / DONE all three fixes + tests + gates + visuals / REMAINING none.
NEXT READY: YES
