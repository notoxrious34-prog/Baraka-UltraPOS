# TASK-BRK-080 REPORT — Audit event + mandatory reason on every delete path; supplier debt-check (G1+G4)

## (0) Step-0 G4 verdict — supplier debt-check: ABSENT → MIRROR ADDED

- Grep `debt|balance` in `crates/application/src/use_cases/deletion.rs` (pre-fix) hit
  ONLY the customer guard (`CUSTOMER_BALANCE_NONZERO`, then `deletion.rs:122-123`).
- Supplier balance accessor existed but was never consulted:
  `SupplierView.current_balance_centimes` (`crates/application/src/ports.rs:1641`).
- `DeleteSupplierUseCase::execute` (now `deletion.rs:~178-196`) checks
  `supplier.current_balance_centimes != 0` and returns `coded("SUPPLIER_BALANCE_NONZERO")`
  — same `coded()` taxonomy style as the customer mirror, no new taxonomy.
- Wire code surfaces as `SUPPLIER_BALANCE_NONZERO` (all-caps token passes
  `stable_code_or` through, cf. `ipc.rs:369-372`); verified by the debt test asserting
  the code string contains `SUPPLIER_BALANCE_NONZERO`.

## (1) FAIL-before (RED, quoted from pre-fix run against the old 2-arg API)

```
test red_supplier_hard_delete_writes_audit_row ... FAILED
test red_warehouse_hard_delete_writes_audit_row ... FAILED
test red_supplier_with_open_debt_is_refused ... FAILED
test red_customer_hard_delete_writes_audit_row ... FAILED
test result: FAILED. 0 passed; 4 failed
```

Premise proven: no delete path wrote `audit_log`; suppliers carried no open-debt guard.

## (2) Wiring summary (engine only, `src/**` untouched)

- `crates/application/src/ports.rs`: new `DeleteAudit { actor, reason, old_json }`;
  `hard_delete_*` / `set_*_active` for **customer/supplier/warehouse** take
  `&DeleteAudit`. Product trait signatures UNCHANGED (BRK-076 owns
  `sqlite_catalog_repository.rs` — no overlap; `git status` confirms untouched by BRK-080).
- `crates/application/src/use_cases/deletion.rs`: all 4 `execute()` take
  `reason: &str`. `check_purge_reason` requires ≥3 chars on PURGE paths.
  NOTE — spec text cited `DomainError::ValidationError`, but that variant does not
  exist in this tree; live convention is
  `DomainError::InvalidCanonicalString("reason must be at least 3 characters")`,
  mapped to `INVALID_INPUT` at `ipc.rs:369-372`. DISABLE records reason best-effort.
  `audit_ctx` (actor = `session.user_id`) + `json_escape` helpers.
- `crates/infrastructure/src/adapters/sqlite_audit_log.rs`: new
  `insert_delete_audit_row` — DISABLE/PURGE rows through the SAME hash chain via
  `insert_cash_audit_row`; `new_json` carries `reason` + `prior_disable_seq`
  (MAX sequence_no of a prior DISABLE for the entity, null when purged unreferenced);
  `old_json` is the before-state id snapshot (`{"id":…[, "name":…]}`).
- Adapters `sqlite_customer/supplier/warehouse_repository.rs`: mutation + audit row
  in the SAME `run_transaction` (customer `hard_delete` converted from single
  statement to tx for atomicity). Missing-row behaviour preserved at use-case level
  (existence is checked before refs-count).
- `crates/commands/src/dto.rs`: `DeleteEntityRequestDto.reason` (+3 lines, CRLF-clean).
- `crates/commands/src/ipc.rs`: 4 handlers forward `&request.reason`.
- Tests: `crates/application/tests/returns_use_case_tests.rs` mock updated to new
  trait signatures; `dual_track_deletion_timbre.rs` 8 call sites pass `"test cleanup"`;
  `entity_deletion_audit_tests.rs` updated to 3-arg calls.

## (3) PASS-after + G4 outcome

- New suite: `test result: ok. 4 passed; 0 failed` —
  `red_customer_hard_delete_writes_audit_row`,
  `red_warehouse_hard_delete_writes_audit_row`,
  `red_supplier_hard_delete_writes_audit_row`,
  `red_supplier_with_open_debt_is_refused`.
  (Names keep the `red_` prefix as the red-before-green record; §1 above is the RED quote.)
- `dual_track_deletion_timbre`: `test result: ok. 8 passed; 0 failed`.
- G1 `cargo fmt --check`: exit 0. G2
  `cargo clippy --all-targets --all-features -- -D warnings`: clean (one iteration:
  removed unused `CreateWarehouseUseCase` import; updated returns-test mock).
- G3 `cargo test --workspace -j 2`: EXIT=101 with exactly 2 failures, BOTH in
  `crates/infrastructure/tests/migration_replay_and_data_preservation.rs:462`
  (`assertion left == right failed: no products_fts table exists in this schema line`):
  `migration_replay_preserves_legacy_business_data` and
  `downgrade_guard_probe_future_version_is_deterministic` — same pre-existing
  BRK-076 replay-allowlist assertion (076's 0016 adds `products_fts`; fix belongs to
  076, NOT touched here). 33 other suites ok. The named 80k-consistency case is
  `#[ignore]`d and does not run in default G3 (BRK-076 territory, cited only).
- G4-G7: N/A backend-only (no `src/**`, no frontend touched).

## (4) Deferred gaps with owner pointers (NOT fixed here, by design)

1. **Product-adapter audit** — `DeleteProductUseCase` validates `reason` on PURGE
   but `sqlite_catalog_repository.rs` cannot yet emit the audit row in-tx (BRK-076
   worktree owns the file). Emitting there is REMAINING work after 076 lands;
   product-audit test stays unwritten until then.
2. **Frontend reason wiring** — `src/**` untouched per SCOPE-OUT. Compat gap: the
   engine now REQUIRES `reason` (≥3 chars) on purge paths, but `src/lib/ipc.ts`
   sends no `reason` yet (frontend passes `""` → engine rejects with
   `INVALID_INPUT`). DSN-082 owns the modal typed-gates + `reason` threading.

## (5) Hygiene

- `git diff --stat` (BRK-080 files): 10 files, 288+/86-, `dto.rs` exactly +3,
  no whole-file churn (CRLF byte-discipline preserved; one `cargo fmt` whole-file
  rewrite incident reverted, edits re-applied surgically + verified
  `fmt --check` clean).
- Tree also holds pre-existing uncommitted BRK-076 / BRK-078 / BRK-079 work
  (0016, fts tests, catalog repo, connection, migration, backup/migration tests,
  audit/BRK-076|078|079) — NOT touched by this task.
- No live DB opened (tempdir only); no new dependencies; no ad-hoc scripts left
  (one G3 log in scratch deleted). DO NOT COMMIT.
