# TASK-BRK-078 REPORT — checkout print-outbox pins (premise corrected)

## (3) Overlap check: PASS — no overlap.
`git status` BRK-076 paths (catalog repo, connection, migration, backup/migration
tests, 0016, fts tests) are disjoint from BRK-078 paths. No BRK-076 file touched.

## Task-premise conflict (AGENTS.md §1: repo fact wins)
AUD-038 claimed "checkout never enqueues; `enqueue_job` has zero production
callers". That grep missed the real call path:
`crates/infrastructure/src/adapters/sqlite_checkout_repository.rs:465-479`
**step 8** calls `enqueue_in_tx(&tx, &job_id, &saved.document_id, "RECEIPT",
&payload_json)` **inside the sale transaction, before COMMIT** (after
`command_log` row at :459). `git log -S enqueue_in_tx` → present since the
05f0f39 baseline: the enqueue PREDATES the redesign, it is not missing. The
`CheckoutRepository` port (`ports.rs:1249`) documents "print outbox" as part of
the atomic unit of work. Conclusion: fix-A1's production change is
**already implemented** — no checkout edit was made or needed.

## What was actually done (test-only diff)
New `crates/infrastructure/tests/checkout_print_outbox_tests.rs` (2 tests)
pins the correct behavior as regression guards:
- `committed_sale_enqueues_exactly_one_pending_job`: sale → exactly 1 PENDING
  job (`job_type=RECEIPT`, `document_id` matches receipt); replay same
  `command_id` → same receipt id, still 1 job.
- `failed_sale_enqueues_no_job`: shortage-gated failure → error + 0 jobs.
Payload matches the mandated convention (document_id/number/total_ttc, same
shape reprint consumes). Step 8 is unconditional on doc type, so TICKET/BL/
FACTURE share the path (pinned TICKET + BL-adjacent cash ticket; FACTURE-credit
flows through the same step).

## (1) FAIL-before: N/A with reason — there is no "broken" prod code to fail
against. First run failed only on a test-harness mistake of mine
(`InsufficientPayment`: 30_000 cash vs 30_000 + timbre), fixed by paying
50_000. After the fix, first real run: **2 passed, 0 failed** — the behavior
was correct from the start, which IS the evidence the premise was stale.

## (2) PASS-after + counts
`cargo test -p baraka-infrastructure --test checkout_print_outbox_tests`:
`test result: ok. 2 passed; 0 failed.` Existing `print_worker` suite
(checkout→job→worker integration) untouched and unaffected.

## Where "printing does not work" actually breaks (for the fix tasks)
Enqueue is green. Downstream per AUD-038 TRACE: worker polls 500ms ✓, thermal
renderer ✓, `SerialPrinterTransport` opens `printer.serial_port` per job —
**unset/empty port → `None` → `PrinterTransportUnavailable` → PRINTER_OFFLINE
retry → `EXHAUSTED_MAX_RETRIES`** (`transport_serial.rs:24-70`,
`sqlite_printer_settings.rs:62` defaults empty). On a machine with no COM port
configured, jobs pile up FAILED — consistent with the owner symptom. Plus:
no A4 renderer for BL/FACTURE (thermal-only), and the Details drawer never
calls `getDocumentDetails`.

## (4) G1-G3
- G1 `cargo fmt --check`: exit 0.
- G2 `cargo clippy --all-targets --all-features`: zero errors/warnings.
- G3 `cargo test --workspace -j 2`: 321 passed / **2 failed** — both failures
  are in `migration_replay_and_data_preservation.rs`
  (`migration_replay_preserves_legacy_business_data`: "no products_fts table
  exists in this schema line"; `downgrade_guard_probe_future_version_is_...`),
  i.e. the uncommitted BRK-076 migration-0016 worktree, NOT this task's scope
  (this task touched zero prod files). Own suite:
  `checkout_print_outbox_tests`: **2 passed, 0 failed**.
- G4-G7 N/A backend-only (zero frontend touch).

STATE: complete-as-pins (no prod change needed) / DONE: 2 regression pins
green + premise correction with evidence / REMAINING: real print fix = printer
configuration/UX surface (A3), A4 dispatch (A2), drawer B1 — separate tasks.
