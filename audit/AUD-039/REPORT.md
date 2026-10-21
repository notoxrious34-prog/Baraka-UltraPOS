# TASK-AUD-039 REPORT — deletion matrix (read-only)

Zero builds started; file reads + ripgrep only (~10 min CPU). Detail: MATRIX.md.

## (1) Six one-line verdicts
1. Products: **SAFE** — dual-track (`deletion.rs:68-93`), SafeDeleteModal + mode note.
2. Warehouses: **INCONSISTENT** — engine guards (DEP-01 immutable, nonzero-stock
   refuse), but UI offers delete then fails post-confirm (`isBlocked={false}`).
3. Customers: **SAFE** — nonzero-balance hard refuse + pre-confirm blocked guidance.
4. Documents: **FORBIDDEN-BY-DESIGN** — no delete path exists anywhere; AVOIR only.
5. Suppliers: **SAFE*** — dual-track OK, but no balance guard (*verify purchase-debt exposure).
6. Users: **SAFE** — deactivation-only + `LAST_ADMIN_PROTECTED`, no delete affordance.

## (2) Gaps with proposals + sizes
- G1 engine-M: no audit event / no reason collected on any delete (`deletion.rs`
  zero audit refs). Proposal: add `reason` param + emit audit in-tx
  (files: `deletion.rs`, `ports.rs:80` audit port, 4 adapters).
- G2 frontend-S: no typed gate for irreversible hard deletes (ADR-074).
  Proposal: `requireText` prop on `SafeDeleteModal` (`common/SafeDeleteModal.tsx`),
  enforced when `mode` will be `hard_deleted`; +FR/AR keys (4 call sites).
- G3 frontend-S: depot modal offers DEP-01/stocked deletes. Proposal: pre-check
  (code==DEP-01 const exists `:40`; stock>0 via existing levels query) →
  `isBlocked` + reason before confirm (`WarehouseTransferModal.tsx:105-213`).
- G4 engine-S: supplier balance exposure unverified. Proposal: read purchase-debt
  path; if balances exist, mirror `CUSTOMER_BALANCE_NONZERO`
  (`deletion.rs:160-186`).

## (3) git status --short
Only `?? audit/AUD-039/` added (REPORT.md + MATRIX.md). No src/crates touched.

## (4) STATE/DONE/REMAINING
STATE: complete / DONE: REPORT.md + MATRIX.md / REMAINING: none.
