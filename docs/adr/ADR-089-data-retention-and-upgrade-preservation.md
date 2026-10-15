# ADR-089: Data-Retention Invariants, In-Place Upgrade Safety, and Downgrade Policy

Status: ACCEPTED
Date: 2026-10-14
Session: TASK-REL-054 drill (rc.6 line) / TASK-DOC-045 filing

## Context

Charter v2.0 §§5, 9.2 and ADR-076 require proven — not asserted — zero-data-loss
across schema upgrades: a merchant updating Baraka UltraPOS must never suffer
database erasure, credit-ledger desynchronization, or stock-count zeroing.
Migration files 0001–0015 are frozen and byte-checksummed (`schema_migrations`
verification at startup), and the v1.0.0-rc.6 NSIS bundle
(`src-tauri/tauri.conf.json`) is the delivery vehicle. Before the Gold Master,
three properties needed executable proof: (1) migrations 0001→0015 replay over
realistic legacy data with centime-exact parity, (2) in-place NSIS
reinstall/upgrade preserves `%APPDATA%/com.baraka.ultrapos`, (3) a database
carrying an unknown future schema version is handled deterministically.

## Decision

1. **Migration replay parity is proven by permanent test, not by inspection.**
   `crates/infrastructure/tests/migration_replay_and_data_preservation.rs`
   builds a legacy v13 database (migrations 0001–0013 applied with identical
   bookkeeping checksums), seeds realistic business facts (DEP-01 warehouse,
   admin user, 2 categories, 3 products with base units, 2 customers with a
   450,000-centime opening debt + append-only `DEBIT` ledger entry, 100 base
   units + `IN` movement, one `FINALIZED` BL with item + cash payment), runs
   the official `run_migrations` upgrade to v15, and asserts: 15/15
   bookkeeping rows with checksums byte-identical to `checksum_of(embedded
   SQL)`; debt, ledger net, stock micro-count, BL total, and all row counts
   unchanged; `pragma_foreign_key_check` clean. Full drill record:
   `docs/operations/DRILL-DATA-RETENTION-UPGRADE-v1.0.0.md`.
2. **Migration 0015 defaults legacy customers to canonical lowercase
   `'retail'`** (`crates/infrastructure/migrations/0015_pricing_tiers.sql`
   line 2; `CustomerTier::as_str` contract in
   `crates/domain/src/sales/customer.rs`). Legacy `product_units` keep
   `wholesale_price_centimes IS NULL` with sale/purchase prices untouched.
   (The task text's `'RETAIL'` and FTS5/0014 expectations were stale; the
   drill pins the real shapes: 0014 is the `print_jobs.document_id`
   nullability rebuild and no `products_fts` table exists in this schema line.)
3. **NSIS bundle policy: user data is never purged on reinstall or upgrade.**
   `src-tauri/tauri.conf.json` targets exactly `["nsis"]` with no
   `deleteAppDataOnUninstall` key (Tauri v2 default `false`), no installer
   hooks, and no custom NSIS template; the runtime `baraka.db` lives in the
   Tauri `app_data_dir` (ADR-049), outside the installed program-files tree,
   so in-place reinstall overwrites program files only.
4. **Downgrade policy: unknown future versions are ignored, never acted on.**
   Pinned observed behavior (`downgrade_guard_probe_future_version_is_deterministic`):
   with `(9999, '9999_future_schema.sql', …)` present in `schema_migrations`,
   `run_migrations` returns `Ok(())`, performs zero writes, leaves the future
   row untouched, and all parity assertions re-pass. The engine does NOT
   refuse startup on future schemas — forward handling is deterministic
   ignore-and-continue. A loud-refusal guard is deliberately deferred to a
   future BRK task if the architect wants it; it is not a defect found here.

## Consequences

- Upgrade path for merchants is safe by test: ledger, stock, and finalized
  documents survive 0013→0015 replay with 100% centime parity; the drill test
  runs in every `cargo test --workspace` invocation as a permanent guard.
- Migrations 0001–0015 remain strictly frozen; any future schema change is a
  new numbered migration with its own preservation test (constitution §3.7).
- No change to installer configuration, runtime paths, or the migration
  runner in this ADR; at-rest encryption stays an open owner decision.
- Validation gate: none — this ADR records verified behavior, no owner action required.
