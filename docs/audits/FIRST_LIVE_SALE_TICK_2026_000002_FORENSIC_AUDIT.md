# FORENSIC AUDIT — First Live Sale TICK-2026-000002 (TASK-BRK-030)

> **Nature:** read-only forensic audit. The active database was opened
> read-only (`mode=ro` URI, no copy, no WAL interference) and never mutated.
> No source, migration, or data file was modified except this report.
> Secrets (hashes, tokens) are excluded throughout; only business-evidence
> identifiers are quoted.

---

## Section 1 — Audit Scope and Evidence Identity

- **Audited document:** `TICK-2026-000002`, id
  `cb7813b9-1c1f-40b8-a349-38935363eb70`, committed
  `2026-09-27T17:22:17.422Z` by operator `admin` (ADMIN, active;
  users.id `1fba125d-5ceb-458c-9e2b-5684c6c25e79`).
- **Database location (resolved, not assumed):** the production path from
  `src-tauri/src/lib.rs` (`app.path().app_data_dir()` ⋈
  `default_db_file_name()`) =
  `%APPDATA%\com.baraka.ultrapos\baraka.db` (schema version **12**;
  WAL active at read time; no workspace-root database exists).
- **Context:** warehouse DEP-01 (`01950000-…0001`, migration 0012 seed),
  terminal `39614fd1-…` (document `origin_device_id` == cash session
  `terminal_id`), cash session `7c973d09-…` (OPEN, float 0, opened 15:08:02Z),
  checkout command `483cfdb1-…` (`sales.checkout`, request hash
  `261c52b9…9b7f`).
- **Method:** direct read-only SQL over all 12 mandated tables + code-path
  verification in `pricing.rs`, `sales.rs`, `sqlite_checkout_repository.rs`,
  `sqlite_stock_repository.rs`, `CheckoutModal.tsx`. Owner screenshots are the
  external live evidence; this report is the database/code forensic counterpart.
- **Material context fact:** an identical twin sale `TICK-2026-000001`
  (id `7539c3a6-…`, same totals/payments) committed 30 s earlier (17:21:47Z)
  under a *different* command id (`afdc9575-…`). Both are analyzed where they
  interact (stock, cash); the verdicts below concern `…000002`.

## Section 2 — Document and Financial Totals

Persisted `documents` row (`TICK-2026-000002`, TICKET, FINALIZED, FR/FR):
`subtotal_ht=15050`, `global_discount=0`, `vat_total=0`, **`timbre=500`**,
`rounding=0`, **`ttc=15550`**, `paid_amount=30050`, `change_amount=14500`,
`payment_method=MIXED`, `revision=1`.

Single `document_items` row (`1e7c2a58-…`): product
`f20f6f21-…` (VIS-8X40), unit `78d65d76-…`, `quantity_micro=10000`,
`quantity_base_micro=10000` (= 1.0 base unit), `unit_price=15050`,
`discount=0`, `tva_rate=0`, `line_ht=15050`, `line_tva=0`, `line_ttc=15050`,
`cost_price=10000`.

Independent integer recomputation: `15050 + 0 + 500 + 0 = 15550` ✓;
item `1.0 × 15050 − 0 + 0 = 15050` ✓. **Every persisted total reconciles
exactly — zero centime discrepancy.**

## Section 3 — Timbre / Tax Rule Verification

- **Rule source:** pure code formula, not configuration — `tax_rules` table
  holds **0 rows**. Rule: `compute_timbre` in
  `crates/domain/src/sales/pricing.rs:156-169`: 1% of (HT+TVA),
  ceiling-to-100-centimes, clamped to [500, 250000], applying only to
  TICKET/FACTURE with cash tendered > 0.
- **This sale:** `cash_tendered=15000 > 0`, base `15050` →
  `((15050+9999)/10000)*100 = 200` → clamped to **MIN 500** → persisted
  `timbre=500` (5.00 DZD). Computation verified exact against the formula.
- **Why checkout displayed the message:** `CheckoutModal.tsx:253-257` renders
  `checkout.timbreNote` ("Timbre fiscal 1% appliqué (espèces,
  Ticket/Facture)") whenever cash > 0 and doc ≠ BL — informational, always
  shown pre-commit.
- **Consistency of the visible 150.50 total:** the modal's TOTAL TTC row
  (`totals.net`, lines 249-252) is preview HT−discount **excluding timbre**
  (no timbre row exists in the preview). Displayed 150.50 vs persisted
  155.50 diverge by exactly the 500 timbre.
