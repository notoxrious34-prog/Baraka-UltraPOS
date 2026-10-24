# TASK-BRK-090 REPORT — Graphique ticket mode (setting + worker FRONTEND_RENDERED)

Status: DONE. No commit. Live DB never opened (all tests use tempdir databases).

## 1. Authority
- ADR-104 (`docs/adr/ADR-104-ticket-rendering-split.md`, ACCEPTED 2026-10-24): setting
  `ticket_render_mode` (`escpos` default, validated); graphique worker emits zero ESC/POS
  bytes; job transitions to terminal `FRONTEND_RENDERED` + audit event; frontend renders
  80mm HTML via OS graphics (DSN-093 follow-up, out of scope here).
- Owner hardware verdict (XP-410B: queue exact, Idle, graphics OK, ESC/POS bytes silently
  discarded) is the reason graphique exists; hardware re-verification stays open
  (HARDWARE-PENDING).

## 2. Setting + validation (lines)
- `crates/domain/src/printing/transport.rs`: `TICKET_RENDER_ESC_POS` / `TICKET_RENDER_GRAPHIQUE`
  consts + `validate_ticket_render_mode()` — rejects anything but `escpos`|`graphique`
  (trimmed) with `ticket_render_mode must be 'escpos' or 'graphique', got '{other}'`.
  Unit tests: `accepts_both_render_modes`, `rejects_unknown_render_mode` (tspl/empty/ESC POS).
- `crates/domain/src/printing/mod.rs`: re-exports.
- `crates/application/src/ports.rs`: `PrinterSettings.ticket_render_mode`; trait
  `update_printer_settings(..., ticket_render_mode: Option<&str>)` (`None` keeps stored).
- `crates/application/src/use_cases/printing.rs` (`UpdatePrinterSettingsUseCase::execute`,
  +`#[allow(clippy::too_many_arguments)]` per G2): `Some(mode)` validated pre-write, zero
  writes on rejection.
- `crates/infrastructure/src/adapters/sqlite_printer_settings.rs`: get defaults `escpos`
  on absent/invalid key; update persists trimmed `printer.ticket_render_mode` when `Some`.
  Zero schema changes (existing `settings` KV).
- IPC: `crates/commands/src/dto.rs` (`PrinterSettingsDto` + `UpdatePrinterSettingsRequest`
  +`ticket_render_mode`, `#[serde(default)]`); `crates/commands/src/ipc.rs`
  (`printer_get_settings`, `printer_update_settings` map the field both ways).
- Byte-hygiene note: `dto.rs` is mixed CRLF/LF in HEAD; the edit was re-applied via
  script preserving original endings — final `git diff` for `dto.rs` is +7/-0 lines.

## 3. Worker branch (lines)
- `crates/domain/src/printing/print_job.rs`: `PrintJobStatus::FrontendRendered`,
  code/from_code `FRONTEND_RENDERED` (terminal like DONE), round-trip unit test.
- `crates/application/src/services/print_worker_service.rs`: `JobOutcome::FrontendRendered`;
  `process_one_job_with_identity` resolves graphique per job (settings failure → escpos
  default, never fells the job); `process_claimed(..., graphique: bool)` — graphique sale
  jobs `mark_frontend_rendered()` + return BEFORE hydrate/render/transport. Legacy
  `process_one_job` passes `false` (escpos byte-identical). Debt pipeline untouched.
- `crates/infrastructure/src/adapters/sqlite_print_job_repository.rs`
  (`mark_frontend_rendered`): ONE transaction — `status='FRONTEND_RENDERED'` (no attempt++,
  no duplicata watermark) + `insert_cash_audit_row(TICKET_FRONTEND_RENDERED, print_jobs,
  job_id, {"status":"FRONTEND_RENDERED"}, actor print-worker)`.
- `crates/infrastructure/src/printing/worker_thread.rs`: `FrontendRendered` matches with
  Done/Failed (terminal, no sleep).
- Reprint semantics (state behavior, no extra code): reprocessed DUPLICATA in graphique
  mode → FRONTEND_RENDERED, zero bytes; `get_job_status` returns the code string as-is.
