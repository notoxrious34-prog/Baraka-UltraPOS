# ADR-126: Dual-printer role routing (BRK-113)

Status: ACCEPTED (implemented, gates green).

## Context
Wave 3 (`ROADMAP-v2.0-DUAL-VERTICAL.md`) requires two printers: the
counter receipt printer and a dedicated `XP-410B` label printer.
A single global `printer.transport` / `printer.queue_name` pair cannot
express that: a USB label printer plus a COM receipt printer need
independent arms and independent spooler queues, including a fallback
queue per role. The owner directive stands: no printer hardware is
currently attached, the existing print path runs with high precision,
and the routing is built on it and tested logically without paper.

## Decision
Two independent roles, `receipt` and `label`:

- Domain (`crates/domain/src/printing/routing.rs`): `PrintRole` with
  `transport_key()` / `queue_key()` / `fallback_queue_key()`;
  `resolve_role_transport` (role key wins; receipt inherits the legacy
  global, label never does — an unconfigured label role means the
  spooler arm with an empty queue, which fails LOUDLY); `RoleQueues` +
  `resolve_role_queues` (empty primary rejected, blank fallback means
  none, distinct-or-singleton); `should_try_fallback` (try the fallback
  exactly once on primary failure, else surface the PRIMARY error —
  never silently redirect).
- Ports: `PrinterSettings` gains six `*_transport` / `*_queue_name` /
  `*_queue_name_fallback` fields; `PrinterTransportPort` gains
  `send_raster_for_role` / `is_available_for_role` with defaults that
  forward to the legacy single-arm methods (zero churn for mocks).
- Infrastructure: `RoutingPrinterTransport` routes per role with one
  fallback attempt; the spooler arm gains `send_to_queue` /
  `queue_available`; the serial (COM) arm stays global (single-COM
  limitation, documented); `SqlitePrinterSettings` reads role keys with
  legacy fallback and writes them in one transaction, mirroring receipt
  back onto the legacy keys so pre-BRK-113 readers keep working. The
  legacy `update_printer_settings` write mirrors transport/queue onto
  the receipt role keys in the same transaction, so both write paths
  keep the two physical copies in sync.
- Read posture (regression-driven): the router falls back to serial on
  an invalid stored RECEIPT arm — pre-BRK-113 databases carry the
  seeded uppercase default and the old router treated any non-`spooler`
  spelling as serial. The LABEL role stays strict (its keys are new).
- Application: `UpdatePrinterRoleSettingsUseCase` (ADMIN-gated, LOUD
  validation of every `Some` arm/queue before any write); every TSPL
  use case gates and sends through its own role (labels + label
  self-test + 40×20 calibration → Label; tickets + bitmap + rulers +
  ticket self-test → Receipt); the print worker and the ESC/POS test
  print ride the Receipt role.
- IPC: `printer_update_settings` accepts six optional role keys; the
  update runs only when at least one is addressed.
- Frontend: `PrinterSettings` carries the six role fields;
  `ipcUpdatePrinterRoleSettings` patches them; the Printer page gains a
  numbered roles section (receipt group + label group + own save).

## Consequences
- Single-printer merchants reconfigure nothing: receipt inherits the
  legacy keys byte-for-byte; the label role stays unconfigured until
  addressed, failing loudly when used.
- No migration: all six keys ride the append-only `settings` key/value
  table (`printer.{receipt,label}.{transport,queue_name,
  queue_name_fallback}`).
- The `update_printer_role_settings` port method is intentionally
  required (no default): a future settings port must persist roles
  explicitly, never silently drop them.

## Evidence
- `cargo test -p baraka-domain --lib printing::routing`: 12/12.
- `crates/application/tests/tspl_print_tests.rs`: 25/25 (fake seeds
  both roles from the legacy arm — pre-split behavior preserved).
- G1–G7 green on the BRK-113 tree (G7: 99 files / 989 tests, incl. 6
  new `printer-roles.test.ts` pins).