- **Classification: (b) — behavior correct but UI display misleading.** The
  persisted timbre is lawful per the coded rule; the preview total and the
  preview change (see §4) do not include it, so the operator saw figures 5.00
  DZD away from the books. (Full impact under Findings.)

## Section 4 — Payment and Change Reconciliation

Persisted `payments` rows (both `sequence_no=3`, see §7 note):
`9fc54fad-…` CASH **15000** (cash_session `7c973d09-…`, actor admin);
`46bd45f6-…` CARD **15050** (`cash_session_id` NULL).
Recomputation: tendered `15000+15050 = 30050 = paid_amount` ✓;
`30050 − 15550 = 14500 = change_amount` ✓.
- Card was **not** treated as refundable change and creates **no** cash
  movement (NULL session link); only the CASH leg feeds the drawer.
  No duplicated revenue: distinct row ids, TTC counted once.
- `customer_ledger` holds **0 rows** — absent as expected (no CREDIT leg,
  `customer_id` NULL on both rows, `payment_method=MIXED`).

## Section 5 — Stock Integrity and Negative-Stock Rule

- Product `f20f6f21-…` (VIS-8X40, created 17:17:13Z via catalog UI), sold unit
  `78d65d76-…` (base, factor 10000, price 15050).
- Post-sale `stock_levels` (DEP-01): `quantity_base_micro=-20000`,
  `revision=2`. History: **zero inbound movement ever** (no purchase,
  adjustment, or opening row — only the two SALE_OUT rows below exist).
  `…000001` deducted 10000 (0 → −10000); `…000002` deducted 10000
  (−10000 → −20000). Both `stock_movements` rows carry
  `authorized_negative_stock=1`, `reference_type='SALE'`, correct
  `reference_id` linkage.
- Authorization trail: `audit_log` seq 3 and seq 4 `NEGATIVE_STOCK_OVERRIDE`
  on `stock_levels|DEP-01:product` with `new_value_json`
  `{"quantity_base_micro":-10000}` / `{-20000}` — one per sale.
  Mechanism (`sales.rs:239-241` + `sqlite_stock_repository.rs:241-247`):
  the flag derives **automatically** from the actor's standing
  `NegativeStockOverride` permission (admin holds it); no explicit per-sale
  operator gesture exists, and the audit row is the only trace.
- **Classification: (b) — negative stock occurred with valid authorized
  override and audit evidence** (mechanism PASS; the *policy* question of an
  explicit confirmation gesture is a HIGH finding, not an invariant breach).
- Reader-trap note: `stock_levels.quantity_micro` reads **0** because that
  column is written only once at row-creation INSERT (`sqlite_stock_repository.rs:168-174`)
  and never updated (UPDATE at :249-254 writes base+revision only);
  `quantity_base_micro` is authoritative.

## Section 6 — Cash Session and Cash Movement Integrity

Session `7c973d09-…`: OPEN, DEP-01 terminal-bound, float 0, no close fields —
internally consistent. Movements: `OPENING_FLOAT 0` + `SALE_CASH_IN 500`
(`…000001`) + `SALE_CASH_IN 500` (`…000002`).
Rule (`sqlite_checkout_repository.rs:296-318`): `net = cash_tendered −
change`, persisted iff > 0. Per sale: `15000 − 14500 = 500` ✓. Card 15050
correctly excluded from drawer math. Expected drawer after both sales:
`0 + 500 + 500 = 1000` (10.00 DZD) — consistent with the books (see the
physical-vs-book caveat in Findings).

## Section 7 — Atomicity, Idempotency, Audit Chain, Print Outbox

- **Atomicity:** document, item, 2 payments, stock movement, cash movement,
  audit seq 4, command `483cfdb1-…` (payload embeds the stored receipt incl.
  `timbre_centimes:"500"`), and RECEIPT job `9dc4ed2f-…` all share stamp
  `17:22:17.422Z`; `revision=1`; no partial-commit evidence.
- **Idempotency:** the twin sales carry *distinct* command ids, so both
  committed legitimately (double submission, not replay). A same-command
  replay would hit the recorded identity (`command_id` PK + request hash);
  no duplicate-command attempt is observable, so replay-rejection is
  structurally present but empirically unexercised — stated, not claimed.
- **Audit chain:** all 4 rows recomputed in this audit with SHA-256 over the
  writer canonical form — `seq1 FIRST_ADMIN_BOOTSTRAP`, `seq2
  CASH_SESSION_OPENED`, `seq3/4 NEGATIVE_STOCK_OVERRIDE`: hash_match 4/4,
  GENESIS-rooted linkage 4/4. Continuous.
