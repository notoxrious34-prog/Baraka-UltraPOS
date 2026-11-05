# ADR-109: Split `ipc.rs` into 13 domain modules, pure motion (TASK-BRK-107)

Status: ACCEPTED.

## Context

`crates/commands/src/ipc.rs` (4508 lines, 100 handlers) was the last
monolith after the BRK-106 `dto.rs` split. Same scale problem, same
prescription: mirror the dto boundaries so each IPC module speaks the
vocabulary of one domain.

## Decision

Move `ipc.rs` to `ipc/mod.rs` (hub) plus 13 modules on the dto map:

- `common` (kernel: `AppState`, `app_health`, guards, wire parsers),
  `auth`, `catalog`, `inventory`, `sales`, `returns`, `backups`,
  `drafts`, `customers`, `printing`, `cash`, `suppliers`, `reports`.
- The hub keeps the 5 regression tests and re-exports every handler on
  its historic `baraka_commands::ipc::<command>` path, so the
  `generate_handler!` list in `src-tauri/src/lib.rs` is untouched.
  Domain modules are re-exported by glob so new commands (plus their
  `#[tauri::command]`-generated `__cmd__*` items, which
  `generate_handler!` resolves beside each handler) flow automatically;
  `pub(crate)` helpers are never elevated. The kernel (`AppState`,
  `app_health` + its macro items) stays an explicit list.
- 12 kernel helpers and 3 display helpers
  (`sales::parse_document_language`,
  `drafts::discounted_unit_price_text`,
  `drafts::parse_optional_int`) become `pub(crate)`; everything else
  keeps its visibility. Test-only hub imports are `#[cfg(test)]` so
  normal builds carry zero dead imports.

## Equivalence proof

- 100/100 handler paths resolve (`cargo check` green, no lib.rs change).
- Bodies match the pre-split file modulo `pub(crate)` on 17 helpers
  and 4 `cargo fmt` signature reflows; all 13 per-module `pub fn`
  inventories match their source ranges.
- The hub tests block is byte-identical to the pre-split tail.
- G1 `fmt --check` clean, G2 clippy `-D warnings` zero findings,
  G3 workspace suite green, 5/5 hub tests pass.

## Consequences

- New IPC commands land in their domain module plus one hub
  re-export line; the flat external path never changes.
- `stable_code_or` and `to_unit_dto` stay private inside `common`
  (single-user helpers are not re-exported).
