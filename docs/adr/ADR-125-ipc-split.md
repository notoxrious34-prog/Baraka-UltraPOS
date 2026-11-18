# ADR-125: Split `src/lib/ipc.ts` into 12 domain modules, pure motion (Wave 3 frontend twin)

Status: ACCEPTED.

## Context

`src/lib/ipc.ts` (3372 lines) was the last frontend monolith after the
BRK-106/107/112 backend splits. Same scale problem, same prescription:
mirror the call path so each module speaks one domain vocabulary. 104
consumers import the flat `@/lib/ipc` path — that path is frozen.

## Decision

`ipc.ts` → hub plus 12 modules on the domain map:

- `ipc-kernel` (transport edge: invoke/session injection + shared
  `DeleteEntityResult`), `ipc-auth`, `ipc-catalog`, `ipc-inventory`,
  `ipc-users`, `ipc-sales` (stock, checkout, preview, drafts, conversions,
  document search/detail, returns), `ipc-customers`, `ipc-printing` (outbox,
  silent TSPL, settings, fiscal, identity, diagnostics, vertical profile),
  `ipc-cash`, `ipc-suppliers`, `ipc-reports`, `ipc-backups`.
- The hub is a barrel (`export *` x12 + `export type { Locale }`); the flat
  `@/lib/ipc` path is untouched, so zero consumer churn.
- Visibility deltas (additive only, one name): `call` becomes exported
  (modules need it through the hub); `DeleteEntityResult` moves to the
  kernel (shared by catalog/inventory/customers/suppliers deletion paths).
- 12 source-text pin tests (they `readFileSync` the bridge source) now read
  the hub + all 12 modules concatenated: same pins, new homes, zero
  assertion changes.

## Equivalence proof

- Command-string multiset identical (117 entries incl. duplicates),
  `call<` sites identical (111).
- `export` name set identical except the deliberate additive `call`.
- `npm run lint` EXIT 0, `typecheck` EXIT 0, `vitest` 98 files / 983 tests
  pass, `npm run build` EXIT 0 (6.05s).
- Rust gates unaffected (zero Rust files touched): G1–G3 carried over from
  BRK-112 on the same tree.

## Consequences

- New IPC wrappers land in their domain module; the flat `@/lib/ipc` path
  never changes. Adding a 13th domain means extending the 12 pin-test file
  lists (one line each).
- Source-text pins remain the contract guard: any wrapper retarget still
  fails the suite first.
