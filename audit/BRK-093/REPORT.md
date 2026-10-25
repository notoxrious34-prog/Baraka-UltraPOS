# TASK-BRK-093 REPORT — Catalog exchange round-trip (re-importable + honest limits)

Authority: AUD-043 gaps G-EX1..6 (`919e45c`). Owner-proven failure: export
header `Désignation` rejected by the importer demanding `name_fr`. No
schema change. No new dependencies. Live DB never touched (tempdir
file-DBs + node-only vitest). DO NOT COMMIT.

## 1. Dialect table + chosen direction

Direction: **importer-alias coverage** (no user-visible header renamed,
template untouched). The export keeps its owner-facing FR reporting
headers; the parser learns every export header cell. HANDOFF CLAUSE
satisfied: no blind rename — the alias table below is the proof.

| Export header cell | Normalized | Importer target | Verdict |
| --- | --- | --- | --- |
| `Réf (SKU)` | `réf(sku)` | `sku` | COVERED (aliases `réf(sku)`, `réf`, `référence`) |
| `Code-barres` | `codebarres` | `barcode` | COVERED (added plural; singular was already there) |
| `Désignation FR` | `désignationfr` | `name_fr` | COVERED (was the owner failure: old set had only unaccented `designation`) |
| `Désignation AR` | `désignationar` | `name_ar` | COVERED |
| `Unité` | `unité` | `base_unit_name_fr` | COVERED (new UNIT alias set; was silently dropped) |
| `Prix Achat HT (DZD)` | `prixachatht(dzd)` | `purchase_price` | COVERED |
| `Prix Vente TTC (DZD)` | `prixventettc(dzd)` | `sale_price` | COVERED |
| `Marge (DZD)` / `Marge (%)` | — | — | EXCLUDED (derived; recomputed from prices on every export) |
| `Stock (DEP-01)` / `Stock (MAG-02)` | `stock(...)` | `initial_stock` | COVERED by `stock(...)` pattern (G-EX4: header now names the picked depot) |
| `Statut` | — | — | EXCLUDED (operational state; imports always create active products) |
| (no export column) | — | `category_code` | EXCLUDED (export carries no category column; re-imports land uncategorized) |

Money stays Latin-decimal strings; quantities stay fixed-point 10⁴
strings; BOM + `;` + Arabic preserved (fixtures include Arabic names).

## 2. Honesty rules + pins

- **Over-limit**: `ImportProductsUseCase` rejects batches over
  `MAX_IMPORT_ROWS` with `IMPORT_ROW_LIMIT_EXCEEDED` BEFORE any write
  (store sees zero rows — pin asserts empty). Rule: reject, never silent
  partial. Old clamping test rewritten to the reject rule.
- **Malformed purchase/stock**: parser emits `invalid purchase_price` /
  `invalid initial_stock` row errors (rows skipped); engine additionally
  rejects negative values with `NEGATIVE_PURCHASE_PRICE` /
  `NEGATIVE_INITIAL_STOCK` (defense in depth for direct IPC callers).
- **Barcode collisions**: `BARCODE_ALREADY_ASSIGNED: <code>` row error +
  row skipped — never silently cleared (create and update paths; self
  barcode restated on update is a keeper).
- **Unknown categories**: with a lookup, `unknown category "<code>"` row
  error + skip; engine rejects unknown ids with `UNKNOWN_CATEGORY: <id>`
  (map-less callers must supply real ids — documented).
- **Upsert**: new `Update` policy wired end-to-end (modal selector
  `skip|update`, default `skip`; IPC `"update"`; engine overwrites names,
  known category (absent keeps the link), base-unit prices/names, top-up
  stock via audited movement; `updated_count` in report + DTO + UI badge).
  `Error` policy retained in the engine (abort path unchanged).
- **Export truncation**: `EXPORT_LIMIT+1` fetch; overflow slices to the
  limit and the note carries `settings.exchangeTruncated`.

## 3. Round-trip proof lines

`catalog-roundtrip.test.ts` (7 tests): dialect pin feeds the exact
owner-failing header row → 1 valid row, 0 errors (sku/barcode/names/unit/
prices/stock asserted); `Stock (MAG-02)` pattern pin; covered-fields
field-diff empty on export→parse (category explicitly `undefined`);
**byte-identical** re-export (`toBe(exported)`), Arabic intact.
Rust `catalog_import_engine.rs` (+6 tests): update overwrite + top-up,
category keep/reject, barcode loud, category loud, negative purchase/stock,
over-limit reject with zero writes.

## 4. Gates

| Gate | Result |
| ---- | ------ |
| G1 `cargo fmt --check` | CLEAN |
| G2 `cargo clippy --all-targets --all-features -- -D warnings` | 0 errors, GREEN |
| G3 `cargo test --workspace -j 2` | GREEN, exit 0 — **537 passed, 0 failed, 7 ignored** (70 suites; 531 baseline + 6 new exchange pins) |
| G4 `npm run lint` | exit 0, GREEN |
| G5 `npx tsc --noEmit` | clean, GREEN |
| G7 `npx vitest run` | 819 passed, 0 failed (81 files) |

Two pre-existing tests pinned the old silent behavior and were updated
to the ordered-loud rules (not regressions): `starter-pack.test.ts`
(malformed purchase now dropped, was silent-0) and
`ProductImportModal.test.tsx` (selector pin `error` → `update`).

## 5. Handoff

Template unchanged (canonical headers). Engine sealed; DSN-EX owns UI
polish + template copy. Round-trip holds for active products (Statut
exclusion) and uncategorized re-imports (category exclusion).

NEXT READY: YES (after G3 counts land).
