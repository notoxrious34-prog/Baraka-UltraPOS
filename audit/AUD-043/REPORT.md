# TASK-AUD-043 REPORT — Data-lifecycle audit (backup + catalog exchange + factory reset + warehouse deletion)

Read-only. No code/config/DB writes; no builds; live DB never opened. Tree clean at `7bec129` before/after (verified §7).

## 1. Backup map (current behavior)

| Aspect | Current truth (file:line) |
|---|---|
| Commands | `backups_create` (`crates/commands/src/ipc.rs:2166-2196`, Admin-only via `require_backup_admin` = `SystemSettingsManage`, `ipc.rs:2127`), `backups_list` (`:2198`), `backups_verify` (`:2212`), `backups_restore_precheck` (`:2363`), `backups_restore` (`:2378`, Admin-only, quarantine + rollback) |
| Use cases | `CreateBackupUseCase` / `ListBackupsUseCase` / `VerifyBackupIntegrityUseCase` / `PrecheckRestoreUseCase` / `RestoreBackupUseCase` (`crates/application/src/use_cases/backup.rs:38,145,196,275,309`) |
| Trigger points | **MANUAL only, from `BackupModal`** (`src/components/pos/modals/BackupModal.tsx:451` `triggerManualBackup` → `createBackup(token, "MANUAL")`, `src/lib/ipc.ts:2505`). `SHIFT_CLOSE` / `SCHEDULED` codes exist (`backup.rs` enum, `ipc.rs:2176-2178`, i18n labels) but **no caller passes them** — no cash-close hook, no timer/cron/setInterval anywhere (repo-wide grep negative). Dead codes today. |
| Format | Whole-DB SQLite online backup (`rusqlite::backup::Backup::run_to_completion`, `sqlite_backup_manager.rs:72-93`, shared lock held, WAL-safe) → `baraka_pos_backup_YYYYMMDDTHHMMSS_<8hex>.db` (`backup.rs:33-36`) |
| Destination | `<data_dir>/backups/` (`src-tauri/src/lib.rs:153` `backup_dir: data_dir.join("backups")`); retention **newest 30 SUCCESS** (`ipc.rs:2189`, `backup.rs:127-141` prunes files + marks PRUNED) |
| Included | Everything in the DB file: all data tables + `backups_log`, `audit_log`, `command_log`, `schema_migrations`. Nothing excluded. |
| Excluded (outside DB) | The `backups/` dir itself, `backups/quarantine/` (`sqlite_backup_manager.rs:326`), WebView2/Crashpad sidecars — a restore never brings back pruned files. |
| Restore path | Verified restore exists and is real: precheck report → re-verify internally → quarantine active DB → swap → rollback on failure (`sqlite_backup_manager.rs:326-347`, `backup.rs:303-334`); audit is the `RESTORE_PERFORMED` event + quarantine copy (`ipc.rs:2378-2383` — nothing written to `command_log` by design). UI: `BackupModal` restore panel + `RestoreRowButton` + `RestoreReportPanel` (`BackupModal.tsx:115-130+`). Restart required after restore (`RestoreOutcome.restart_required`). |
| Verification | Create-time: `PRAGMA quick_check` + FK violations + SHA-256 recorded in `backups_log` (`backup.rs:79-110`); FAILED → file deleted, row marked FAILED, error returned (`:81-96`). Re-verify on demand (`VerifyBackupIntegrityUseCase`, marks CORRUPTED on pragma failure, `:234-236`). |
| Failure surfacing | Engine: wire codes verbatim (`BACKUP_VERIFICATION_FAILED`, `BACKUP_MISSING`, `BACKUP_CHECKSUM_MISMATCH`, `BACKUP_CORRUPTED`, `INVALID_TERMINAL_ID`, `INVALID_TRIGGER_TYPE`, `UNAUTHORIZED`). UI: `s.errorMessage` → `translateBackendError` (`BackupModal.tsx:487-489`) + dedicated restore keys (`restoreErrorI18nKey`); inline note auto-dismisses after 8s (`DataExchangeSettingsTab`-style `flashNote` pattern also used for backup notes). |

