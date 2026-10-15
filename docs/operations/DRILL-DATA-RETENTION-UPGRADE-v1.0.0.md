# DRILL-DATA-RETENTION-UPGRADE v1.0.0 — Migration Replay, NSIS Upgrade Preservation & Downgrade Guard

Task: TASK-REL-054 · Release line: v1.0.0-rc.6 (commit `b6b5c4c`) · Charter v2.0 §§5, 9.2 · ADR-076
Status: **SIGNED OFF** (see §7) · Live user databases were never touched; all work ran on scratch SQLite files in `%TEMP%`.

## 1. Objective

Prove, with executable evidence rather than assertions on paper, that a merchant upgrading Baraka UltraPOS:

1. loses **zero records down to the centime** when migrations 0001→0015 replay over a pre-populated legacy database;
2. keeps `%APPDATA%/com.baraka.ultrapos` intact across an in-place NSIS reinstall/upgrade;
3. cannot silently corrupt the database when facing an unknown future schema version (downgrade guard probe).

## 2. Methodology

### 2.1 Migration replay test (permanent, in-tree)

File: `crates/infrastructure/tests/migration_replay_and_data_preservation.rs` — 2 tests, both green:

- `migration_replay_preserves_legacy_business_data` (stages 1–3)
- `downgrade_guard_probe_future_version_is_deterministic` (stage 4)

**Stage 1 — legacy simulation.** The test applies embedded migrations 0001–0013 one by one through `rusqlite::Connection::execute_batch`, writing the same `schema_migrations (version, name, checksum, applied_at)` bookkeeping rows the official runner writes (checksums via the public `checksum_of`, so the runner later recognises them as clean). It then seeds realistic business data at the v13 shape:

| Entity | Seeded rows |
|---|---|
| Warehouse | `DEP-01` / Dépôt Principal — the canonical row seeded by migration 0012 itself (reused, never duplicated) |
| User | `admin` (role `ADMIN`, Argon2id-formatted hash placeholder) |
| Categories | 2 (`QUI` Quincaillerie, `ELE` Électricité) |
| Products + base units | 3 (Vis bois 6x60 @ 2 500/1 800, Marteau 500g @ 85 000/62 000, Câble 2.5mm @ 12 000/9 500 centimes sale/purchase) |
| Customers | 2 — `c-01` with opening debt **450 000 centimes** (4 500.00 DZD), `c-02` at zero |
| Customer ledger | 1 append-only `DEBIT` entry of 450 000, running balance 450 000 |
| Stock | 100 base units (1 000 000 micro) of `p-01` in DEP-01 + matching `IN` movement |
| Document | 1 `FINALIZED` BL (`BL-2026-000001`, TTC 5 950) + 1 item (2 units) + 1 cash payment of 5 950 |

Baselines captured pre-upgrade: debt 450 000, ledger net 450 000, stock 1 000 000 micro, 1 document, 2 customers, BL TTC 5 950, full unit-price vector.

**Stage 2 — upgrade.** The official `run_migrations(&mut conn)` brings the file to v15 (applies exactly 0014 + 0015).

**Stage 3 — parity assertions.** 15/15 bookkeeping rows with non-empty checksums byte-identical to `checksum_of(embedded SQL)`; debt exactly 450 000; ledger net exactly 450 000; stock exactly 1 000 000 micro; BL TTC 5 950; row-count parity on documents/document_items/payments/customers/products/stock_movements; `customer_tier = 'retail'` for both legacy customers; all 3 legacy units keep `wholesale_price_centimes IS NULL` with sale/purchase vectors unchanged; `print_jobs.document_id` nullable (`pragma_table_info.notnull = 0`); `pragma_foreign_key_check` returns 0 rows; catalog lookup by SKU returns the seeded French name; the upgraded file also reopens cleanly on a fresh connection and re-passes parity.

**Stage 4 — downgrade probe.** Inserts `(9999, '9999_future_schema.sql', 'fake_sha256_hash_for_probe')` into `schema_migrations`, re-runs `run_migrations`, asserts the deterministic outcome (see §4), then asserts full parity again plus idempotent re-run (16 rows: 15 known + future, no duplication).

### 2.2 NSIS verification (read-only)

Inspected `src-tauri/tauri.conf.json` (`bundle` §). No file outside the two deliverables plus the new test was modified.

### 2.3 Quality gates

Full G1–G7 run, outputs in §6.

## 3. SQL invariants verified (exact)

