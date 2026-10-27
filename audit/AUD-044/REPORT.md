# TASK-AUD-044 REPORT — Line-discount application path (F5 remise ligne)

- Status: PROVEN (not ambiguous — no handoff trigger). Read-only: no code,
  config, DB, build, or test-command changes. No cargo/npm/node executed.
- Verdict in one line: the F5 line discount is a **flat centime amount
  subtracted from the line gross** (`gross − discount`); the unit price is
  never touched at any layer (cart, wire, kernel, storage, receipt).

## 1. F5 path lines (keyboard → modal → store → wire → IPC → use case)

- `src/hooks/use-pos-keyboard.ts:207-213` — `if (event.key === "F5") {
  event.preventDefault(); if (!modalOpen && cart.lines.length > 0) {
  ui.openModal("discount"); } }`.
- `src/components/pos/modals/DiscountModal.tsx:1-3` — header: "Discount
  modal (F5): global discount editor plus selected-line discount editor.
  Amounts typed in DZD, stored as centime strings."
  `:37-52` — `apply()` parses both fields with `parseMoneyToCentimes`,
  rejects negatives, calls `setGlobalDiscount(global)` + `setLineDiscount(
  selected.key, line)`; i.e. both discounts are **amounts**, no rate
  anywhere. Labels: `discount.title` "Remise (F5)", `discount.line`
  "Remise ligne (DZD)" (`src/lib/i18n.ts:749-752`; AR `:1882-1884`).
- `src/stores/cart-store.ts:187-193` — `setLineDiscount: (key,
  discountCentimes)` stores the raw centime string on the line; `:194`
  `setGlobalDiscount` likewise doc-level.
- `src/components/pos/modals/CheckoutModal.tsx:104-115` —
  `buildLineWires` maps each line to `{ product_id, unit_id,
  quantity_micro, unit_price_centimes: null, line_discount_centimes:
  line.lineDiscountCentimes }` — "identical shape for preview and commit";
  the stored amount passes through verbatim, and the frontend never sends
  a price (override is `null` here).
- `crates/commands/src/ipc.rs:510-527` — `parse_cart_line_dto` (shared by
  `sales_checkout` + `sales_create_draft`): `unit_price_override` from
  optional wire, `line_discount_centimes: parse_wire_int(...)` required.
  `sales_checkout` itself `:1443-1470` repeats the same two fields.
