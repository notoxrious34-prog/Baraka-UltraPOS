# ADR-108 — Split `dto.rs` into domain modules (TASK-BRK-106)

Status: ACCEPTED. Date: 2026-11-07 (TASK-BRK-106). Authority: Sovereign (self-tasked under v3.1 §1.3).

## Context

`crates/commands/src/dto.rs` held all 171 IPC DTOs (2806 lines) in one
file — same structural smell as `ipc.rs` (F7, 4508 lines). Navigation,
review, and future per-domain splits (TASK-BRK-107: `ipc.rs`) all suffer.

## Decision

Pure code motion, zero behavior change:

- `dto.rs` → `dto/mod.rs` hub: 12 `pub mod` + `pub use` re-exports.
- 12 modules: `admin, auth, cash, catalog, common, customers, documents,
  inventory, printing, reports, sales, suppliers`.
- Every path stays `crate::dto::X`; no caller (ipc.rs, src-tauri) changed.
- Only `documents.rs` keeps `use super::*` (it reuses sales'
  `PaymentLineDto` in `ConvertBlToFactureRequest`); the other 11 modules
  are self-contained and import only `serde` (+ `baraka_domain` in
  `common`).

## Conformance

- Byte-identity proof: concatenated module bodies == original file body
  (98,049 chars, verified by script before commit).
- G1 `cargo fmt --check` clean; G2 clippy `-D warnings` clean
  (11 unused-glob warnings found and removed during the task);
  G3 `cargo test --workspace` green (see task report).
- Line endings normalized to LF (repo convention; the old file was CRLF).

## Consequences

- `ipc.rs` split (TASK-BRK-107) can now mirror these exact module
  boundaries.
- A mid-task incident (modules drafted from compacted context instead of
  ground truth) was caught by the mandatory byte-diff gate; the rule is
  now explicit: never treat collapsed/demoted tool output as source.
