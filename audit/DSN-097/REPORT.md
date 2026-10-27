# TASK-DSN-097 — Per-unit discount display (modal + cart + receipt lines)

## 1. SCOPE
- Display only: what the owner types per unit is what the cart shows and the ticket prints.
- Kernel/checkout/returns math frozen (BRK-096, `be8c3f8`); stored values read-only.
- Legacy `NULL` per-unit renders the old format exactly; numbers LTR-isolated.
- DO NOT COMMIT.

## 2. FAIL-BEFORE (quoted from prior report)
- `DiscountModal.tsx`: label `discount.line` ("Remise ligne (DZD)") — amount read as off-total, no per-unit copy, no live total, no bound hint.
- `cart-store.ts:245`: `previewLineTotal = max(0, qty*tierPrice - discount)` — flat off-total math.
- `CartTable.tsx:239`: price cell rendered `lineUnitPriceCentimes(...)` raw only — discounted PU never shown.
- `ipc.rs` `ReturnableLineDto:1826`: `net_unit_price = line_ht*10000/qty` — truncated display aid, no stored per-unit.
- `print_worker_service.rs:91`: `unit_price_text = item.unit_price_centimes` raw.
- `graphique-ticket.ts`: `unitPrice: item.unitPriceCentimes` raw.
- `DocumentDetailDrawer` + A4: raw `unit_price_centimes` only.
- `sales_get` / `ReturnableItemDto`: no `discount_per_unit` field on any display wire (extended with 2 nullable transcription-only DTO fields + worker text selection — accepted as designed, no recompute).

## 3. CHANGES (production)
- `crates/commands/src/dto.rs`: `DocumentItemDto` + `ReturnableItemDto` += `discount_per_unit_centimes: Option<String>` + `discounted_unit_price_centimes: Option<String>` (CRLF-safe, 11-line diff).
- `crates/commands/src/ipc.rs`: `discounted_unit_price_text()` helper (exact `unit_price - per_unit`, `None` legacy); `sales_get` + `documents_get_return_context` map both fields.
- `crates/application/src/services/print_worker_service.rs`: receipt `unit_price_text` = stored `unit_price - per_unit` when `Some`, raw when `None` (transcription only).
- `src/stores/cart-store.ts`: new `previewDiscountedUnitPrice()` (`max(0, PU - perUnit)`); `previewLineTotal()` = `qty x discountedPU`.
- `src/components/pos/modals/DiscountModal.tsx`: `discount.linePerUnit` + EACH-unit hint + live `qty x netPU = total` + `discount.exceedsUnitPrice` pre-wire gate (engine `LineDiscountExceedsUnitPrice` stays backstop). One-token class fix during gates: hint `text-slate-500` -> `text-ink-muted` (DSN-068 contrast).
- `src/components/pos/CartTable.tsx`: struck raw + emerald net PU when discounted, raw alone otherwise (`<bdi dir="ltr">`).
- `src/components/pos/modals/CheckoutModal.tsx`: `buildLineWires` per-unit comment (verbatim map) + passes per-unit fields into graphique args.
- `src/stores/return-modal-store.ts`: `computeReturnTotals` per-unit path (picked x net PU verbatim) / legacy lump pro-rating preserved.
- `src/components/pos/modals/ReturnModal.tsx`: struck raw + net PU + `(Remise /u: …)` badge; row preview prices net PU.
- `src/features/tickets/graphique-ticket.ts`: `TicketWireItem` += 2 nullable fields; `unitPrice = discounted ?? raw` verbatim.
- `src/features/documents/DocumentDetailDrawer.tsx`: details + A4 struck-raw/net + per-unit badge; legacy (`null` or equal) renders old format exactly.
- `src/lib/ipc.ts`: `DocumentDetailsWireResponse.items` + `ReturnableItem` += 2 nullable fields.
- `src/lib/i18n.ts`: FR/AR `discount.linePerUnit`, `linePerUnitHint`, `lineTotal`, `exceedsUnitPrice`, `returns.perUnitDiscount`, `documents.perUnitDiscount` (old `discount.line` key kept, unused by modal).

## 4. PINS (all green)
- `src/stores/__tests__/cart-store.test.ts` "per-unit discount display (DSN-097)": owner `3 x (1000-100)` -> net `"900"`, total `"2700"` (one-char fix `"27000"`->`"2700"` applied + re-run 15/15); plain line; corrupt over-discount floors at 0.
- `src/components/pos/modals/discount-modal.test.tsx` [CREATED] 11/11: per-unit label, EACH hint, live figure, over-price bound gate, global untouched + FR/AR keys (discount/returns/documents).
- `src/components/pos/modals/return-modal.test.tsx` "per-unit settlement": 1 of 3 x (1000-100) = 900 HT; legacy NULL lump path 900 (byte-identical).
- `src/features/tickets/graphique-ticket.test.ts`: wire net PU verbatim (`unitPrice "900"`, html `9,00 DZD`, no `10,00 DZD`); legacy null -> raw `1000` / `10,00 DZD`.
- `src/features/documents/document-print.test.tsx`: details + A4 struck raw + `dont remise /u` (FR) / `dir="ltr"` (AR); legacy null renders raw alone (fixed `<s` -> `<s `, React emits `<s class=`; `<section>` false positive).
- Fixture inserts (legacy null): `return-modal.test.tsx` (2), `document-detail-drawer.test.tsx` (5), `document-print.test.tsx` (1), `graphique-ticket.test.ts` (2).