- `crates/application/src/use_cases/sales.rs:70-129` —
  `verify_and_price_cart` (doc comment `:60-67`: "Single source of truth
  for sale totals (TASK-BRK-031): the read-only preview
  ([`PreviewSaleTotalsUseCase`]) and the atomic commit
  ([`CheckoutSaleUseCase`]) both travel this exact path"). `:101-112`
  builds `PricedLineInput { quantity_micro, unit_price_centimes:
  line.unit_price_centimes, line_discount_centimes: raw.line_discount_
  centimes, … }` — discount carried raw alongside the verified price.
  Draft path `:749-755` identical.
- CORRECTION to the task's parenthetical: **F6 is not the global
  discount.** `use-pos-keyboard.ts:214-220` F6 → `ui.openModal("parked")`;
  `src/lib/i18n.ts:791` `key.f6` = "F6 Reprendre" (park/retrieve). The
  global discount is edited in the same F5 `DiscountModal`
  (`discount.global` "Remise globale (DZD)") and is likewise a flat
  doc-level amount, allocated pro-rata — see §5.

## 2. Arithmetic quotes (discount hits the line total, never the unit price)

- `crates/domain/src/sales/pricing.rs:24-25` — `PricedLineInput`:
  "Line discount in centimes (`<=` gross)." (an amount bounded by the
  gross — not a rate).
- `pricing.rs:49-57` — `price_line_gross(quantity_micro,
  unit_price_centimes)`: `Money::from_centimes(unit_price_centimes).
  checked_mul_qty(...)` = `round(qty × price)`.
- `pricing.rs:304-315` (`price_document`, "Gross + line discounts"):
  `let gross = price_line_gross(line.quantity_micro,
  line.unit_price_centimes)?; if line.line_discount_centimes > gross {
  return Err(LineDiscountExceedsGross); } gross_lines.push((gross, gross -
  line.line_discount_centimes));` — **the rate hits nothing; the amount
  is subtracted from the gross.**
- `pricing.rs:32-47` — `PricedLine` carries `line_gross`,
  `line_after_discount`, allocated share, taxable/tva/ttc — **no
  discounted-unit-price field exists.**
- `sales.rs:320-343` (checkout persistence): "Stored discount is the line
  discount plus the allocated global share."
  `unit_price_centimes: line.unit_price_centimes` (raw catalog/tier/
  override price, unchanged) + `discount_centimes:
  raw.line_discount_centimes.saturating_add(allocated)` +
  `line_ht/line_tva/line_ttc` from the kernel. Draft `:805-820` same
  (discount without the global share).
- `VerifiedLine` (`sale_pricing_service.rs:46-…`): `unit_price_centimes`
  = `override_price.unwrap_or_else(|| unit.price_for_tier(tier))` —
  catalog/tier/override only; discount is not an input to price agreement.
  `calculate_margin_basis_points(unit_price, cost_price)` therefore guards
  on the pre-discount price.

## 3. Consumer lines (re-derive, never trust; receipt transcribes storage)

- Totals: `price_document` outputs are the single truth consumed by both
  preview and commit receipts; `PricedDocument.total_ttc = subtotal_ht +
  total_tva + timbre` (`pricing.rs:370-373`).
- Storage: `document_items` (`0001_baseline_schema.sql:199-217`) keeps
  `unit_price_centimes` (raw) + `discount_centimes` (line + allocated
  global) + derived `line_ht/tva/ttc_centimes`. No discounted-PU column.
- Reprint: `print_worker_service.rs:88-93` transcribes storage verbatim:
  `unit_price_text: format_centimes_plain(item.unit_price_centimes)` (raw
  PU — discount invisible on the PU), `total_text:
  format_centimes_plain(item.line_ttc_centimes)`. `receipt_model.rs:30`:
  "One printable item row (transcribed, never recomputed)." Rasterizer
  `:69-76`: `{qty} x {unit_price} = {total}` per line + doc-level
  "Remise" total (`:82-84`); per-line discount is never printed.
- Adjacent consumer: returns pro-rate the stored lump discount —
  `returnLineDiscountCents(originalDiscountCents, qtyMicro,
  originalQtyMicro) = (discount × qty + origQty/2) / origQty`
  (`src/stores/return-modal-store.ts:58-67`), i.e. returns also assume
  total-application.

## 4. Test pins (all pin total-application; none derives a discounted PU)

- `pricing.rs:569-580` `empty_cart_and_bad_discounts_rejected`:
  `input(10_000, 1_000, 2_000, …)` → `Err(LineDiscountExceedsGross)` —
  pins the `discount <= gross` bound.
- `pricing.rs:534-567` `full_document_prices_end_to_end`: lines
  `input(20_000, 10_000, 0, …)` + `input(10_000, 5_000, 500, …)`; comment
  "Gross: 20,000 + 4,500 after line discount" — asserts allocation sum,
  HT sums, `total_ttc = HT + TVA + timbre`. No PU assertion post-discount.
- `crates/infrastructure/tests/sale_pricing.rs:127-…`
  `priced_document_invariants_hold`: qty 3.0 × 12,000 = 36,000 gross with
  `line_discount_centimes: 1_000` (flat amount); asserts only sums
  (allocation = 2,500, HT/TVA/TTC identities).
- `crates/infrastructure/tests/sale_preview_commit_parity.rs:307-…`
  `preview_equals_commit_with_discounts_bl_no_timbre`: comment "A:
  2 x 100.00 − 10.00 = 190.00 after line discount. B: 1 x 50.50 = 50.50.
  Global discount 40.50 over bases [190.00, 50.50] → allocated
  [32.00, 8.50], HT = 158.00 + 42.00 = 200.00." Commits
  `line_discount_centimes: 1_000` on a 20,000 gross; asserts preview ==
  commit byte-for-byte in centimes. Unit price 10,000 never re-asserted.
- Frontend pins use `"0"` discounts only
  (`tier-pricing-cockpit.test.tsx:43`, `multi-unit-cockpit.test.tsx:55,67`,
  `checkout-double-submit.test.ts:33`) — no TS test pins discount
  application at all. **No test anywhere computes or asserts a discounted
  unit price.**

## 5. Change surface + edge cases (for BRK-096 unit-price application)

- SEMANTIC DECISION REQUIRED (architect): the modal takes an **amount**
  (DZD). "Apply to the unit price" needs one of: (a) amount-per-unit
  input (÷ qty — lossy, qty in micros); (b) keep amount input, convert
  to per-unit inside the kernel (`PU' = PU − round(amount/qty)` — lossy);
  (c) change the modal to a **rate %** (`PU' = round(PU × (1−rate))` —
  clean, matches "remise" shop usage). Recommend (c); (a)/(b) cannot
  represent 10.00 off 2×100.00 exactly per unit.
- Kernel: `price_document` (`pricing.rs:304-315`) must derive the
  discounted PU and recompute `gross' = round(qty × PU')`, keeping
  `LineDiscountExceedsGross` meaningful (on rate or on derived amount);
  `PricedLine` needs a new discounted-PU field (nothing carries it today).
  Canonical rounding order must be fixed in the kernel (per-unit-round ×
  qty vs round(qty × PU') diverge — currently only one rounding exists).
- Use cases: `verify_and_price_cart` (`sales.rs:101-112`) + draft path
  (`:749-755`); `CartLineInput`/`PricedLineInput`/DTOs change shape only
  if the wire switches amount→rate (`dto.rs:580`,
  `ipc.rs:512-527,1443-1470,2701-2703` — checkout, draft, and one more
  call site at `:2701`); `PricedCheckoutLine` + `DocumentItem`
  (`sales.rs:327-343, 805-820`).
- Storage: `document_items` has no discounted-PU column — either a new
  append-only migration (checksums; data-preservation test on a copy) or
  re-derive at read. Old documents (lump discount, no PU') vs new
  documents (derived PU') need dual-path rendering or backfill rule.
- Frontend: `DiscountModal` (rate vs amount input + validation),
  `cart-store` (`lineDiscountCentimes` semantics, `previewLineTotal`
  `:244-253`, `previewTotals`), `CartTable` PU column `:239` must show
  the discounted PU, `CheckoutModal.buildLineWires` (`:104-115`).
- Print: `print_worker_service.rs:88-93` `unit_price_text` source;
  `ReceiptLine`/`ReceiptTotals` (`receipt_model.rs:33-…`); 80mm +
  graphique templates share the model (change once, verify twice).
- Edge cases: (1) rounding per-unit vs per-line (micros quantities
  amplify the divergence — define canonical order in the kernel);
  (2) TTC display — TVA base is unchanged numerically iff gross'
  equals today's after-discount gross; pin it; (3) reprint fidelity —
  old docs must render exactly as today (transcribed lump discount),
  new docs from stored PU'; (4) Margin Guard currently uses the
  pre-discount price — decide whether below-cost detection moves to the
  discounted PU; (5) tier pricing (`price_for_tier`) + `PriceOverride`
  interplay — discount applies after tier resolution, and must not become
  a backdoor override (permission gate stays); (6) global allocation
  bases are after-line-discount grosissants — unchanged numerically
  under (c) iff gross' matches, but re-pin the parity test; (7) returns
  `returnLineDiscountCents` pro-rating assumes a lump sum — revisit
  under per-unit semantics; (8) kit headers (`is_kit`) expanded
  downstream of pricing.
- Alternative noted, not recommended: route the discounted PU through
  the existing `unit_price_override` wire — it is `PriceOverride`-
  gated and stored as the price, which would corrupt price-agreement
  semantics and margin snapshots.

## 6. git status --short

```text
?? audit/AUD-044/
```

## 7. STATE / DONE / REMAINING

- STATE: DONE (PROVEN — application point is the line total at every
  layer; nothing ambiguous, no probe needed).
- DONE: F5 path, arithmetic quotes, consumer chain, test pins, F6
  correction, UI display, change surface + 8 edge cases, REPORT written.
- REMAINING: none — blocks BRK-096 as specified (needs the §5 semantic
  decision first).
- Files: `D:/Dev/Baraka UltraPOS/audit/AUD-044/REPORT.md` [CREATED].
  No other files touched; no scratch/scripts created (nothing to delete).

NEXT READY: YES.