### Backup gaps (G-BK)

- **G-BK1 (scheduling absent):** `SCHEDULED` trigger + `backups.triggerScheduled` label exist with zero firing infrastructure. No daily/close-of-day policy. Fix-shape: scheduler (OS-timer or idle-hook) calling `backups_create(SCHEDULED)` + retention already handles rotation; surface last-scheduled status in `BackupModal`.
- **G-BK2 (no auto-backup on cash close):** `SHIFT_CLOSE` dead; the highest-risk moment (money counted, session closed) takes no snapshot. Fix-shape: fire `backups_create(SHIFT_CLOSE)` in the cash-close success path (best-effort: failure must never fail the close — log + banner only).
- **G-BK3 (destination is local-only):** `data_dir/backups` = same disk as the DB. Disk loss kills DB + backups together. Fix-shape: export-copy affordance (copy verified file to USB/path via existing `desktop-download`-style hardened path) + document off-device cadence.
- **G-BK4 (success-note volatility):** confirmation notes auto-clear (8s); no persistent "last verified backup at …" hero in settings. Fix-shape: persistent status line from `backups_list` newest SUCCESS row.
- **G-BK5 (restore drill unowned):** restore is engineered but S1–S4-style interactive restore was never executed on hardware (REL-010 precedent). Fix-shape: owner drill script (precheck → restore → restart → verify) as a BRK task with tempdir rehearsal first.

## 2. Catalog export/import map

### Export (`src/features/catalog/catalog-export.ts`, wired `DataExchangeSettingsTab.tsx:63-96`)

- Trigger: settings card → `ipcSearchCatalog("", EXPORT_LIMIT=1000)` + per-product `ipcGetStock` on the **picked depot** (`pickDepotId`: DEP-01 else first active, `ProductCreateModal.tsx:235-246`) → `buildCatalogCsvContent(rows, levels, locale)` → `downloadTextFile` (REL-055 hardened path).
- Format: UTF-8 **BOM** (`\uFEFF`, `catalog-export.ts:19`) so Excel decodes Arabic; `;` separator (French decimal commas safe); every text cell `"`-quoted with `""` escapes; money = Latin-decimal strings (`centimesToDecimalFixed`), margin `%` bare Latin number for Excel; stock = base-unit micros→decimal; status localized (`Actif/Inactif`, `نشط/غير نشط`, `:66-71`); header is a **fixed French display row** (`:23-24`).
- Field coverage: sku, barcode, name_fr, name_ar, base-unit name_fr only, purchase, sale, computed margin + %, **one depot's** stock, status. NOT exported: category, secondary units + their barcodes/prices, kit composition, pricing tiers, product id, timestamps, origin.

### Import (`catalog-import-parser.ts` + `ProductImportModal.tsx` + `ImportProductsUseCase` + `import_batch`)

- Template: `sku;barcode;name_fr;name_ar;purchase_price;sale_price;initial_stock;category_code` (`:24-25`), sample rows with Arabic (`:309-317`); parser: BOM-tolerant, `;`-first with `,` fallback, quote-aware, CRLF/LF/CR (`:70-139`); header alias sets incl. Arabic (`الاسم`, `التعيين`, `:57-67`); money/qty via pure string arithmetic to centimes/micros (`:147-201`, strips spaces + zero-width chars, round-half-up, `MAX_SAFE_INTEGER` cap).
- Validation: `validate_import_row` (`catalog.rs:809-835`) — name_fr non-blank, sale ≥ 0; row index preserved; per-row `ImportRowError` (never throws on content).
- Duplicates/idempotency: engine `import_rows_in_transaction` (`sqlite_catalog_repository.rs:1224+`): missing SKU → deterministic generated candidate loop against DB + in-batch set; SKU hit → policy `Skip` (count + report entry) or `Error` (whole-batch rollback, `Conflict("SKU already exists: …")`); within-batch duplicates detected via `seen_skus`/`seen_barcodes` sets. **Barcode collision → silently cleared** (`barcode = None`, row still imports). Unknown category → detached (imports uncategorized). UI policy selector skip/error (`ProductImportModal.tsx:210,422-429`, default skip); warehouse = backend default (UI passes `undefined`, `ipc.ts:337-347` → DEP-01 else first active).
- Limits: `MAX_IMPORT_ROWS = 1000` (`catalog.rs:729`); UI shows first 5 preview rows + first 10 parse errors; report card shows imported/skipped/errored + duration (`ProductImportModal.tsx:161-191`).