- Atomicity invariant preserved: job row still created inside the sale transaction with
  initial status `PENDING` (`enqueue_in_tx` untouched; `checkout_print_outbox_tests`
  green: `committed_sale_enqueues_exactly_one_pending_job`).

## 4. Migration verdict (0018 NOT needed + proof)
- `print_jobs.status` carries NO database CHECK in any migration:
  `0001_baseline_schema.sql:279-288` (`status TEXT NOT NULL`, no CHECK);
  `0004_sales_engine_schema.sql` (adds job_type/payload only);
  `0006_print_worker_columns.sql` (adds last_error/claimed_at only);
  `0014_make_print_jobs_document_id_nullable.sql` (rebuild preserves columns, no CHECK).
  `grep CHECK` across `migrations/*.sql` hits only stock/cash/supplier tables.
- Statuses are enforced in code (`PrintJobStatus::from_code`); new file `migrations/0018*`
  NOT created. Drills enumerate no statuses (`forensic_full_system_drill.rs:390,643-647`
  assert row counts; `crash_matrix_tests.rs:334` counts) → no drill updates.

## 5. Test lines
- New: domain transport (2) + status round-trip (1); `spooler_transport_tests.rs`
  (2: `ticket_render_mode_defaults_to_escpos_and_round_trips`,
  `use_case_rejects_bad_render_mode_before_any_write` — quotes the contract string, proves
  zero writes); `print_worker.rs` (3: `graphique_mode_frontend_renders_without_bytes_and_audits`
  with transport-spy `sends()==0`/`attempts==0`/audit assertion,
  `escpos_mode_stays_on_the_byte_path` with `sends()==1`/no job audit,
  `graphique_reprint_returns_terminal_status_without_bytes` draining both PENDING rows).
- Regression note (new-code fallout, test-only fix): first reprint-test run failed because
  the original sale PENDING row claims before the DUPLICATA row; fixed by draining both
  (`print_worker.rs` loop), production code untouched.
- Serial/escpos suites green unchanged (dual_width 7, print_worker legacy 15, debt_wiring 4,
  store_identity 14, serial path via `defaults_select_the_serial_arm`).

## 6. Gates
| Gate | Result |
|------|--------|
| G1 `cargo fmt --check` | CLEAN (exit 0; one `cargo fmt` pass for long-line wrapping) |
| G2 `cargo clippy --all-targets --all-features -- -D warnings` | GREEN exit 0 (one new-code fix: `#[allow(clippy::too_many_arguments)]` on the 8-arg use-case execute, same precedent as `process_claimed`) |
| G3 `cargo test --workspace -j 2` | GREEN exit 0: **515 passed, 0 failed, 7 ignored** (unit 137: app 28, returns-use-case 10, commands 5, domain 72 incl. 3 new, infra-lib 22; integration 378 incl. print_worker 18/18 with 3 new, spooler 7/7 with 2 new, dual_width 7/7; crash_matrix suite ok 17/17 scenarios; spooler_raw 5; window_config 2; ignored 7: fts backfill 80k, hardware p95, 3 release benches, release drill, search 80k) |
| G4-G7 | N/A backend-only: no `src/**`, no `ipc.ts`, no frontend file touched (reason: DSN-093 owns the 80mm HTML ticket UI; G5 not triggered) |

## 7. Diff scope
14 modified files, no new files except this report: domain printing (transport, mod,
print_job); application ports, use-cases/printing, print_worker_service; infra
sqlite_printer_settings, sqlite_print_job_repository, worker_thread; commands dto, ipc;
tests dual_width, print_worker, spooler. No production changes outside SCOPE-IN.
Scratch `D:/Dev/_scratch/BRK-090/` (fix_endings.py, fix2.py) DELETED, verified absent.

## 8. UNEXPECTED FINDINGS
- Pre-existing doc-comment corruption in `dto.rs` (`"erial"`, `"poolerol"`, `"serialol"`)
  is in HEAD — left untouched (out of scope).
- `dto.rs` mixed line endings are pre-existing; preserved (+7/-0 final diff).

## 9. UNKNOWNs
- None for BRK-090 scope. HARDWARE-PENDING: owner USB print of a graphique ticket is
  DSN-093 + owner trial (no printer here).

NEXT READY: YES