- **Print outbox (ADR-006):** RECEIPT job created at the commit instant
  (same 17:22:17.422Z ⇒ inside the atomic transaction), status PENDING —
  matches the "Impression en attente" modal exactly. 8 subsequent DUPLICATA
  jobs (17:22:23–27, all PENDING, printer offline) show repeated manual
  reprint taps; `…000001`'s RECEIPT sits PROCESSING (claimed, uncompleted).
- **Cosmetic note:** `payments.sequence_no` mirrors the *global stock-movement
  counter* (`next_sequence_no` over `stock_movements`, checkout repo :269):
  `…000001`→2,2 and `…000002`→3,3. No financial effect; numbering is
  cross-domain, not per-document.

## Section 8 — Sovereign Invariant Verdict

| Invariant | Verdict |
|-----------|---------|
| Atomic sale persistence | **PASS** — 7 row groups, one stamp, FINALIZED rev 1 |
| No unauthorized negative stock | **PASS** — role permission + per-sale flag + audit rows (policy caveat → Finding HIGH-2) |
| Exact money reconciliation | **PASS** — TTC/tendered/change recomputed to the centime |
| Payment/change integrity | **PASS** (persisted) — display divergence → Finding CRITICAL-1 |
| Cash-session integrity | **PASS** — net-cash rule exact, card excluded |
| No unintended credit effect | **PASS** — ledger empty, no CREDIT legs |
| Immutable committed document state | **PASS** — FINALIZED + `prevent_documents_update_finalized` triggers (0001:430-455, narrowed 0011:30-37), rev 1 |
| Audit presence + chain continuity | **PASS** — 4/4 SHA-256 re-verified |
| Transactional print outbox | **PASS** — in-txn RECEIPT, PENDING as displayed |
| Gapless numbering (local evidence) | **PASS** — `document_sequences` TICKET/2026=2; 000001+000002 consecutive (deletion undetectable by design — stated limit) |

## Section 9 — Findings and Single Recommendation

- **CRITICAL-1 — Preview/commit total divergence (cash-short risk):** checkout
  preview total/change (`CheckoutModal.tsx:83` `maxZero(tendered − net)`,
  display rows :241-252) exclude timbre while the commit adds up to 2500 ct.
  Here: screen 150.50/150.00 vs books 155.50/145.00 — if the operator handed
  the displayed 150.00 change, the drawer is 5.00 DZD short of the books on
  this sale alone, recurring on every cash ticket.
- **HIGH-2 — Silent negative-stock accumulation:** −2.0 units with zero
  inbound stock ever; override is role-automatic (`sales.rs:239-241`), no
  explicit confirmation. Decide: explicit override gesture for zero-stock
  sales.
- **HIGH-3 — Twin-sale double submission:** two identical FINANCIAL sales 30 s
  apart under distinct command ids. System behaved correctly; owner must
  confirm whether one was accidental (double-click/F12) — if so, consider a
  submit-guard review, as idempotency keys cannot catch distinct intents.
- **MEDIUM-4 — Reprint/print backlog:** 8 PENDING DUPLICATA + 1 PROCESSING
  RECEIPT with no printer attached; confirm worker reclaim/escalation policy
  for stuck PROCESSING.
- **LOW-5 — payments.sequence_no is a stock-movement counter** (cosmetic;
  consider per-document payment indexing).
- **LOW-6 — `stock_levels.quantity_micro` vestigial** (always 0; base_micro
  authoritative — document, don't read it).
- **INFORMATIONAL-7 — `tax_rules` empty by design** (timbre is code formula
  `pricing.rs:147-169`: 1%, ceil-to-100ct, [500, 250000], cash TICKET/FACTURE
  only); DEP-01 seed + FIRST_ADMIN prologue verified in passing.

**Exactly ONE recommendation:** authorize a single minimal corrective task —
*include the computed timbre in the checkout preview TOTAL TTC and the
preview change before commit* (frontend preview math only; backend formula
untouched) — and **authorize no further live sales until it is verified**,
since every cash ticket otherwise repeats a physical-vs-book cash gap.

### Appendix — read-only queries used (abridged)

`SELECT * FROM documents WHERE number=?`; `... document_items/payments ...
WHERE document_id=?`; product/units/levels/movements by product id;
`SELECT * FROM cash_sessions/cash_movements WHERE ...`; `SELECT * FROM
tax_rules`; `SELECT COUNT(*) FROM customer_ledger`; audit/command/print/sequence
table scans; column listings via `pragma_table_info`. All through a
`mode=ro` URI connection; secrets never selected.