## 5. CAPTURES (file:// bundle method — Chrome->loopback refused env-wide)
- Env block: headless Chrome 156 reaches internet but `ERR_CONNECTION_REFUSED` on all loopback (127.0.0.1/localhost/LAN) incl. bare node http; `--no-proxy-server`/`--no-sandbox` ineffective. Vite served (LAN 200 via node) but unreachable from Chrome.
- Workaround: `vite build` harness entry -> single self-contained `bundle.html` (1,580,445 bytes, JS+CSS+fonts base64-inlined; fixed blanket `</` escaping that corrupted a bundled regex) loaded via `file://`; CDP asserts + screenshots.
- `audit/DSN-097/discount-cart-fr-light.png` (30,086 B): modal `Remise (F5)`, `Remise ligne / unité (DZD)`, `Le montant s'applique à CHAQUE unité`, live `Total ligne estimé: 3 x 9,00 DZD = 27,00 DZD`, row total 27,00 (modal covers PU cells; in-page `<s>` assert true).
- `audit/DSN-097/discount-cart-row-fr-light.png` (12,611 B, bonus): unobstructed struck `10,00 DZD` + emerald `9,00 DZD`, qty 3, total `27,00 DZD` — pixel-verified.
- `audit/DSN-097/discount-ticket-ar-light.png` (13,332 B): RTL, `Quincaillerie El Baraka`, 1 line `VIS-001/برغي`, `قطعة 3`, `9,00 د.ج`, `27,00 د.ج` (NOT 10,00), HT/TVA/TTC 27,00/0,00/27,00 — pixel-verified.
- `audit/DSN-097/dsn097-capture-results.json`: 18/18 asserts PASS, consoleErrors 0.
- Note: bundle predates the one-token `text-ink-muted` class fix (text/asserts identical).

## 6. GATES
- G1 `cargo fmt --check`: EXIT 0.
- G2 `cargo clippy --all-targets --all-features -- -D warnings`: EXIT 0 (46.30s).
- G3 `cargo test --workspace`: EXIT 0 — 70 suites ok, 549 passed / 0 failed / 7 ignored (pre-existing).
- G4 `npm run lint`: EXIT 0.
- G5 `npm run typecheck` (`tsc --noEmit`): EXIT 0.
- G6 `npm run build`: EXIT 0 (7.68s).
- G7 `npx vitest run`: 82 files, 856 tests — 855 passed, 1 failed (`family2-contrast` vs `text-slate-500` in new hint line) -> fixed to `text-ink-muted` -> full re-run green (see §8; counts pasted from re-run log).
- G1-G3 run because Rust display-wire files were touched (`dto.rs`, `ipc.rs`, `print_worker_service.rs`).

## 7. STATUS
- Temp harness files: all deleted (see §8 strays proof); no servers left running (own PIDs only, killed/verified).
- `git status --short`: see §8 (dirty, DO NOT COMMIT).

## 8. EVIDENCE PASTE (filled at seal time)
- G1 `cargo fmt --check` EXIT 0 (`_g1.log`, deleted after pass).
- G2 `cargo clippy --all-targets --all-features -- -D warnings` EXIT 0 (Finished dev profile in 46.30s).
- G3 `cargo test --workspace` EXIT 0: 70 suites `test result: ok`, 549 passed / 0 failed / 7 ignored (pre-existing ignores).
- G4 `npm run lint` EXIT 0. G5 `npm run typecheck` EXIT 0. G6 `npm run build` EXIT 0 (built in 7.68s).
- G7 `npx vitest run` full re-run: `Test Files 82 passed (82)`, `Tests 856 passed (856)`, EXIT 0 (first run 855/856 — `family2-contrast` caught `text-slate-500` in the new hint line, fixed to `text-ink-muted`, re-run green).
- Captures: `discount-cart-fr-light.png` (30,086 B) + `discount-cart-row-fr-light.png` (12,611 B) + `discount-ticket-ar-light.png` (13,332 B); `dsn097-capture-results.json`: 18/18 PASS, consoleErrors 0. Vision-verified (struck 10,00 + net 9,00 + 27,00; modal EACH-unit hint; AR RTL 9,00/27,00 د.ج, no 10,00).
- Strays: `_cap097* _dbg* _inline* __cap097* vite.cap097* _g*.log _cap097-dist/ src/__cap097.tsx public/__cap097.html` all absent (`ls` + `grep -rln cap097 src public audit` clean except REPORT + results JSON); port 5173 ECONNREFUSED; no own-PID chrome listeners.
- `git status --short`: 18 modified (3 Rust display-wire + 10 frontend display + 5 existing test files, listed above) + `?? audit/DSN-097/` + `?? discount-modal.test.tsx`. 18 files changed, 472 insertions, 27 deletions. No commit.
- STATE: DONE. NEXT READY: YES.