- `SELECT current_balance_centimes FROM customers WHERE id='c-01'` → `450000` before and after.
- `SELECT SUM(CASE WHEN entry_type='DEBIT' THEN amount_centimes ELSE -amount_centimes END) FROM customer_ledger WHERE customer_id='c-01'` → `450000` before and after.
- `SELECT quantity_micro FROM stock_levels WHERE product_id='p-01'` → `1000000` before and after.
- `SELECT COUNT(*) FROM documents` → `1`; `FROM customers` → `2`; before and after.
- `SELECT COUNT(*), MAX(version) FROM schema_migrations WHERE version BETWEEN 1 AND 15` → `(15, 15)`.
- Per-version `checksum = checksum_of(embedded sql)` for all 15 (a single changed byte — including CRLF/LF — would fail the runner on real databases; the frozen-migration law holds).
- `SELECT customer_tier FROM customers` → `'retail'` for both legacy rows (canonical lowercase domain form).
- `SELECT COUNT(*) FROM product_units WHERE wholesale_price_centimes IS NULL` → `3`.
- `SELECT "notnull" FROM pragma_table_info('print_jobs') WHERE name='document_id'` → `0`.
- `SELECT COUNT(*) FROM pragma_foreign_key_check` → `0`.

## 4. Downgrade Guard probe — observed behavior (pinned)

**Observed:** `run_migrations` returns **`Ok(())`** with the future row present. It iterates only its embedded `MIGRATIONS[1..=15]` list and verifies only those checksums; unknown higher-version rows are **silently ignored** — neither applied, validated, blocked, nor removed — and no data is wiped or rewritten. `MAX(version)` stays `9999`, the row `(9999, '9999_future_schema.sql')` is untouched, full financial parity re-passes, and a second `run_migrations` is still `Ok` with row count stable at 16.

**Consequence for the record:** there is **no hard startup block** on future schema versions in this engine line — forward handling is *ignore-and-continue*, which is deterministic and non-destructive (the property the task requires: "either returning an Err or gracefully handling higher migration numbers without wiping"). Any future requirement for a loud startup refusal would be a new feature (new migration-guard code + its own BRK task), not a defect found here.

## 5. NSIS upgrade-preservation findings

From `src-tauri/tauri.conf.json` (verified read-only, unmodified):

```json
"bundle": {
    "active": true,
    "targets": ["nsis"],
    "windows": {
      "webviewInstallMode": { "type": "offlineInstaller", "silent": true },
      "nsis": { "languages": ["French", "English"], "displayLanguageSelector": false }
    }
}
```

- `bundle.targets` is exactly `["nsis"]`; installer languages French/English, no language selector.
- **No `deleteAppDataOnUninstall` key exists** (absent = Tauri v2 default `false`): the uninstaller never performs recursive deletion of `%APPDATA%/com.baraka.ultrapos` or `%LOCALAPPDATA%/com.baraka.ultrapos`.
- **No uninstaller/installer hooks** (`installerHooks`, custom NSIS template, `uninstall` scripts) exist in `tauri.conf.json` or `src-tauri` config — grep for `deleteAppDataOnUninstall|uninstall` returns zero hits.
- The runtime database path (`baraka.db` inside the Tauri `app_data_dir`, ADR-049) lives outside the installed `Program Files` tree, so an in-place reinstall overwrites program files only; the data directory is not part of the installed payload and is preserved by the NSIS default upgrade path.
- Out of scope as designed: at-rest encryption remains an open owner decision (constitution §10); backup/restore is covered by its own suites (`backup_tests`, `restore_tests`).

## 6. Deviations from the task text (code is the truth — all in the test's module docs)

1. **Migration 0014 is `0014_make_print_jobs_document_id_nullable.sql`, not `0014_fts5_catalog_search.sql`.** No `products_fts` table or FTS5 object exists anywhere in `crates/` or `migrations/` (repo-wide grep confirms; only SQLite build-artifact bindgen mentions FTS5). The FTS-population assertion was replaced by: (a) `products_fts` absence pinned to 0 with a comment, (b) 0014's real effect asserted (`document_id` nullability), (c) catalog readability asserted against the base tables (SKU lookup). No migration was renamed or edited.
2. **Tier default is lowercase `'retail'`, not `'RETAIL'`** (`0015_pricing_tiers.sql` line 2; domain `CustomerTier::as_str` contract). Assertions use `'retail'`.
3. Gate numbering follows the task block (G6 = vitest via node, G7 = build), which differs from the constitution's G6/G7 order; both ran regardless.

No frozen migration (0001–0015), `.gitattributes`, live database, or dependency was touched.

## 7. Sign-off

- Drill executed: 2026-10-15 (Africa/Algiers) on rc.6 line + TASK-REL-054 additions.
- Evidence: `cargo test -p baraka-infrastructure --test migration_replay_and_data_preservation -j 2` → **2 passed, 0 failed**; full gates table in the executor report.
- Invariants held: 15/15 migrations replay cleanly over seeded legacy data; debt 450 000 centimes exact; stock 100 units exact; 1 document intact; downgrade probe deterministic (ignore-and-continue, zero writes to user data); NSIS config preserves AppData by default with no destructive hooks.
- Residual risk honestly stated: the downgrade path ignores (rather than refuses) future schemas — safe today, but a loud-refusal guard would need a dedicated BRK task if the architect wants it.

NEXT READY: YES (pending architect acceptance of the two documented deviations).
