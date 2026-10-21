# TASK-AUD-036 REPORT — Product-search baseline (LIKE-hybrid vs FTS5)

HEAD note: task states `b57983d`, live HEAD is `b57983d`
(`docs(architecture): persist CTP-SESSION-ARC-024…` — same id, newer
subject line than the task's snapshot). `git status` clean before work.

## 1. Headline: the task premise is STALE — BRK-073 already shipped a
normalized infix hybrid (no FTS5, no name index)

The "~553, 567-574 LIKE PREFIX" claim predates TASK-BRK-073. Live code
(`crates/infrastructure/src/adapters/sqlite_catalog_repository.rs`,
`fn search` :614-756) is a three-stage hybrid:

1. blank query → `ORDER BY sku LIMIT ?` discovery (:624-630);
2. single-token ≥3 chars → exact-barcode fast path on
   `p.barcode`/`u.barcode` (:638-659);
3. else multi-token AND-of-ORs infix search over
   `baraka_search_norm(...) LIKE %…% ESCAPE '\'` on name_fr, name_ar,
   sku, barcode, unit barcodes (:684-699), with relevance
   `ORDER BY CASE` (barcode-exact 0, name-prefix 1, first-token infix 2,
   else 3, tie-break sku) (:726-737). Limit `clamp(1,500)` (:620).

Exact SQL texts: see QUERIES.txt (blank-discovery, barcode-exact ×2,
hybrid template + ranking tail).

Searched columns: `products.name_fr`, `products.name_ar`, `products.sku`,
`products.barcode`, `product_units.barcode`. NOT searched: category names,
supplier names, descriptions, aliases table (none exists — aliases live
in code, see §2).

## 2. Normalization state (all live, domain-pure)

`crates/domain/src/catalog/search_normalization.rs`:
- `normalize_for_search` (:388): lowercase → `fold_accents_and_diacritics`
  (:321: FR accents→ASCII incl. œ/æ/ç/ñ; tashkeel U+064B–0652 + dagger
  alef stripped) → `normalize_arabic` (:372: آأإٱ→ا, ة→ه, ى→ي, ک→ك, ی→ي)
  → whitespace collapse.
- `arabic_tolerant_like_pattern` (repo :81): residual ا/ه/ي → `_`
  single-char wildcards so cashier spelling variants match.
- `ALGERIAN_HARDWARE_DICTIONARY` (74 entries): FR↔AR phonetic aliases both
  directions, e.g. `vis ↔ [فيس, فيص]` (:26-27); `expand_token_variants`
  (:398); `tokenize_query` (:423) ≤5 token groups, AND downstream.
- Registered per-query as deterministic SQLite scalar
  `baraka_search_norm(text)` (:96-116), applied to BOTH column and…
  no: applied to the COLUMN side; the pattern side is normalized in Rust
  before binding — equivalent to normalized-vs-normalized comparison.
- Prefix-only vs infix: INFIX (`%{token}%` :700-702) for every token on
  all five columns; prefix additionally used only for RANKING tier 1
  (`{normalized}%` :720). Prefix-only claim: REFUTED.

## 3. INIT-11 gap table (7 rows)

| # | Requirement | State | Evidence |
|---|---|---|---|
| 1 | FTS5 or equivalent indexed search | GAP: no FTS5, no `CREATE VIRTUAL` anywhere; `baraka_search_norm(col) LIKE %…%` cannot use any b-tree index → full scan per token-group | repo :695-699; `grep -rni fts5\|CREATE VIRTUAL` → zero hits outside tests |
| 2 | AR/FR normalization (diacritics, alef/yaa/taa-marbuta, case, punctuation) | DONE (punctuation NOT folded — only whitespace collapse) | search_normalization.rs :321/:372/:388; repo :81 |
| 3 | Infix/token matching | DONE: infix on all columns, ≤5-token AND, variant OR | repo :684-717; tokenize :423 |
| 4 | Aliases | PARTIAL: 74-entry code dictionary, both directions; no DB table, no admin editing, hardware-only vocabulary | :26-27, :398 |
| 5 | Phonetic-ish French-in-Arabic-letters | DONE for covered terms (e.g. فيس→vis :458-460 test) | dictionary + :444-486 tests |
| 6 | Barcode-exact path | DONE: `=` fast path product+unit barcodes before LIKE | repo :638-659; rank tier 0 |
| 7 | p95 <50ms at 80k products | NOT-MEASURED: no scratch DB in repo (no `*.db` outside target/); only evidence is `hundred_product_search_returns_promptly` (100 rows, prints elapsed, asserts non-empty only, no threshold) | catalog_search_engine.rs :215+ |

Index inventory (migrations 0005/0007/0008/0009/0011/0012): b-tree on
`products.sku`, `products.barcode`, `product_units(product_id)`,
`product_units(barcode)`, `category_id`, `is_active` — NO index on
`name_fr`/`name_ar`/`sku`-normalized expressions. At 80k rows the hybrid
runs ~5 normalized-LIKE full scans per token group + 1-2 EXISTS
subqueries on units → FTS5 (or trigram/expression index) is the correct
BRK target; current path is functionally complete but index-less.

## 4. Frontend path

`src/hooks/use-catalog-search.ts`: `DEBOUNCE_MS = 150` (:14); blank query
→ discovery fetch (first 100 active, `includeInactive=false`) (:52-56 in
hook comment; `ipcSearchCatalog(session.token, query, 100, false)` :56);
POS palette 20 rows; Catalog hub `SEARCH_LIMIT = 500` +
`include_inactive=true` (catalog-hub.test.tsx :510-513). Keystroke →
150ms debounce → `catalog_search` IPC (ipc.rs :930, default limit 20,
`include_inactive` false) → `SearchCatalogUseCase::execute`
(catalog.rs :436+) → repo `search`. Session null → UNAUTHORIZED, no call.

## 5. Pinning tests the BRK task must revisit

- `crates/domain/src/catalog/search_normalization.rs` :437-498 (6 tests:
  accent fold, tashkeel strip, ة/ى/alef unification, both-direction
  dictionary incl. فيس→vis, tokenizer phonetic clauses, 70+ term cover).
- `crates/infrastructure/tests/catalog_search_engine.rs` :128/:144/:162/
  :178/:193/:215 (disordered multi-token, AR-phonetic→FR, accent/case,
  SKU-rank, barcode fast path, 100-row promptness — NO threshold).
- `crates/infrastructure/tests/catalog_search_enhancements.rs` :155/:168+
  (infix-inside-name, multi-token AND).
- `crates/infrastructure/tests/catalog.rs` :246-268
  (`search_and_update_behave`: VIS-8, blank-query, unit-barcode).
- `src/features/catalog/catalog-hub.test.tsx` :510-513 (150ms debounce,
  limit 500, include_inactive true pins).
- `src/features/pos/quick-palette-wiring.test.ts`, `src/lib/ipc-category.test.ts`
  (IPC-shape pins touching the catalog path).

## 6. Worktree

Only `?? audit/AUD-036/` (REPORT.md + QUERIES.txt). No scripts left.
No DB opened/copied (live or otherwise); no code read beyond `grep/sed`.

STATE: complete / DONE: baseline above + 2 files / REMAINING: none.
Next BRK (ADR-101): FTS5 virtual table + triggers keeping normalization
consistent with `normalize_for_search`, alias-table design, 80k-row p95
harness on a scratch DB.