### Exchange gaps (G-EX)

- **G-EX1 (export is NOT re-importable):** export header dialect (`Réf (SKU);Code-barres;Désignation FR;…`) vs import aliases — `Désignation FR` normalizes to `désignationfr`, which matches **no** `NAME_FR` alias → whole file rejected with `missing name_fr column`. Plus localized status cells, margin columns, single-depot stock. Fix-shape (pick one, no both): (a) import-dialect export variant (canonical header + raw centimes/micros + category_code + all units), or (b) document export as reporting-only and keep the template as the sole import contract.
- **G-EX2 (silent 1000-row truncation, both directions):** export `EXPORT_LIMIT=1000` with no "truncated" note; import `take(MAX_IMPORT_ROWS)` drops rows beyond 1000 **without a report entry** (`total_rows` counts only kept + early errors). Fix-shape: `TRUNCATED_AT_LIMIT` report entry + UI banner on both sides.
- **G-EX3 (malformed purchase/stock silently zeroed):** invalid `purchase_price` → `None` → stored **0**; invalid/negative `initial_stock` → `None` → **0** (`catalog-import-parser.ts:283-303`). Only `sale_price` is strict. Fix-shape: reject with `invalid purchase_price` / `invalid initial_stock` like sale.
- **G-EX4 (static `Stock (DEP-01)` header can lie):** header hardcodes DEP-01 while levels come from `pickDepotId` fallback (first active) when DEP-01 is absent/deactivated. Fix-shape: interpolate the actual depot code into the header.
- **G-EX5 (no upsert):** Skip/Error only — price/stock updates for existing SKUs require manual edit or delete+reimport. Fix-shape: third policy `Update` (price/name/category overwrite + optional stock top-up), scoped to Gerant/Admin with per-row `UPDATED` report entries.
- **G-EX6 (UTF-8 Excel-write caveat):** BOM covers Excel; Arabic shaping is fine, but RTL Excel users see Latin digits (by design — Latin digits required). No fix; document.

## 3. Factory reset (NOT implemented — zero hits)

Repo-wide `factory|FactoryReset` search returns only `//! SQLite connection factory` doc comments (`connection.rs:1`, test file header). **No use case, no IPC command, no UI, no DTO.** The table below + safety spec is the dev shape.

### Reset domain table (KEEP vs WIPE + rationale)

