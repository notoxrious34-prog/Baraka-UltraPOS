# TASK-BRK-039 — Payment Overpayment / Change Attribution Investigation

**Status:** fact-finding only — no code changed, no fix proposed.
**Observed trigger:** sale TICK-2026-000005 — TTC 155.50 DZD, Espèces = 155,
CIB/Dahabia = 155.50, displayed "Monnaie à rendre: 155,00 DZD", committed OK.

All line numbers below are from the current HEAD codebase.

---

## 1. Total tendered — exact expression

**File:** `src/components/pos/modals/CheckoutModal.tsx:154-158`

```ts
const cash = toCentimes(cashText);        // :154
const credit = toCentimes(creditText);    // :155
const cheque = toCentimes(chequeText);    // :156
const card = toCentimes(cardText);        // :157
const tendered = addBig(addBig(addBig(cash, credit), cheque), card);  // :158
```

`toCentimes` (`:42-51`) maps blank → `"0"`, otherwise `parseMoneyToCentimes`
(BigInt centimes, no floats). `tendered` is the plain four-way sum
Espèces + Crédit + Chèque + CIB — **Crédit IS included in the sum**, not
handled separately. This local `tendered` is used ONLY as a confirm-gate
(`tendered !== "0"`, `:184`); it is never displayed and never persisted.

The authoritative tendered figure is backend-computed (see §4).

Wire construction — **File:** `src/components/pos/modals/CheckoutModal.tsx:89-109`
(`buildPaymentWires`): one wire line per non-zero field, verbatim amounts,
no caps, no adjustment:

```ts
if (cash !== "0")   payments.push({ payment_method: "CASH",   amount_centimes: cash });
if (credit !== "0") payments.push({ payment_method: "CREDIT", amount_centimes: credit });
if (cheque !== "0") payments.push({ payment_method: "CHEQUE", amount_centimes: cheque });
if (card !== "0")   payments.push({ payment_method: "CARD",   amount_centimes: card });
```

## 2. "Monnaie à rendre" — exact formula

The modal performs **zero change arithmetic**. The displayed value is the
backend preview field, rendered only — **File:**
`src/components/pos/modals/CheckoutModal.tsx:589-596`:

```tsx
<span>{t(locale, "checkout.change")}</span>
<span>{fmt(preview?.change_amount_centimes)}</span>
```

`fmt` (`:391-392`) is `formatMoney` display formatting only.

The backend formula — **File:**
`crates/domain/src/sales/pricing.rs:200-236` (`settle_payments`):

```rust
let mut paid: i128 = 0;
let mut non_cash: i128 = 0;
for payment in payments {
    ...
    paid += amount;
    if !payment.method.is_cash() { non_cash += amount; }  // :213-217
}
if paid < total { → change = 0, remaining_due = total - paid }   // :220-227
if non_cash > total { return Err(ChangeNotPermittedForNonCash); } // :228-230
→ change = paid - total, remaining_due = 0                        // :231-235
```

Answer: **change = total_tendered − TTC unconditionally** (single surplus
pool), subject to ONE guard: the non-cash subtotal may not exceed the total.
There is NO `change = cash − remainder_after_non_cash` split — cash and
non-cash surpluses are pooled. `is_cash()` is true for `Cash` only
(`crates/domain/src/sales/payment.rs:46-49`); CHEQUE/CARD/CREDIT are non-cash.

Applied to the observed scenario (centimes): paid = 15500 + 15550 = 31050,
total = 15550, non_cash = 15550 ≯ 15550 → accepted,
change = 31050 − 15550 = **15500 (155,00 DZD)** — exactly what was displayed.

## 3. Caps on non-cash fields (Chèque / CIB)

**NO SUCH VALIDATION EXISTS — frontend.** Searched
`src/components/pos/modals/CheckoutModal.tsx` (full 639-line read) and
`src/components/pos/modals/checkout-submission.ts` (lock + id only, no
amounts): the only per-field check is `isValidAmount` (`CheckoutModal.tsx:53-63`,
parse-only), and the only confirm-gates are `tendered !== "0"` (`:184`),
`creditBlocked` for guests (`:159,183`), preview validity, and the stock gate
(`:180-190`). No max/remaining/exceed comparison against TTC exists for ANY
tender field. A card/cheque field can freely exceed the remaining balance
up to the backend rule in §2.

**Backend cap (partial):** `non_cash > total → ChangeNotPermittedForNonCash`
(`pricing.rs:228-230`). So a card/cheque line may cover AT MOST the full
total — never more — but may equal it while cash simultaneously overpays
(the observed shape). This rejection surfaces in the modal as `previewError`
(confirm blocked) and identically at commit.

## 4. Backend re-validation (does Rust trust the frontend?)

