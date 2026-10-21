# TASK-DSN-082 REPORT — Reason + typed confirmation in delete modals; DEP-01/stocked-depot blocking

## (1) FAIL-before (quoted, 7/7 new pins red, 10/10 legacy green)

```
src/components/common/safe-delete-modal.test.ts (17 tests | 7 failed)
  × DSN-082 … > collects a mandatory reason with a live counter and a 3-char minimum
  × DSN-082 … > gates confirm on an exact case-sensitive typed match of the entity code
  × DSN-082 … > passes the collected reason into onConfirm and every caller forwards it
  × DSN-082 … > threads reason through all four IPC delete wrappers
  × DSN-082 … > maps engine delete codes to guidance in both locales
  × DSN-082 … > announces modal errors assertively and traps Tab across textarea inputs
  × DSN-082 … > never renders a delete affordance for the locked principal depot
```

## (2) Wiring list (production, `src/` only — engine frozen)

- `src/components/common/SafeDeleteModal.tsx`: `onConfirm: (reason: string) =>
  Promise<DeleteEntityResult>`; reason textarea (`del.reasonLabel/Hint`, live
  `n / 3` counter, `MIN_REASON_LENGTH = 3` mirroring the engine rule); typed gate
  (`del.typeToConfirm` + LTR-isolated code badge, `typedOk = typedInput ===
  expectedToken`, no case folding — pin asserts `toLowerCase`/`toUpperCase`
  absent; token = entityCode else entityName); confirm gated on
  `busy || blocked || !reasonOk || !typedOk`; error `role="alert"` with
  `INVALID_INPUT → del.reasonTooShort`, all other codes via
  `translateBackendError`; `textarea` added to the Tab-trap selector.
- `src/lib/ipc.ts`: `ipcDeleteProduct/Customer/Supplier/Warehouse` take
  `reason: string` and send `reason` (+8/−4, CRLF-clean).
- 5 call sites: `CatalogModule`, `CustomersModule`, `SupplierListScreen`
  (`onConfirm={(reason) => ipcDelete…(token, id, reason)}`);
  `warehouse-transfer-store.submitDeleteWarehouse(id, reason)` +
  `WarehouseTransferModal` forward; `StarterPackBanner` compat
  (`UninstallDeps.remove(token, id, reason)`, `uninstallStarterPack(…, reason =
  "starter-pack uninstall")`, banner pipes the modal reason via ref) +1
  reason-threading pin in `starter-pack.test.ts`.
- `WarehouseTransferModal`: `isBlocked={deleting.code === PRINCIPAL_DEPOT_CODE}`;
  locked DEP-01 rows never render save/delete (`{!locked && …}` — pinned).
  Non-empty depots: no client-side emptiness oracle exists without an engine
  change (out of scope) — `WAREHOUSE_HAS_STOCK` stays engine-enforced and now
  maps to FR/AR guidance instead of a raw code.
- `src/lib/i18n.ts` (+16, CRLF-clean): `del.reasonLabel/Hint/typeToConfirm/
  reasonTooShort` FR+AR; `CUSTOMER/SUPPLIER_BALANCE_NONZERO`,
  `WAREHOUSE_HAS_STOCK`, `DEPOT_PRINCIPAL_IMMUTABLE` in `frErrors`+`arErrors`.
- Typed-gate rule (documented): exact match on entity code, case-sensitive
  (codes are machine identifiers: `VIS-8X40` ≠ `vis-8x40`); falls back to
  entityName when no code (customers without phone).
- Uniform typed gate on every delete modal (no per-mode tier): the frontend
  cannot know the engine's purge-vs-deactivate decision upfront, so every
  delete is treated as potentially irreversible.

## (3) Gates

| Gate | Result |
|------|--------|
| G1 `cargo fmt --check` | exit 0 |
| G2 `cargo clippy --all-targets --all-features -- -D warnings` | 0 errors |
| G3 `cargo test --workspace -j 2` | EXIT=101; only reds = 2 pre-existing BRK-076 `migration_replay_and_data_preservation.rs:462` (`no products_fts table…`); 33 suites ok |
| G4 `npm run lint` | exit 0, no errors |
| G5 `tsc --noEmit` | clean (after StarterPack `UninstallDeps` compat) |
| G6 `npm run build` | ✓ built in 6.42s |
| G7 `npx vitest run` | 73 files / 727 tests passed (incl. 17/17 safe-delete-modal, starter-pack reason pin) |

## (4) PNGs (`audit/DSN-082/`, all rows asserted + `noPageScroll` + 0 console errors)

- `del-typed-1280x800-fr-light.png` (90574 B) — `Supprimer le produit` modal:
  `Motif de suppression` + counter `20 / 3` + `doublon de catalogue`,
  `Tapez le code… : VIS-8X40` typed, `Confirmer la suppression` enabled red.
- `del-blocked-1280x800-ar-dark.png` (51054 B) — RTL dark manage tab:
  `مقفل (DEP-01)` lock badge on `Dépôt Central`, no delete affordance on
  DEP-01 row, `حذف` only on AUX-01; KPIs live (`المستودعات النشطة 2`);
  vision-verified, no stray error text (stub gap on
  `reports_stock_movement_history` shape fixed mid-run).
- `del-typed-1024x600-fr-light.png` (85089 bytes) — typed gate at 1024×600.
- `capture.mjs` (harness, BRK-079 CDP pattern) + `capture-results.json`
  (`DONE fails=0`).

## (5) Hygiene

- Lockfile/manifest churn: `git diff --stat -- pnpm-lock.yaml package-lock.json
  yarn.lock package.json` → empty (exit 0). Zero package-manager commands run
  (vite launched via `node_modules/.bin/vite` directly; dev server killed,
  `PORT 5173 CLOSED` verified; capture-scripts deleted? No — `capture.mjs` is
  a DELIVERABLE under `audit/DSN-082/`, kept. No scratch files remain).
- No live DB opened (tempdir only); no new dependencies; no `src/crates`
  edits beyond SCOPE-IN. DO NOT COMMIT.