| Domain (tables / files) | Verdict | Rationale |
|---|---|---|
| `schema_migrations` | **KEEP (never touch)** | Reset is data deletion + reseed, not migration replay; wiping it bricks startup bookkeeping. |
| `users` | **WIPE + reseed principal admin** | Factory = first-run state; existing `check_first_run_status` flow already handles zero-user bootstrap. Reseed exactly one ADMIN via the same path as first install (no hardcoded password in code — reuse setup flow). |
| Catalog: `products`, `product_units`, `product_kit_items`, `categories` | **WIPE** | Merchant data. No reseed except starter taxonomy **only if** first-run currently seeds it (else empty catalog). |
| Stock: `stock_levels`, `stock_movements` | **WIPE** | Derived/operational; meaningless without catalog. |
| Warehouses | **WIPE + reseed DEP-01** | Invariant (`PRINCIPAL_DEPOT_CODE`, `0012_seed_default_warehouse`): engine assumes DEP-01 exists. Reset must recreate it (active, default name). |
| Documents: `documents`, `document_items`, `payments`, `document_sequences`, `tax_rules` | **WIPE + reseed sequences/rules** | Fiscal history must go; sequences restart at seed and tax defaults re-insert (mirror the seed migrations, not the migration files themselves). |
| Cash: `cash_sessions`, `cash_movements` | **WIPE (precondition: no OPEN/ACTIVE session)** | Refuse reset while a session is open — close first (prevents money loss mid-count). |
| Customers + `customer_ledger`, suppliers + `supplier_ledger` (+ purchase docs if any) | **WIPE** | Merchant counterparties + balances. Precondition: zero open balances equally acceptable — simpler: wipe unconditionally (it IS a factory reset) but surface the written-off totals in the pre-reset report. |
| Settings KV (`settings`: printer, fiscal, paper, render mode, transport…) | **WIPE to first-run defaults** | Owner reconfigures; defaults must equal the first-run seed (COM1/9600/80/serial/escpos etc.). |
| Store identity (`store.*` keys) | **WIPE (factory blank)** | Identity is owner-specific. |
| Outbox `print_jobs` | **WIPE** | Pending jobs reference wiped documents; engine reprint of them would be nonsense. Precondition alternative: refuse while PENDING jobs exist — spec picks WIPE + surfaces the cancelled count (simpler, reset semantics). |
| `backups_log` + `backups/*.db` + `backups/quarantine/` | **KEEP (mandatory)** | The safety net must survive the reset; the mandatory pre-reset backup (below) is worthless if its file/row is deleted. Prune nothing during reset. |
| `audit_log`, `command_log` | **WIPE + write `RESET_PERFORMED` as first row of the fresh DB** | History goes, but the reset event itself (actor, timestamp, pre-reset backup id, written-off totals) must exist in the new DB — otherwise the reset is unauditable. |
| FTS index (`fts_*` from `0016`) | **Rebuild/empty via existing triggers** | Verify post-reset: `fts` tables empty and triggers intact (spec: assert in the reset test). |

### Reset safety spec (shape only)

- Role gate: `SystemSettingsManage` (mirrors `require_backup_admin`, `ipc.rs:2127`) — same grant as backup/restore. No new permission needed.
- Mandatory backup-before-reset, enforced engine-side: `CreateBackupUseCase` (MANUAL or new `FACTORY_RESET` trigger code) must complete VERIFIED (quick_check ok + SHA recorded) **in the same operation**; wipe runs only after; the backup id is embedded in the `RESET_PERFORMED` row. A pre-existing backup is not sufficient (stale).
- Double-confirmation UI: reuse `SafeDeleteModal` typed-token pattern (`SafeDeleteModal.tsx:94-100`, token = entity code) with token = store name or depot code + reason field (min length mirrors `MIN_REASON_LENGTH`) + explicit consequences list (what is wiped, what survives). Two-step: arm → type → hold-to-confirm is acceptable; single click is not.
- Preconditions (hard refuses): active cash session open; backup failed/unverified; `backups/` dir not writable. Soft warnings (surfaced, not blocking): nonzero customer/supplier balances with totals, pending print jobs count, unpruned old backups count.
- Execution: one `BEGIN IMMEDIATE` transaction (deletes in FK-safe order + reseeds + `RESET_PERFORMED` audit row), then `restart_required` (same contract as restore) so in-memory state cannot diverge.
- Tests (dev task): tempdir-only reset test — seed all domains → reset → assert KEEP/WIPE table row-by-row + `RESET_PERFORMED` present + DEP-01 + admin bootstrap alive + backups dir intact.

## 4. Warehouse deletion — verdict vs AUD-039: CONTRADICTION RESOLVED (downgraded to minor UI remnant)

