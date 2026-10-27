# TASK-BRK-096 REPORT — Per-unit line discount semantics (owner-locked)

Status: DONE. No commit (per task order).

## 1. Kernel + bound lines (confirm unchanged intent, new semantics)

- Bound: `crates/domain/src/errors.rs:72` — `LineDiscountExceedsUnitPrice`
  ("line discount exceeds unit price").
- Kernel gate: `crates/domain/src/sales/pricing.rs:320` —
  `return Err(DomainError::LineDiscountExceedsUnitPrice)` when
  `discount_per_unit > unit_price`.
- New field: `pricing.rs:44` — `PricedLine.discounted_unit_price_centimes: i64`.
- Kernel loop (`pricing.rs`, per-unit block): exact subtraction
  `discounted = unit_price - discount` (micros/i64, `checked_sub`), line total
  = `price_line_gross(qty, discounted PU)` — qty multiplication only, single
  canonical rounding. No division in discount math.
- Global allocation unchanged in meaning: flat total, largest-remainder over
  the after-discount line amounts (`allocate_global_discount`, bases are now
  the per-unit-discounted lines — same mechanism, same code path).

## 2. Test lines + pins

- Owner example pinned, kernel (`pricing.rs` unit tests):
  `per_unit_owner_example_three_units_off_100` — 3 × (1,000 − 100) →
  `discounted_unit_price_centimes = 900`, gross 3,000, after 2,700.
  Free-line case: 2 × (1,000 − 1,000) → PU′ 0, total 0.
  Bound rejections: discount > unit price → `LineDiscountExceedsUnitPrice`.
- `crates/infrastructure/tests/sale_pricing.rs::priced_document_invariants_hold`:
  3.0 × (12,000 − 1,000) = 33,000 + 1.5 × 8,000 = 12,000; global 2,500 over
  bases [33,000, 12,000] → allocated [1,833, 667]; asserts PU′ [11,000, 8,000],
  after-discount [33,000, 12,000], shares [1,833, 667].
- Preview/commit parity (`sale_preview_commit_parity.rs`): line A
  2 × (10,000 − 1,000) = 18,000; line B 1 × 5,050; global 4,050 over bases
  [18,000, 5,050] → shares [3,163, 887]; HT = 14,837 + 4,163 = 19,000.
  Stored-line pin: `[(10_000, Some(1_000), 5_163, 14_837),
  (5_050, Some(0), 887, 4_163)]` — per-unit persisted, lump = `gross − after`
  + allocated share exactly.
- Draft parity (`sale_atomic.rs::draft_parks_per_unit_discount_exact`): draft
  3 × (10,000 − 1,000) parks Some(1,000), lump 3,000, HT/TTC 27,000, no stock
  move, no payment.
- Partial-return exactness
  (`sales_return_tests.rs::partial_return_with_per_unit_discount_is_exact`):
  return 1 of 3 units → refund exactly 9,000 (round(1 × 9,000), no division);
  avoir line stores (Some(1,000), lump 1,000, HT 9,000); drawer −9,000; stock
  +1 unit.
- Legacy-reprint parity: `migration_replay_and_data_preservation.rs` asserts
  0018 adds the column AND every legacy line reads back
  `discount_per_unit_centimes IS NULL` (count = baseline item count); all
  pre-existing `None`-fixture tests (returns, conversion, reporting, backup,
  restore) pass unchanged — legacy display keeps the transcribed lump,
  legacy returns keep the lump pro-rating.

## 3. Zero-division proof

`grep -rn "/ qty" crates/domain/src/sales/pricing.rs
crates/application/src/use_cases/sales.rs
crates/application/src/use_cases/returns.rs` → no matches (exit 1).
Remaining divisions in those files are pre-existing and out of scope:
canonical qty scaling `/10_000` inside `checked_mul_qty`, tier-margin
formula, largest-remainder allocation, timbre stepping, and the
legacy-only `mul_div_half_up` pro-rating helper.

## 4. Gates G1–G3 (G4–G7 N/A — backend-only, no shared surface touched)

- G1 `cargo fmt --check`: clean (exit 0).
- G2 `cargo clippy --all-targets --all-features -- -D warnings`: clean (exit 0).
- G3 `cargo test --workspace -j 2`: 70 suites, 549 passed, 0 failed,
  7 ignored (pre-existing ignores: fts + spooler suites).
  - Notable: `restore_older_schema_backup_is_migrated` initially failed
    ("duplicate column name") after registering 0018; fixed in-test by
    adding the `app_version >= 18` strip block (drops the new column from
    the fabricated older backup, mirroring the v15/v16/v17 blocks).
    Re-run: restore_tests 14/14.

## 5. git status --short

```text
M crates/application/src/ports.rs
M crates/application/src/use_cases/returns.rs
M crates/application/src/use_cases/sales.rs
M crates/application/tests/returns_use_case_tests.rs
M crates/commands/src/dto.rs
M crates/commands/src/ipc.rs
M crates/domain/src/errors.rs
M crates/domain/src/sales/document.rs
M crates/domain/src/sales/pricing.rs
M crates/infrastructure/src/adapters/sqlite_checkout_repository.rs
M crates/infrastructure/src/adapters/sqlite_conversion_repository.rs
M crates/infrastructure/src/adapters/sqlite_document_repository.rs
M crates/infrastructure/src/adapters/sqlite_return_repository.rs
M crates/infrastructure/src/persistence/migration.rs
M crates/infrastructure/tests/backup_tests.rs
M crates/infrastructure/tests/bl_to_facture_conversion.rs
M crates/infrastructure/tests/details_names_tests.rs
M crates/infrastructure/tests/devis_to_bl_conversion.rs
M crates/infrastructure/tests/document_search_tests.rs
M crates/infrastructure/tests/fts_migration_tests.rs
M crates/infrastructure/tests/migration_replay_and_data_preservation.rs
M crates/infrastructure/tests/restore_tests.rs
M crates/infrastructure/tests/returns_persistence_tests.rs
M crates/infrastructure/tests/sale_atomic.rs
M crates/infrastructure/tests/sale_preview_commit_parity.rs
M crates/infrastructure/tests/sale_pricing.rs
M crates/infrastructure/tests/sales_return_tests.rs
?? audit/BRK-096/
?? crates/infrastructure/migrations/0018_line_discount_per_unit.sql
```
Diff: 27 files, 481 insertions(+), 112 deletions(-). Nothing committed.

## 6. STATE / DONE / REMAINING

STATE: DONE. REMAINING: none for BRK-096. Next: DSN-097 owns modal/cart/receipt
display (wire + stored values already in place).