**The backend never trusts frontend totals.** `CheckoutSaleUseCase::execute`
(`crates/application/src/use_cases/sales.rs:185-357`) re-derives everything
through the shared kernel `verify_and_price_cart` (`sales.rs:68-123` →
domain `price_document`, `pricing.rs:266-354`) — the same path the read-only
`PreviewSaleTotalsUseCase` travels (single source of truth, TASK-BRK-031).
The persisted `CheckoutCommand.totals` (`sales.rs:340-348`) are the kernel's
`paid_total/change/remaining_due`, not frontend values. Additional
independent gates at this layer (`sales.rs:225-273`): credit-leg summation,
`CREDIT_CUSTOMER_REQUIRED`, `CreditDisabled`, `CreditLimitExceeded`, and
`tender coverage: remaining_due > 0 && credit == 0 → InsufficientPayment`
(`:268-273`). IPC mapping (`crates/commands/src/ipc.rs:969-977`) parses each
tender via `PaymentMethod::from_code` + `parse_wire_int` with no capping;
DTO shape (`crates/commands/src/dto.rs:492-499`, `PaymentLineDto`) is
`payment_method` + `amount_centimes` strings (Section 7 canonical).

## 5. `payments` table persistence — raw or adjusted?

**Raw entered values, verbatim.** **File:**
`crates/infrastructure/src/adapters/sqlite_checkout_repository.rs:379-409`:

```rust
for payment in &command.payments {
    ...
    tx.execute(
        "INSERT INTO payments
         (id, sequence_no, document_id, customer_id, amount_centimes,
          payment_method, cash_session_id, created_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            ...,
            payment.amount_centimes,   // :398 — raw, unadjusted
            payment.method_code,       // :399 — raw
            if payment.method_code == "CASH" { Some(session) } else { None }, // :400-404
            ...
```

No capping, netting, or change-deduction is applied to payment rows.
Document header stores kernel totals (`:269-301`: `ttc_centimes`,
`paid_amount_centimes`, `change_amount_centimes`, `payment_method` derived
`:264-268` as `NONE`/single-code/`MIXED`).

Cash drawer leg (`:410-429`):

```rust
let net_cash_delta: i128 = cash_tendered - i128::from(command.totals.change); // :410
if net_cash_delta < 0 { return Err("INVALID_PAYMENT_STATE"); }                // :411-413
if net_cash_delta > 0 { INSERT INTO cash_movements ... 'SALE_CASH_IN' ... }    // :414-429
```

Only CASH rows bind the session; CARD/CHEQUE rows carry `cash_session_id = NULL`.

## 6. Summary table

| Question | Current Behavior (evidenced) | File:Line |
|---|---|---|
| Is total tendered capped to TTC before computing change? | No. `paid` is the raw sum; change = paid − total, gated only by `non_cash ≤ total`. | `pricing.rs:204-236` |
| Can CIB/Chèque exceed remaining balance without rejection? | Partially. A non-cash subtotal may equal but never exceed the total (`ChangeNotPermittedForNonCash` otherwise); no frontend cap exists at all. | `pricing.rs:228-230`; `CheckoutModal.tsx:53-63,180-190` |
| Does backend re-validate payment split independently? | Yes. Kernel reprices from cart + tenders; frontend totals are display-only; commit carries kernel totals. | `sales.rs:216-223,340-348`; `pricing.rs:266-354` |
| Are raw entered amounts persisted verbatim to `payments`? | Yes. One row per tender with raw amount + method code; CASH rows bind session, others NULL. | `sqlite_checkout_repository.rs:379-409` |

## 7. Observed scenario — persisted state + consistency

For 155.00 cash (15500) + 155.50 CIB (15550) against TTC 15550:

- `payments` rows: `(CASH, 15500, session)`, `(CARD, 15550, NULL)` — raw values.
- `documents`: `paid_amount_centimes = 31050`, `change_amount_centimes = 15500`,
  `payment_method = 'MIXED'`.
- Consistency: SUM(payments) − TTC = 31050 − 15550 = 15500 = displayed change ✓.
- `cash_movements`: `SALE_CASH_IN = 15500 − 15500 = 0` → **no drawer row is
  written at all** (`:414` strict `> 0`; the `net_cash_delta < 0` invariant
  `:411` cannot fire here). Drawer running total is unaffected; the 155.50
  card leg settles externally. The state is arithmetically self-consistent
  (cash round-trips: 155 in, 155 out as change) **iff the 155.00 change is
  physically handed back** — the open commercial question (card charged the
  full ticket while cash also changed hands) is left for a follow-up decision;
  no fix is proposed here.

Note: with cash > 0 on a TICKET/FACTURE, `compute_timbre` adds 1 % timbre
(`pricing.rs:156-169,327-341`), so the TTC itself depends on cash presence.
No repository files were modified (`git status`: no `.git` repo present;
verification by zero-write discipline — investigation used reads only).
