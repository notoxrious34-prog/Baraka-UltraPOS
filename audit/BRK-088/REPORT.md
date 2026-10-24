# TASK-BRK-088 REPORT — Document-details product designation (read path)

Wire fix for AUD-041 verdict B. LIVE join for display (Architect-set);
finalized docs stay ID-immutable. No schema change (JOIN over existing
columns). No production changes in C1 (fixtures only).

## (1) FAIL-before (quoted, red on old wire)

```
error[E0599]: no method named `get_items_with_names` found for struct `SqliteDocumentRepository`
error[E0599]: no method named `get_items_with_names` found for struct `SqliteDocumentRepository`
error: could not compile `baraka-infrastructure` (test "details_names_tests") due to 2 previous errors
```

## (2) Implementation (JOIN + DTO one-liners)

- `crates/infrastructure/src/adapters/sqlite_document_repository.rs`:
  `get_items_with_names` — one statement, explicit 14-col `di.` list
  (NOT `di.*`: `document_items` has 16 physical columns incl.
  `source_item_id`, `tax_exemption_code`; caught honest FAIL at runtime,
  fixed) + `LEFT JOIN products p ON p.id = di.product_id`
  + `LEFT JOIN product_units u ON u.id = di.unit_id`,
  `WHERE di.document_id = ?1 ORDER BY di.sort_order`;
  `map_item_view` reuses `map_item` (idx 0–13) + idx 14–18 nullable,
  `product_missing = name_fr.is_none()`.
- `crates/domain/src/sales/document.rs` (+ re-export `sales/mod.rs`):
  `DocumentItemView { item: DocumentItem, product_name_fr/ar,
  sku, unit_name_fr/ar: Option<String>, product_missing: bool }` —
  frozen item untouched, display fields only.
- `crates/application/src/ports.rs` + `use_cases/sales.rs` (required
  companions, cited): port method `get_items_with_names`;
  `SaleDetail.items_view` alongside `items` (same order); same permission,
  no logic change. Fake in `returns_use_case_tests.rs` → `Ok(vec![])`.
- `crates/commands/src/dto.rs` + `ipc.rs` (`sales_get`): `DocumentItemDto`
  +6 nullable + `product_missing`; map iterates `detail.items_view`.
  (Line-ending repair: dto.rs tail restored to LF, diff = +13 only.)
- `src/lib/ipc.ts`: `DocumentDetailsWireResponse.items` +6
  `string | null` + `product_missing: boolean` (typed-contract sync).
- BRK-080 no-conflict: `deletion.rs:79` (purge only when unreferenced,
  else deactivate) + `document_items.product_id REFERENCES products(id)`
  (0001) — live orphan impossible; fallback covers legacy rows only.

## (3) Tests (`crates/infrastructure/tests/details_names_tests.rs`)

```
test details_items_carry_designation_in_both_locales ... ok
test details_missing_product_degrades_gracefully ... ok
test result: ok. 2 passed; 0 failed
```

2-line sale (FR+AR names, sku, unit labels, one call); orphaned product
(FK-off simulated delete) → 2 rows kept, nulls + `product_missing=true`.

## (4) No-N+1 evidence (temp EXPLAIN probe, deleted after use)

```
SEARCH di USING INDEX idx_document_items_document (document_id=?)
SEARCH p USING INDEX sqlite_autoindex_products_1 (id=?) LEFT-JOIN
SEARCH u USING INDEX sqlite_autoindex_product_units_1 (id=?) LEFT-JOIN
USE TEMP B-TREE FOR ORDER BY
```

One statement; PK lookups are the JOIN mechanism, not extra queries.
`zz_tmp_explain_088.rs` removed (`grep -c zz_tmp` = 0, NO_LEFTOVERS).

## (5) Gate table

| Gate | Command | Result |
|---|---|---|
| G1 | `cargo fmt --check` | CLEAN |
| G2 | `cargo clippy --all-targets --all-features -- -D warnings` | CLEAN (1 honest fix: `assert_eq!(x, true)` → `assert!(x)`) |
| G3 | `cargo test --workspace -j 2` (`audit/BRK-088/g3.log`, G3-EXIT=0) | 65 suites ok, **493 passed / 0 failed** (baseline 64/491 + new file) |
| G4 | `npm run lint` | EXIT 0 (re-run after fixtures) |
| G5 | `npm run typecheck` | EXIT 0 (after fixtures) |
| G6/G7 | full build/vitest | N/A — no build logic touched; the 2 affected Vitest files run directly: 2 files / 15 tests pass |

## (6) C1 fixtures (scope note — disclosed)

Authorized 2 + 1 discovered: the G5 re-run after the first two fixes
surfaced a THIRD stale fixture in the same file
(`document-detail-drawer.test.tsx:43`, `prod-vis` TICKET object) with the
identical shape error. Fixed in-kind (plausible FR/AR names + SKU +
unit labels, `product_missing: false`), test-only, zero production
impact. No other fixtures touched; no rendering changes (DSN owns drawer).

## (7) Diff footprint

[MODIFIED] domain `sales/document.rs` + `sales/mod.rs`; application
`ports.rs`, `use_cases/sales.rs`, `tests/returns_use_case_tests.rs`;
commands `dto.rs`, `ipc.rs`; infrastructure
`adapters/sqlite_document_repository.rs`; `src/lib/ipc.ts`;
2 frontend test fixtures.
[CREATED] `crates/infrastructure/tests/details_names_tests.rs`,
`audit/BRK-088/REPORT.md`, `audit/BRK-088/g3.log`.
No commit. Live DB never opened. No strays.

STATE done. DONE: wire carries designation + fallback; gates green.
REMAINING: DSN drawer render (shows names + archived marker) +
strip/printer-UX work (unblocked by this).
NEXT READY: YES.
