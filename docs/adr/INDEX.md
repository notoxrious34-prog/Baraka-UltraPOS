# ADR Index

Generated and maintained under SESSION-ARC-001 (Charter v2.0). Evidence base: AUD-004 (audit/AUD-004/S4.txt) at HEAD 977a804.

## 1. ADR-001..015 - defined in docs/architecture/MASTER-ARCHITECTURE-v1.0.0.md (section 36)
Status of all: ACCEPTED as architectural baseline. Realization status differs per item; see docs/architecture/MASTER-ARCHITECTURE-ERRATA.md.

| ID | Title |
|---|---|
| ADR-001 | Tauri v2 + Rust |
| ADR-002 | SQLite + rusqlite, no runtime ORM |
| ADR-003 | Money/Qty/Factor fixed-point model |
| ADR-004 | Typed IPC wire representation |
| ADR-005 | Atomic Sale / Unit of Work |
| ADR-006 | Transactional Outbox for printing |
| ADR-007 | Append-only ledger enforcement |
| ADR-008 | Gapless committed document numbering |
| ADR-009 | SQLCipher production profile (NOT realized: no sqlcipher in the repo; OPEN OWNER DECISION) |
| ADR-010 | Backup / Recovery protocol |
| ADR-011 | FTS5 and normalization (NOT realized: FTS absent) |
| ADR-012 | POS keyboard contract |
| ADR-013 | Tax rule versioning and legal validation gate (engine part deferred by ADR-068; validation gate stays open) |
| ADR-014 | Future synchronization metadata |
| ADR-015 | Performance benchmark methodology |

## 2. ADR-016..061 - referenced only, no definition document
The original texts lived in the previous architect's context packages, which the current Architect does not hold. These ids are reserved and must not be reused. The table lists ids that are referenced in the repository (scan roots: docs, crates, src, src-tauri/src, crates/infrastructure/migrations; excluded: docs/adr).

| ID | Occurrences | First reference (path:line) |
|---|---|---|
| ADR-025 | 9 | crates/application/src/use_cases/cash.rs:1: //! Cash session lifecycle use cases (ADR-025/026/027). |
| ADR-027 | 4 | crates/application/src/ports.rs:990: /// ADR-027: the full amount is cash-collected into the active session. |
| ADR-028 | 11 | crates/application/src/use_cases/sales.rs:802: /// ADR-028: `terminal_id` is resolved from TerminalContext at the IPC |
| ADR-029 | 15 | crates/application/src/ports.rs:1654: /// when no source BR (standalone combined receipt, ADR-029). |
| ADR-030 | 14 | crates/application/src/ports.rs:1670: /// SUPPLIER_PAYMENT_OUT only for CASH per ADR-030). |
| ADR-031 | 1 | crates/infrastructure/tests/supplier_purchase_lifecycle_tests.rs:421: assert_eq!(view.status, "FINALIZED"); // ADR-031: committed == FINALIZED |
| ADR-033 | 1 | crates/infrastructure/tests/supplier_purchase_lifecycle_tests.rs:462: // Gerant holds Permission::PriceOverride (ADR-033): the use case passes |
| ADR-034 | 2 | crates/domain/src/sales/document.rs:93: /// Purchase-order rollup: some ordered quantity received (ADR-034). |
| ADR-037 | 10 | crates/application/src/ports.rs:1759: /// One low-stock row (ADR-037: stockout / depletion signal). |
| ADR-039 | 1 | src/features/reports/components/report-format.ts:2: * Integer-only display helpers shared by every report screen (ADR-039). |
| ADR-040 | 12 | crates/application/src/ports.rs:603: /// Execute a single-shot DEVIS-to-BL conversion atomically (ADR-040): |
| ADR-044 | 1 | crates/infrastructure/src/adapters/sqlite_conversion_repository.rs:747: // q. Print outbox job (checkout RECEIPT pattern, ADR-044). |
| ADR-045 | 4 | crates/application/src/use_cases/mod.rs:286: /// `terminal_id` is TerminalContext-resolved at the IPC edge (ADR-045). |
| ADR-046 | 9 | crates/application/src/ports.rs:618: /// `run_transaction`. Zero stock writes by invariant (ADR-046). |
| ADR-048 | 3 | crates/application/src/use_cases/mod.rs:276: /// First-run administrator bootstrap (ADR-048). |
| ADR-049 | 5 | crates/infrastructure/src/persistence/connection.rs:25: /// application data directory (ADR-049). |
| ADR-050 | 1 | docs/audits/LOGIN_PIPELINE_TRACE_AUDIT.md:511: ADR-050 shape. Success `T` (`dto.rs:72-84`): `session_token`, `user_id`, |
Unreferenced ids in 016..061: 016, 017, 018, 019, 020, 021, 022, 023, 024, 026, 032, 035, 036, 038, 041, 042, 043, 047, 051, 052, 053, 054, 055, 056, 057, 058, 059, 060, 061.
Total referenced ids in 016..061: 17.