AUD-039 called warehouses **INCONSISTENT** (engine guarded, UI offered DEP-01 delete then failed post-confirm, no reason/audit collection). Current code:

- Engine (`DeleteWarehouseUseCase::execute`, `crates/application/src/use_cases/deletion.rs:253-295`): `ManageWarehouses` perm → DEP-01 → `DEPOT_PRINCIPAL_IMMUTABLE` (`:268-270`); nonzero levels → `WAREHOUSE_HAS_STOCK` (`:271-273`, counts `quantity_base_micro != 0`, `sqlite_warehouse_repository.rs:195-204`); refs (stock_movements + documents counts, `:175-193`) → `deactivated` + audit; else `check_purge_reason(reason)` + `hard_delete_warehouse` in-tx (deletes zero-levels + row + `PURGE` audit row, `:207-224`). **Reason is now collected and audited** (`audit_ctx`, `DeleteAudit` plumbed through set_active + hard delete) — AUD-039 G1 fixed here. Suppliers likewise gained `SUPPLIER_BALANCE_NONZERO` (G4 fixed).
- UI (`WarehouseTransferModal.tsx:95-234`): DEP-01 shows lock badge (`:105-119`) and **no delete button at all** (`{!locked && …}`, `:163-193`); the modal still receives `isBlocked={deleting.code === PRINCIPAL_DEPOT_CODE}` (`:213`) as unreachable defense-in-depth; typed token = entity code + reason min-length enforced in `SafeDeleteModal` (`SafeDeleteModal.tsx:28,94-100`) — AUD-039 G2/G3-fixed for DEP-01.
- **Remnant:** stocked non-DEP-01 depots still show an active delete button with **no stock pre-check** — click → confirm → type → engine refuses `WAREHOUSE_HAS_STOCK` post-confirm (`WarehouseTransferModal` has no levels query; only `s.errorMessage` surfaces it, `:199-203`). Offered-then-rejected survives for this one case.

Verdict: the "contradiction" (UI offers what engine forbids **for DEP-01**) is **gone** — DEP-01 cannot reach the modal. What remains is a minor UX gap (stocked depots), not a safety contradiction: the engine was and is the safe backstop, and it now audits. Fix-shape (S): pre-check `count_warehouse_nonzero_levels`-equivalent via existing levels query before enabling the delete button → `isBlocked` + `blockedReason` (the exact Customers pattern AUD-039 praised: `blocked = hasOpenBalance(...)` + guidance). No engine change needed.

## 5. Cross-area fix/dev specs (for BRK/DSN follow-ups)

1. **BRK-BK:** wire `SHIFT_CLOSE` into cash-close success (best-effort) + add idle/daily `SCHEDULED` trigger + persistent last-backup status line + USB export-copy affordance. Owner drill: precheck → restore → restart → verify.
2. **BRK-EX:** `TRUNCATED_AT_LIMIT` report entries both directions; strict purchase/stock validation; dynamic depot-code export header; `Update` import policy (scoped, reported).
3. **DSN-EX:** import-dialect export variant (or reporting-only label); persistent backup status hero; reset double-confirm screen reusing `SafeDeleteModal` token pattern; depot delete `isBlocked` pre-check.
4. **BRK-RST (new):** factory-reset use case + IPC + UI per §3 (transactional wipe + reseed + `RESET_PERFORMED` + restart contract + tempdir test asserting the KEEP/WIPE table).

## 6. UNPROVEN / runtime probes

- None blocking: all verdicts above are line-proven. Two owner-hardware probes remain for the dev tasks, not this audit: (P1) restore drill on real hardware incl. restart behavior; (P2) 1000+-product import timing on the owner machine (clamp UX under real Excel files).

## 7. git status --short

```
?? audit/AUD-043/
```

Nothing modified. No commit.

STATE: complete / DONE: REPORT.md / REMAINING: none (fix/dev tasks branch from §5).

NEXT READY: YES
