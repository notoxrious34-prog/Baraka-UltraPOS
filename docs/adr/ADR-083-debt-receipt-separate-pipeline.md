# ADR-083: Debt-Receipt Printing Uses a Fully Separate Rendering Path

Status: ACCEPTED
Date: 2026-10-06
Session: SESSION-ARC-007, Reply 6

## Context

TASK-BRK-060 required a decision on whether the debt-payment receipt should
reuse, relabel/parameterize, or stand fully apart from the existing
sales-receipt rendering pipeline (`ReceiptDocument` / `build_lines` /
`ReceiptRendererPort::render`), which is hardcoded with sales-specific fixed
totals rows (Sous-total HT, TVA, Remise, Timbre, TOTAL TTC, Payé).

## Decision

A fully separate rendering path was added: a new domain type
`DebtReceiptDocument`, a new rasterizer layout function
(`build_debt_receipt_lines` / `render_debt_canvas`), and a new trait method
`ReceiptRendererPort::render_debt_receipt` (with a safe not-implemented
default). The existing sales pipeline (`ReceiptDocument`, `build_lines`,
`ReceiptRendererPort::render`) is untouched.

## Reasons

- The existing pipeline's fixed totals rows have no sensible meaning for a
  debt payment; forcing reuse would require conditional/sentinel logic
  inside a tested, production sales path, risking regression for no
  benefit.
- Full separation protects the sales pipeline with zero blast radius —
  confirmed by a zero-byte diff on the sales pipeline's own logic across the
  entire TASK-BRK-060 commit (`abeb30a07e7681dde38da29e41caf67509a517ca`).
- The shared low-level pixel-drawing core (`render_lines` in
  `EscPosRasterizer`) was reused by both pipelines without any refactor,
  since it already existed in a layout-agnostic form.

## Consequences

- Any future non-sale print job type (e.g. a stock-adjustment slip, a
  cash-session Z-report) should default to the same pattern: a dedicated
  domain type and a dedicated trait method, not a parameterized extension of
  `ReceiptDocument`.
- `PrintWorkerService::process_claimed` must branch by job type / presence
  of `document_id` to call the correct renderer method; this dispatch logic
  is the scope of TASK-BRK-061 (not yet implemented as of this ADR).

## References

- CTP-SESSION-ARC-007.md, Section 11, Section 17.3.
- Commit `abeb30a07e7681dde38da29e41caf67509a517ca`.