## 3. ADR-062..065 - from the previous architect's package SESSION-BRK-011
Recorded as stated in that package. Not independently re-verified by the current Architect.

| ID | Summary | Status |
|---|---|---|
| ADR-062 | Safe restore protocol: two IPC commands (read-only precheck; execute re-verifies everything internally); quarantine instead of deletion; unified rollback net; RESTORE_PERFORMED audit event; exempt from command_log because the database itself is replaced. | Implemented (per previous package). Confirmation UX amended by ADR-074. |
| ADR-063 | Crash and power matrix: external leader process re-launches itself as child; kill points injected through SQLite hooks (update_hook / commit_hook) on leader-owned connections; no traps in production code; rusqlite feature "hooks" authorized with Cargo.lock fingerprint; honesty limit: process death is not OS power loss. | Implemented (per previous package). |
| ADR-064 | In-transaction idempotency claim: duplicate check is the first step inside the transaction, claim insert (with payload) is the last; request_hash comparison distinguishes conflict, replay, resume. | Implemented (per previous package). |
| ADR-065 | Printing diagnostics and capture: test print is a synchronous settings-scoped diagnostic (not an outbox job, no accounting effect); capture sink; preview from the shared layout lines; structural stream inspector; the single source of truth for the device is the printer.serial_* settings. Includes Amendment 1. | Implemented (per previous package). Output profiles extended by ADR-070. |

## 4. ADR-066..085 - Architect (Charter v2.0)

| ID | Title | Status |
|---|---|---|
| ADR-066 | Viewport baseline and window policy | ACCEPTED |
| ADR-067 | Single top bar; no floating action buttons | ACCEPTED |
| ADR-068 | Tax regime IFU: no VAT in v1 | ACCEPTED (validation gate OPEN) |
| ADR-069 | Locales fr + ar, Latin digits always | ACCEPTED |
| ADR-070 | Output formats: thermal 58, thermal 80, A4 | ACCEPTED |
| ADR-071 | Deployment topology: single device, single instance | ACCEPTED |
| ADR-072 | Catalog migration from Dolisoft export | ACCEPTED (amended by ADR-075 and ADR-077; profiling completed in AUD-005-C1) |
| ADR-073 | Canonical repository location | ACCEPTED |
| ADR-074 | Destructive-action confirmation pattern | ACCEPTED |
| ADR-075 | Catalog import scope: products only | ACCEPTED |
| ADR-076 | Isolated data directory override (BARAKA_DATA_DIR) | ACCEPTED |
| ADR-077 | Product entry priority, French name with optional Arabic alias, generic import/export | ACCEPTED |
| ADR-078 | Information architecture and navigation | ACCEPTED |
| ADR-079 | Roles in use and manager approval in place | ACCEPTED |
| ADR-082 | Debt-receipt numbering reuses customer_ledger sequence | ACCEPTED |
| ADR-083 | Debt-receipt printing uses a fully separate rendering path | ACCEPTED |
| ADR-084 | Persisted CTP files authored in Arabic, superseding ADR-080 | SUPERSEDED (by ADR-085) |
| ADR-085 | English-language CTP persistence re-ratified by owner, superseding ADR-084 | ACCEPTED |
| ADR-086 | Release Candidate v1.0.0 Packaging and Production Approval | ACCEPTED |
| ADR-087 | Artisan & wholesale pricing tiers, Margin Guard, official brand identity | ACCEPTED |

| ADR-088 | Single-instance application mutex and quincaillerie starter fixture | ACCEPTED |

Next free id: ADR-089.
