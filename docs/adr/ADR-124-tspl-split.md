# ADR-124: Split `tspl.rs` into 8 domain modules, pure motion (TASK-BRK-112)

Status: ACCEPTED.

## Context

`crates/domain/src/printing/tspl.rs` (4135 lines) was the last domain
monolith after the BRK-106/107 splits. Same scale problem, same
prescription: mirror the print pipeline stages so each module speaks one
vocabulary (ADR-108/109 standard).

## Decision

`tspl.rs` → `tspl/` hub plus 8 modules on the pipeline map:

- `probe` (probe switch + lossy fold), `media` (setup, validators,
  geometry consts, self-describing preamble), `common` (crate-internal
  font/Code128/TEXT helpers), `ticket_model` (input structs + captions),
  `ticket_plan` (v1 + v2 geometry), `label` (40x20 engine),
  `ticket_render` (80 mm emitters + ruler), `bitmap` (raster jobs).
- The hub keeps the module docs and the full `mod tests` block
  byte-identical; `pub mod tspl` + the explicit `pub use tspl::{...}`
  list in `printing/mod.rs` are untouched, so no downstream crate changes.
- Visibility deltas (crate-internal only, zero public API change):
  `Captions` + fields and `TicketFace::{face_str,scale,width}` become
  `pub(crate)`; `LABEL_SAFE_*` drop from `pub` to private (label-only);
  shared private consts/helpers centralize in `common` as `pub(crate)`;
  test-only hub pins (`DomainError`, 7 geometry consts) are `#[cfg(test)]`.

## Equivalence proof

- `cargo check` zero errors/warnings; `fmt --check` clean;
  `clippy --all-targets --all-features -D warnings` zero findings.
- `baraka-domain --lib printing::`: 56/56 pass (all TSPL goldens
  byte-identical — pure motion proven by the pins themselves).
- `tspl_print_tests` (application): 25/25 pass.
- Full workspace G3: 662 passed / 0 failed.

## Consequences

- New print code lands in its stage module plus the hub `pub mod` line;
  the flat `printing::tspl::*` path never changes.
- `common` stays `pub(crate)`-only: shared geometry is not a public API.
