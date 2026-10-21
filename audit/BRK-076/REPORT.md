# TASK-BRK-076 REPORT — Migration 0016: FTS5 apparatus (no query-path change)

## 1. Step-0 proof (run BEFORE any schema was written)
- `SELECT sqlite_compileoption_used('ENABLE_FTS5')` = **1** (`step0_fts5_available_in_bundled_sqlite`, green).
- Bundled SQLite = **3.46.0**: `unicode61 "remove_diacritics N"` (needs 3.51+) is a **parse error** (proven by throwaway probe, deleted per NO_LEFTOVERS). Design delta vs ADR-101: FTS table uses plain `tokenize='unicode61'` — equivalent here because every indexed token is pre-normalized (accents/diacritics stripped, Arabic unified) by `baraka_search_norm` at write time; unicode61 retains ASCII case-folding.

## 2. Files (DO NOT COMMIT)
- `crates/infrastructure/migrations/0016_fts_search_index.sql` [CREATED, ~9.5kB]: `search_norm_fr/ar` columns; `product_aliases(product_id, alias_norm, UNIQUE, FK CASCADE)` + index; `products_fts(product_id UNINDEXED, 5 norm cols, unicode61)` content-ful; 8 triggers (products AI/AD/AU, units AI/AU/AD, aliases AI/AD — DELETE + re-INSERT refresh from relational truth); in-tx norm backfill UPDATE (fires AU → FTS converges in one pass); alias seeding by runner hook.
- `crates/infrastructure/src/persistence/migration.rs` [MODIFIED +~50]: v16 registration (0015 pattern, checksum mechanism unchanged) + `seed_fts_aliases` in the v16 tx (per-product token → `expand_token_variants`, INSERT OR IGNORE; same domain fn as query-time expansion).
- `crates/infrastructure/src/persistence/connection.rs` [MODIFIED +27]: `register_search_norm` in `apply_pragmas` (every connection: file + memory + migration). Idempotent (re-registration replaces binding). Zero query-path change.
- `crates/infrastructure/src/adapters/sqlite_catalog_repository.rs` [MODIFIED +~158]: `verify_fts_consistency()` only (count + every-64th stride, sorted token-set hash recomputed from raw columns; `Port` error names the product). Search SQL byte-identical.
- `crates/infrastructure/tests/fts_migration_tests.rs` [CREATED]: 9 tests (8 fast + 1 ignored 80k): step-0, fresh apply + checksum, upgrade 1-15→16, preservation (counts + spot norms + فيس alias + MATCH probes), RED pre-backfill (row-count FAIL), RED stale-norm (content FAIL), GREEN consistency, trigger mirror (insert/update/delete/unit/alias), ignored 80k clock.
- `crates/infrastructure/tests/migration.rs` [MODIFIED +3]: two hardcoded `15` counts → `MIGRATIONS.len()` (FAIL-before quoted §5, PASS-after 11/11).
- `crates/infrastructure/tests/backup_tests.rs` [MODIFIED +5]: FK-corruption test opens COPY via `open_connection` (raw `Connection::open` lacks the scalar → v16 unit trigger errored; test intent preserved, now 6/6).
- 0001…0015 bytes untouched; `.gitattributes` untouched; query path untouched.

## 3. Design deltas vs ADR-101 (reasons)
1. plain `unicode61` (see §1).
2. `content=''` REJECTED by probe: in FTS5 it means contentless — stored columns read back NULL and `DELETE ... WHERE product_id = ?` matches nothing, so the mirror silently accumulated duplicates (2 products → 10 FTS rows in probe). Default content-ful table: triggers work as written.
3. TEXT-UUID PK → `product_id UNINDEXED` key column (no rowid sync possible).
4. No covering barcode index added: `idx_products_barcode` + `idx_product_units_barcode` already exist since 0001 (verified live).
5. `verify_fts_consistency` recomputes expected values from RAW columns, never stored norms — valid for rows written before the BRK-077 repo norm writes.

## 4. Red-before-green evidence (regression law)
- `consistency_fails_on_pre_backfill_state`: `DELETE FROM products_fts` → `Err(Port("fts row-count mismatch: products=2 fts=0"))` — FAIL as required, then `consistency_passes_after_migration` Ok.
- `consistency_fails_on_stale_norm`: `UPDATE products_fts SET norm_fr='zzz-stale'` → Err (needed 65 seeded rows: stride samples every 64th; a 2-row DB would hide corruption in the unsampled gap — documented in test).
- `backup_tests::foreign_key_violations_mark_backup_corrupted`: FAILED first with `no such function: baraka_search_norm` (raw opener) → PASS after sanctioned-opener fix. Lesson recorded: every future writable raw open must use `open_connection`.
- Harness counts: FAIL-before `left: 16, right: 15` (both tests) → PASS-after 11/11.

## 5. Backfill wall time (80k, same i5-3450/16GB/SATA/debug box as BRK-075)
- PENDING — `backfill_80k_wall_clock --ignored` running in background at REPORT draft time.

## 6. Gates
- G1 `cargo fmt --check`: EXIT 0.
- G2 `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- G3 `cargo test --workspace -j 2`: PENDING final re-run (last full run had the 2 harness failures, now fixed; migration 11/11 + fts 8/9 + backup 6/6 re-verified individually).
- G4-G7 N/A: zero frontend touch (Rust migration + tests + markdown only).

## 7. ADR-102 candidate (flagged for Architect, NOT decided here)
Custom scalar required on EVERY writable connection: any future raw `Connection::open` writer touching products/units/aliases will fail loudly (fail-closed, good) but the failure mode is non-obvious. Options: (a) lint/deny raw opens, (b) move norm computation out of triggers (BRK-077 repo writes + trigger uses stored cols only), (c) accept + document.
