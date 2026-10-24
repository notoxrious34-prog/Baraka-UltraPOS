# TASK-BRK-089 REPORT — USB-direct spooler RAW transport

STATE BLOCKED (safety-lint contradiction — no production changes made).
No commit. Live DB never opened.

## (1) Dep + lock lines

NONE ADDED. `windows = "0.62"` was authorized but wiring it is
functionally blocked (see conflict): adding an unused dependency plus
lockfile churn for zero working code would itself violate the
no-dead-code/no-placeholder rules, so `Cargo.toml` + `Cargo.lock`
are untouched.

Offline-cache evidence (closure present for future unblock):
`windows-0.62.2.crate` + extracted src, `windows-core-0.62.2`,
`windows-collections-0.3.2`, `windows-future-0.3.2`,
`windows-numerics-0.3.1`, `windows-result-0.3.4/0.4.1`,
`windows-link-0.1.3/0.2.1`, `windows-strings-0.4.2/0.5.1`,
`windows-targets-0.52.6`, `windows-interface-0.59.3`
(`~/.cargo/registry/cache/...`). Not verified with `--offline`
resolve (no manifest edit made).

## (2) Transport / error-map lines

NOT IMPLEMENTED (blocked). Groundwork mapped, file:line each:

- Trait: `crates/application/src/ports.rs:1434-1440`
  (`PrinterTransportPort::send_raster` / `is_available`).
- Serial impl (frozen reference): `crates/infrastructure/src/printing/
  transport_serial.rs:58-82` (fresh settings read per call;
  missing port → `Validation(PrinterTransportUnavailable)`).
- Taxonomy target: `crates/domain/src/errors.rs:110-111`
  (`PrinterTransportUnavailable`, "printer transport unavailable");
  test-print maps `Validation(d)` → `"{d}"` via `device_error_code`
  (`crates/application/src/use_cases/printing.rs:376-381`).
- Wiring point: `src-tauri/src/lib.rs:35,123`
  (`SerialPrinterTransport::new` → `AppState.printer_transport`).
- Settings KV (no migration needed — upsert pattern):
  `crates/infrastructure/src/adapters/sqlite_printer_settings.rs:33-40,
  88-98` (new keys `printer.transport`, `printer.queue_name` fit).
- IPC surface to extend on unblock: `crates/commands/src/ipc.rs:2856-2894`
  (`printer_get/update_settings`), `TestPrintUseCase`
  (`printing.rs:288-374`, transport-agnostic already — routing covers it).

## (3) Test lines

No new tests (nothing to test — no code written).
Serial-regression baseline (unchanged tree): existing suites green,
see gate table (493/0 includes `print_worker`, `print_rendering`,
`dual_width_thermal_printing`, `checkout_print_outbox` paths).

## (4) G1-G3 exits + counts (baseline, unchanged tree)

| Gate | Result |
|---|---|
| G1 `cargo fmt --check` | EXIT 0 |
| G2 `cargo clippy --all-targets --all-features -- -D warnings` | EXIT 0 |
| G3 `cargo test --workspace -j 2` | EXIT 0 — 65 suites, 493 passed / 0 failed |
| G4–G7 | N/A — backend-only task, zero frontend files touched |

## (5) Git status --short

(no production changes; expected: clean except pre-existing worktree
state — verified below)

## STATUS / CONFLICT / PROPOSAL

STATUS: BLOCKED.
REASON (exact contradiction): the Architect-set design mandates
`OpenPrinterW → StartDocPrinterW → WritePrinter → EndDocPrinter →
ClosePrinter`, and every one of those is `pub unsafe fn` in the
authorized windows 0.62.2
(`.../Windows/Win32/Graphics/Printing/mod.rs:219,577,1187,1490,1535`).
Repository fact: `unsafe_code = "forbid"` in ALL FIVE manifests
(`crates/{domain,application,infrastructure,commands}/Cargo.toml`,
`src-tauri/Cargo.toml`). A `forbid`-level lint cannot be relaxed by a
module-level `allow` — any `unsafe {}` block is a hard compile error
in every crate that could host the transport. The dependency
authorization alone does not unblock: declaring `windows = "0.62"`
without calling it is dead weight; calling it does not compile.
No safe alternative exists within scope: shell-out printing is a
different API (handoff forbids wrapping another), cannot guarantee
RAW byte-identity, and a stub would be prohibited placeholder code.
QUESTION FOR ARCHITECT: authorize ONE of (a) a new minimal
`spooler-ffi` crate WITHOUT the forbid (audited `unsafe` wrappers,
`deny` + review), owned by infrastructure; or (b) downgrade
`unsafe_code` to `deny` with a per-module allow + audit note; or
(c) redirect to a safe out-of-process helper (explicit new design).
PROPOSED SOLUTION (not implemented until approved): (a) — keeps the
workspace forbid intact everywhere else; transport keeps the exact
Win32 RAW call sequence; seam-test the safe boundary as specced.
NEXT READY: NO.
