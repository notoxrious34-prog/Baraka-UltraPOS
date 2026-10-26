# TASK-BRK-095 REPORT — Depot stock-presence read wire

STATUS: **DONE**. DO NOT COMMIT.

Option (a) authorized on DSN-096 REPORT §2: one new read-only IPC command
returning per-depot stock presence. Zero delete-path changes, zero
writes, zero migrations (read-only over `warehouses` + `stock_levels`).

## 1. Query + gate lines

Shape: `[{warehouse_id, nonzero_levels}]` — one row per ACTIVE depot,
code-ordered, zero-count depots included; UI derives `stocked` as
`nonzero_levels > 0`. DEP-01 reported truthfully (lock badge stays UI).

Single query (`sqlite_warehouse_repository.rs`, one prepared statement,
no per-depot loop):

```sql
SELECT w.id, COUNT(s.warehouse_id)
FROM warehouses w
LEFT JOIN stock_levels s
  ON s.warehouse_id = w.id AND s.quantity_base_micro != 0
WHERE w.is_active = 1
GROUP BY w.id
ORDER BY w.code
```

`EXPLAIN QUERY PLAN` (scratch temp DB, deleted after):

```text
SCAN w USING INDEX sqlite_autoindex_warehouses_1
SEARCH s USING INDEX sqlite_autoindex_stock_levels_1 (warehouse_id=?) LEFT-JOIN
USE TEMP B-TREE FOR ORDER BY
```

PK-indexed join (`stock_levels` PK is `(warehouse_id, product_id)`),
single round-trip. No new IPC beyond this one command.

Gate: `ManageWarehouses` in `WarehouseStockPresenceUseCase`
(`inventory.rs`) — same gate as `DeleteWarehouseUseCase`
(`deletion.rs:262`); `Vendeur` → `Unauthorized` (pinned). Command
`warehouses_stock_presence` (`ipc.rs`, `rename_all snake_case`,
`session_token: String` arg) enforces `require_session` at the boundary;
gate lives in the use case, mirroring `warehouses_delete`.

Wire lines: domain `WarehouseStockPresence` (`inventory/warehouse.rs` +
`mod.rs` export) → port `active_nonzero_level_counts`
(`ports.rs:371`) → `SqliteWarehouseRepository` impl → use case →
`WarehouseStockPresenceDto` / `WarehouseStockPresenceResponseDto`
(`dto.rs`, +16/-0 byte-spliced, no churn) → command → registration
(`src-tauri/src/lib.rs:248`).

## 2. Test lines (`crates/infrastructure/tests/warehouse.rs`, tempdir file DBs)

- `presence_empty_depot_reports_zero` — DEP-01 + empty MAIN → 2 rows,
  all `nonzero_levels == 0`.
- `presence_stocked_depot_reports_counts_code_ordered` — MAIN gets 2
  non-zero rows, EAST 1 zero-qty row (must not count) → `[0, 0, 2]`
  in code order (DEP-01, EAST, MAIN).
- `presence_reports_dep01_stock_and_writes_nothing` — DEP-01 with stock →
  `nonzero_levels == 1` (wire hides nothing); `warehouses` /
  `stock_levels` / `stock_movements` / `audit_log` row counts identical
  before/after (zero-write proof).
- `presence_rejects_vendeur_like_deletion_path` — `Unauthorized`.
- Fixture note: product rows inserted via `open_connection` (not raw
  `Connection::open`) so the `baraka_search_norm` scalar for the 0016
  FTS trigger exists; existing raw-reopen tests untouched.

## 3. G1-G3 exits + counts

| Gate | Result |
| ---- | ------ |
| G1 `cargo fmt --check` | CLEAN |
| G2 `cargo clippy --all-targets --all-features -- -D warnings` | 0 errors |
| G3 `cargo test --workspace -j 2` | EXIT 0 — **70 suites, 545 passed, 0 failed, 7 ignored** (541 baseline + 4 new) |
| G4-G7 | N/A — zero `src/` touched (wire is Rust-only; frontend binding is DSN-096-R's scope) |

No product (non-test) file changed after the green run — tallies stand.

## 4. git status --short

```text
M crates/application/src/ports.rs
M crates/application/src/use_cases/inventory.rs
M crates/commands/src/dto.rs
M crates/commands/src/ipc.rs
M crates/domain/src/inventory/mod.rs
M crates/domain/src/inventory/warehouse.rs
M crates/infrastructure/src/adapters/sqlite_warehouse_repository.rs
M crates/infrastructure/tests/warehouse.rs
M src-tauri/src/lib.rs
?? audit/BRK-095/
```

Diff limited to the presence wire + tests + audit. No migration, no
delete-path edits, no frontend, no new dependencies. Scratch
(`brk095-g3.log`, `scratch_presence_plan.rs`) deleted — verified absent.
Live DB never opened (tempdir file DBs + `EXPLAIN` scratch only).

## 5. STATE / DONE / REMAINING

- STATE: **DONE**. `warehouses_stock_presence` answers pre-delete
  presence truthfully, including DEP-01, with zero writes.
- DONE: domain struct, port method, single-query impl + plan proof,
  gated use case, DTO + command + registration, 4 tests, REPORT.
- REMAINING: none — DSN-096-R unblocked (UI: call on manage-tab render,
  `isBlocked` + guidance naming the depot when count > 0, keep engine
  `WAREHOUSE_HAS_STOCK` as the race backstop).

NEXT READY: YES.
