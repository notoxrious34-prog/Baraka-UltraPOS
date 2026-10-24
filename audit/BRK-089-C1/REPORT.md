# TASK-BRK-089-C1 REPORT — USB spooler RAW transport (ADR-103)

STATE DONE (green close-out). No commit. Live DB never opened.

## (1) Dep + lock lines

`Cargo.toml` (workspace):
`windows = { version = "0.62", features = ["Win32_Graphics_Printing", "Win32_Graphics_Gdi", "Win32_Foundation"] }`
(+ ADR-103 comment; authorized dep, single authorized version).
`Win32_Graphics_Gdi` is REQUIRED, not optional: `PRINTER_DEFAULTSW` is
gated behind it in windows 0.62.2 (adapted + recorded, no version change).

`crates/spooler_raw/Cargo.toml` (new member): `windows = { workspace = true }`
under `[target.'cfg(windows)'.dependencies]` ONLY (minimal surface).
`crates/infrastructure/Cargo.toml`: `spooler-raw = { path = "../spooler_raw" }` ONLY.

`Cargo.lock` diff: +`spooler-raw 1.0.0 → windows` edge and the windows
entry ONLY — zero `windows-*` companion changes (all pre-locked via the
existing Tauri closure). No other new dependency.
Network use (recorded): one online `cargo metadata` downloaded
polling/zvariant/zbus (missing Tauri cache entries, unrelated to
windows); the windows 0.62.2 closure itself was fully cached.

## (2) API + error-map lines

- Safe API: `crates/spooler_raw/src/lib.rs` — `SpoolerError` (:41:
  `InvalidQueueName | OsCallFailed{call,queue,os_code} | UnsupportedPlatform`),
  `SpoolerQueue::open` (:108), `probe` (:115, open+close WITHOUT a
  document — availability checks never spool empty jobs), `write_raw`
  (:229, all-or-error loop + `EndDocPrinter` surfacing), RAII `Drop`
  (:282), `submit_raw` (:311). Call order OpenPrinterW →
  StartDocPrinterW(RAW) → WritePrinter → EndDocPrinter → ClosePrinter.
  `Error::from_thread()` captures the GetLastError code (windows-result
  0.4 has no `from_win32` — adapted + recorded).
- Domain rules: `crates/domain/src/printing/transport.rs` —
  `validate_printer_transport` (:22, serial|spooler strict),
  `validate_spooler_queue_name` (:35, non-empty, ≤220 chars, unicode OK).
- Selection: `transport_router.rs:35 use_spooler` (fresh read per call;
  absent/garbage → serial legacy fallback), `transport_spooler.rs:50
  map_error` (empty queue → existing `PrinterTransportUnavailable`,
  overlong/OS failures → existing `InvalidCanonicalString` carrying
  call+queue+code — no new taxonomy).
- Plumbing: `ports.rs` (+transport/queue_name, extended update sig);
  `sqlite_printer_settings.rs` (KV read/write, validated upstream);
  `use_cases/printing.rs` (all rejections before any write, None keeps
  stored); `dto.rs` (+2 fields both structs, serde-defaulted request);
  `ipc.rs:2871-2872,2889-2890,2898-2899`; `src-tauri/lib.rs:35,123`
  (router wired). Serial body byte-identical (untouched).

## (3) Test lines

- `spooler_raw` unit (5): validation empty/overlong/boundary, verbatim +
  unicode queue names, unknown queue fails clean (names OpenPrinterW,
  no panic), display format. GREEN 5/0.
- `crates/infrastructure/tests/spooler_transport_tests.rs` (5): serial
  default + serial-arm taxonomy, spooler routing with `OpenPrinterW` +
  queue in diagnostic, no-queue unavailability, round-trip + None-keeps,
  use-case triple-reject with zero-writes proof. GREEN 5/0.
- Domain `transport.rs` unit (4): both arms, unknown arm, exact queues,
  empty/overlong/boundary. GREEN (in domain suite).
- Fixed body (C2 defect): `crates/domain/src/printing/transport.rs:76-84`
  `rejects_empty_and_overlong_queue_names` now uses `"x".repeat(...)`
  forms (plus identical-pattern fix in `spooler_raw/src/lib.rs`
  `rejects_overlong_queue_names`, `"ن".repeat(...)` — disclosed).
- Serial regression: `print_worker` 15/15, `print_rendering` 5/5,
  `dual_width_thermal_printing` 7/7, `checkout_print_outbox` 2/2,
  `store_identity` 14/14 — all green, zero changes to serial behavior.

## (4) Forbid / unsafe grep (re-verified AFTER final edits)

- Forbids intact (6 grep hits = 5 `forbid` + 1 explanatory comment):
  application:18, commands:20, domain:19, infrastructure:37,
  src-tauri:32 all `unsafe_code = "forbid"`; spooler_raw:10 is the
  ADR-103 comment documenting the deliberate exception.
- `unsafe {` blocks: 7, ALL in `crates/spooler_raw/src/lib.rs`, each with
  a SAFETY comment. Zero elsewhere (the single other `unsafe` hit is a
  doc comment in `transport_spooler.rs:4`).

## (5) G1-G3 exits + counts

| Gate | Result |
|---|---|
| G1 `cargo fmt --check` | EXIT 0 |
| G2 `cargo clippy --all-targets --all-features -- -D warnings` | EXIT 0 |
| G3 `cargo test --workspace -j 2` | EXIT 0 — 68 suites ok, 507 passed / 0 failed / 7 ignored (slow/bench drills) |
| G4–G7 | N/A — backend-only, zero frontend files touched |

New coverage in G3: spooler_raw 5, spooler_transport_tests 5, domain
transport 4 (493 baseline → 507).

## (6) git status --short

M: Cargo.toml, Cargo.lock, crates/application/src/ports.rs,
use_cases/printing.rs, crates/commands/src/dto.rs (+10), ipc.rs,
crates/domain/src/printing/mod.rs, crates/infrastructure/Cargo.toml,
adapters/sqlite_printer_settings.rs, printing/mod.rs,
tests/dual_width_thermal_printing.rs, tests/print_worker.rs,
src-tauri/src/lib.rs.
??: crates/spooler_raw/, crates/domain/src/printing/transport.rs,
crates/infrastructure/src/printing/transport_spooler.rs,
transport_router.rs, tests/spooler_transport_tests.rs,
audit/BRK-089/ (pre-existing, tracked), audit/BRK-089-C1/ (this file).
NO_LEFTOVERS: no ad-hoc scripts (all work via tracked files).
DO NOT COMMIT honored.

## HARDWARE-PENDING + STATE/DONE/REMAINING

HARDWARE-PENDING: no real printer was touched. Byte-path integrity
rests on (a) both arms forwarding the same `&[u8]` untouched,
(b) the spooler write loop asserting full delivery or error, and
(c) RAW datatype passthrough — live delivery to the XP-410B queue is
for the owner USB test (blocked next: DSN queue-field).
STATE: DONE. DONE: defect fixed, G1/G2/G3 green with counts, REPORT
written, forbids + unsafe confinement re-verified, tree clean of
strays. REMAINING: none in scope. NEXT READY: YES.
