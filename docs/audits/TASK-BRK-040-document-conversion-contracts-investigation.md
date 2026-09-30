# TASK-BRK-040 — Document Conversion IPC Contracts Investigation

**Status:** fact-finding only — no code changed, no fix proposed, no UI built.
**Scope note:** the task brief names `crates/application/src/use_cases/documents.rs`
as the use-case home. **That file does not exist.** The `use_cases/` directory
contains `cash, catalog, customers, inventory, mod, printing, purchases,
reports, sales, suppliers` (verified by directory listing). **Both conversion
use cases live in `crates/application/src/use_cases/sales.rs`.** All
citations below use the real path.

---

## 1. `convert_devis_to_bl`

### 1.1 Request DTO — verbatim (`crates/commands/src/dto.rs:1618-1627`)

```rust
pub struct ConvertDevisToBlRequest {
    pub session_token: String,      // required — authorizing session token
    pub command_id: String,         // required — client idempotency key (UUID v4)
    pub source_document_id: String, // required — source DEVIS id (UUID string)
}
```

No optional fields. No tender/payment fields — this conversion is
payment-agnostic. Use-case input mirrors it 1:1
(`sales.rs:766-771`, `ConvertDevisToBlInput { command_id: String,
source_document_id: DocumentId }`); `terminal_id` is resolved from
TerminalContext at the IPC edge, never client-supplied (`sales.rs:802-803`,
ADR-028).

### 1.2 Response DTO — verbatim (`dto.rs:1629-1644`)

```rust
pub struct ConvertDevisToBlResponse {
    pub new_document_id: String,      // new BL id (UUID string)
    pub new_document_number: String,  // gapless human number, e.g. "BL-2026-000001"
    pub new_document_status: String,  // "FINALIZED"
    pub total_ttc_centimes: String,   // copied grand total, canonical centime string
    pub items_count: u32,             // copied line count
    pub warnings: Vec<String>,        // negative-stock override notices (empty when stock sufficed)
}
```

Handler mapping (`ipc.rs:1124-1131`) stringifies `total_ttc_centimes`
(`.to_string()`); all money crosses as strings (Section 7).

### 1.3 Error variants (each with trigger condition)

| Wire code (`error_code`, `ipc.rs:155-336`) | Source variant | Trigger condition |
|---|---|---|
| `UNAUTHORIZED` (`ipc.rs:321`) | `ApplicationError::Unauthorized` | Caller lacks `CreateSale` (`sales.rs:811-813`) |
| `INVALID_INPUT` via `InvalidCanonicalString("command id must not be empty")` | empty `command_id` (`sales.rs:814-818`, passthrough `stable_code_or`, `ipc.rs:310-312`) | blank command id |
| `INVALID_TERMINAL_ID` (passthrough) | `coded("INVALID_TERMINAL_ID")` (`sales.rs:819-821`) | empty terminal context |
| `SOURCE_DOCUMENT_NOT_FOUND` (`ipc.rs:289-291`) | `DomainError::SourceDocumentNotFound` (`sales.rs:824-826`) | unknown source id |
| `INVALID_CONVERSION_SOURCE_TYPE` (`ipc.rs:286-288`) | non-Devis source (`sales.rs:829-833`; BL sources belong to the Facture closing) | e.g. BL id supplied |
| `INVALID_CONVERSION_SOURCE_STATUS` (`ipc.rs:283-285`) | Draft/Cancelled source (`sales.rs:838-841` pre-check + authoritative in-txn gate `sqlite_conversion_repository.rs:305-312`) | DRAFT / CANCELLED |
| `DOCUMENT_ALREADY_CONVERTED` (`ipc.rs:280-282`) | replay against sealed source — pre-check defers CONVERTED to the repo (`sales.rs:838-842`), repo `ALREADY_CONVERTED` (`sqlite_conversion_repository.rs:309-310,431-437`) | second distinct command on same source |
| `EMPTY_SOURCE_DOCUMENT` (`ipc.rs:292-294`) | zero lines (`sales.rs:843-848`) | empty DEVIS |
| `INSUFFICIENT_STOCK_FOR:<label>:<shortfall>` (full-message passthrough, `sqlite_conversion_repository.rs:108-114`) | short stock without override grant (`:412-427`) | unprivileged short sale |
| `STOCK_NEGATIVE_CONFIRMATION_REQUIRED` (`ipc.rs:327-329`) | privileged short sale without explicit `allow` gesture (TASK-BRK-033 path) | Gerant/Admin, unconfirmed |
| `IDEMPOTENCY_CONFLICT` (`ipc.rs:212`) | same `command_id`, divergent payload (canonical-hash mismatch) | key reuse with different body |

### 1.4 Authorized roles — file:line evidence

Gate is `Permission::CreateSale` (`sales.rs:811`). Holders
(`crates/domain/src/role.rs`): **VENDEUR** (`:121`), **CAISSIER** (`:131`),
**GERANT** (`:156`), **ADMIN** (`:191`) — i.e. **all four roles** may invoke
it (the BL test suite states this verbatim at
`bl_to_facture_conversion.rs:1045-1046`: "All four roles hold CreateSale").
Caveat: short-stock conversion additionally needs `NegativeStockOverride`,
held only by GERANT (`role.rs:147`) and ADMIN (standing-grant list) —
Vendeur/Caissier short conversions hard-fail.

### 1.5 Finality + source mutation

- New BL is inserted with status `'FINALIZED'`
  (`sqlite_conversion_repository.rs:330-333`, `?14, 'FINALIZED'`) and
  reported as `new_document_status = "FINALIZED"`
  (`:468`, asserted in test at `devis_to_bl_conversion.rs:321,346`).
  FINALIZED documents are immutable (baseline triggers
  `prevent_documents_update_finalized`, `0001_baseline_schema.sql`).
- Source is sealed by a guarded single-row flip
  (`:431-437`): `UPDATE documents SET status = 'CONVERTED' WHERE id = ?1 AND
  status = 'FINALIZED'` — exactly-one-row check, else `ALREADY_CONVERTED`.
  Test asserts `doc_status == "CONVERTED"` (`devis_to_bl_conversion.rs:379`).

### 1.6 Stock side effects

**Yes — decrements at BL stage.** Leaf deductions via the sale engine's own
`apply_one(..., StockMovementType::SaleOut, ...)` (`:382-399`), one
`SALE_OUT` movement per line referencing the new BL (test `:385-400`),
levels drop exactly (test `:381-383`: 100_000 → 80_000 / 90_000). Kit lines
are expanded pre-transaction by the use case (`sales.rs:849-861`).

---

## 2. `convert_bl_to_facture`

### 2.1 Request DTO — verbatim (`dto.rs`, added TASK-BRK-038)

```rust
pub struct ConvertBlToFactureRequest {
    pub session_token: String,      // required
    pub command_id: String,         // required — idempotency key (UUID v4)
    pub source_document_id: String, // required — source BL id (UUID string)
    pub document_language: String,  // required — "AR" or "FR", never defaulted
    pub payments: Vec<PaymentLineDto>, // required, non-empty — exact-settlement tenders
}
```

`PaymentLineDto` reuses the checkout tender shape verbatim (`dto.rs:492-499`):
`{ payment_method: String /* CASH|CREDIT|CHEQUE|CARD */, amount_centimes:
String }`. Use-case input mirrors it (`sales.rs:920-929`, `ClosingInput {
command_id, source_document_id: DocumentId, language: DocumentLanguage,
payments: Vec<ClosingPaymentInput> }` with `ClosingPaymentInput { method:
PaymentMethod, amount_centimes: i64 }` at `sales.rs:911-916`).

### 2.2 Response DTO — verbatim (`dto.rs`, added TASK-BRK-038)

```rust
pub struct ConvertBlToFactureResponse {
    pub new_document_id: String,                     // new FACTURE id
    pub new_document_number: String,                 // e.g. "FAC-2026-000001"
    pub new_document_status: String,                 // "FINALIZED"
    pub total_ttc_centimes: String,                  // settled total (source copy)
    pub items_count: u32,                            // copied line count
    pub customer_new_balance_centimes: Option<String>, // set iff credit tendered
}
```

Handler (`ipc.rs`, `convert_bl_to_facture`, registered
`src-tauri/src/lib.rs:146` next to `convert_devis_to_bl` at `:145`):
session extraction, `parse_wire_id` source, per-tender
`PaymentMethod::from_code` + `parse_wire_int`, `parse_document_language`,
`command_id` threaded into `ClosingInput`, executes
`ConvertBlToFactureUseCase::new(state.customers, state.documents)` with
`&state.terminal_id`, maps via `error_code`, stringifies money (incl. the
`Option<i64>` → `Option<String>` balance).

### 2.3 Error variants (each with trigger condition)

Superset of §1.3 (same session/id/terminal/source/type/status/empty/already
arms) plus the financial legs:

| Wire code | Source variant | Trigger condition |
|---|---|---|
| `EMPTY_CART` (`ipc.rs:185`) | `DomainError::EmptyCart` (`sales.rs:983-985`) | `payments: []` (test `empty_and_non_positive_tenders_are_rejected`, `bl_to_facture_conversion.rs:1062-1072`) |
| `INVALID_PAYMENT_AMOUNT` (passthrough) | `coded(...)` (`sales.rs:986-990`) | any tender ≤ 0 (test `:1073-1086`) |
| `INSUFFICIENT_PAYMENT` (`ipc.rs:186-188`) | tender sum ≠ source TTC, either direction (test `tender_mismatch_is_rejected_both_directions`, `:864-897`: 15_000 and 25_000 vs 20_000 both rejected, zero writes) | under- or over-tender |
| `CREDIT_CUSTOMER_REQUIRED` (passthrough) | credit tender but source has no customer (`sales.rs:1041-1049`) | CREDIT leg, BL without party |
| `CUSTOMER_NOT_FOUND` / `NOT_FOUND` | unknown credit customer (`sales.rs:1050-1053`) | dangling party id |
| `CREDIT_DISABLED` (`ipc.rs:192`) | inactive or DISABLED policy (`sales.rs:1054-1060`, repo `authorize_credit_for_closing`, `sqlite_conversion_repository.rs:82-86`) | disabled profile |
| `CREDIT_LIMIT_EXCEEDED` (`ipc.rs:211`) | LIMITED cap breach without `CreditLimitOverride` (`sales.rs:1061-1074`; test `credit_over_limit_fails_with_checkout_error`) | over-limit credit (Vendeur/Caissier; Gerant/Admin hold the override) |
| `NO_OPEN_CASH_SESSION_FOR_TERMINAL` (`ipc.rs:238-240`) | cash tendered with no OPEN session (test `cash_without_open_session_fails_with_checkout_error`) | cash leg, closed drawer |
| `CONCURRENCY_INVARIANT_VIOLATED` (`ipc.rs:330-332`) | post-ledger invariant re-check trips (test `post_ledger_failure_rolls_back_everything`, full rollback asserted) | ledger/balance skew |
| `IDEMPOTENCY_CONFLICT` | divergent payload, same key (test `idempotent_replay_and_conflict`, `:901-978`: identical replay returns equal result; conflict writes nothing) | key reuse, different body |

### 2.4 Authorized roles

Same `CreateSale` gate (`sales.rs:971-973`) → **all four roles**
(VENDEUR `:121`, CAISSIER `:131`, GERANT `:156`, ADMIN `:191`). Credit
over-limit additionally needs `CreditLimitOverride` (Gerant/Admin only).

### 2.5 Finality + source mutation

Identical to §1.5: new FACTURE inserted `'FINALIZED'`
(`sqlite_conversion_repository.rs:623-626`), source sealed by the same
guarded `CONVERTED` flip (`:724-730`). Test asserts `FAC-…-000001 /
FINALIZED / CONVERTED` (`bl_to_facture_conversion.rs:397-399,457`).

### 2.6 Stock side effects — NONE (quoted proof)

> "ADR-046 HARD INVARIANT: this method performs ZERO stock writes. Physical
> stock left the warehouse when the source BL was finalized, so no
> `stock_movements` INSERT and no `stock_levels` UPDATE may occur here under
> any circumstance." (`sqlite_conversion_repository.rs:489-493`)

Use-case header concurs (`sales.rs:931-938`: "The repository performs zero
stock writes by invariant (ADR-046)"). Proven by test: `inventory_snapshot`
before/after are byte-identical (`bl_to_facture_conversion.rs:388,470-474`).

### 2.7 Payment: part of the command (not a later step)

Payment collection **is** the command's core input: tenders must cover the
source TTC exactly ("no change concept", `sales.rs:1016-1036`; repo
`TENDER_MISMATCH`, `sqlite_conversion_repository.rs:537-552`). Cash legs
require an OPEN session (`:555-560`); a `SALE_CASH_IN` movement referencing
the FACTURE is written (`bl_to_facture_conversion.rs:456-468`); CREDIT legs
post a ledger DEBIT + balance update (test `credit_closing_…`, `:496+`).
There is no separate "pay a converted invoice later" step — settlement is
atomic with the closing.

---

## 3. Cross-cutting answers

- **Idempotency:** both commands are idempotent, keyed by `command_id`
  (UUID string, blank rejected). Pattern per command: `request_canonical`
  joined string (`sales.rs:881-907` devis; `:1101-1119` BL), `checksum_of`
  hash, `command_log` lookup-or-replay inside the same `BEGIN IMMEDIATE`
  transaction (repo `:504-513` BL side; same shape devis side), equal-result
  replay + `IDEMPOTENCY_CONFLICT` on divergence. Proven by
  `idempotent_replay_returns_identical_result_without_duplicates`
  (`devis_to_bl_conversion.rs:707`) and `idempotent_replay_and_conflict`
  (`bl_to_facture_conversion.rs:901`).
- **Frontend wrappers in `src/lib/ipc.ts`:** **completely absent for both.**
  Grep for `convert|Convert|devis|Devis|facture|Facture|bl_to|BlTo` across
  `src/lib/ipc.ts` returns zero hits. No typed wrapper, no wire types, no
  screen consumes either command (consistent with TASK-BRK-036).
- **Registration:** `src-tauri/src/lib.rs:145-146` —
  `baraka_commands::ipc::convert_devis_to_bl,` /
  `baraka_commands::ipc::convert_bl_to_facture,` inside `generate_handler!`.

### Realistic example payloads (from fixtures, canonical string wire types)

Devis→BL (derived from `convert_finalized_devis_creates_bl_and_seals_source`,
`devis_to_bl_conversion.rs:292-324` — two lines, TTC 40_000):

```json
{
  "session_token": "<login token>",
  "command_id": "<fresh UUID v4>",
  "source_document_id": "<DEVIS id, status FINALIZED, ttc 40000>"
}
```
Response (asserted values): `{"new_document_number": "BL-<year>-000001",
"new_document_status": "FINALIZED", "total_ttc_centimes": "40000",
"items_count": 2, "warnings": []}`.

BL→Facture (derived from `cash_closing_creates_facture_movement_and_seals_source`,
`bl_to_facture_conversion.rs:367-402` — BL TTC 40_000 = 2×10_000 + 1×20_000):

```json
{
  "session_token": "<login token>",
  "command_id": "<fresh UUID v4>",
  "source_document_id": "<BL id, status FINALIZED>",
  "document_language": "FR",
  "payments": [{ "payment_method": "CASH", "amount_centimes": "40000" }]
}
```
Response (asserted values): `{"new_document_number": "FAC-<year>-000001",
"new_document_status": "FINALIZED", "total_ttc_centimes": "40000",
"items_count": 2, "customer_new_balance_centimes": null}`. Credit variant
(test `:496+`): `payments: [{CREDIT, "<exact TTC>"}]` with the BL carrying
`party_id`, returns the new balance string.

---

## 4. Summary table

| Question | `convert_devis_to_bl` | `convert_bl_to_facture` |
|---|---|---|
| Request fields | `session_token, command_id, source_document_id` (all required Strings; `dto.rs:1620-1627`) | `session_token, command_id, source_document_id, document_language (AR/FR), payments: Vec<PaymentLineDto>` (`dto.rs`, BRK-038) |
| Response fields | `new_document_id/number/status, total_ttc_centimes (String), items_count, warnings` (`dto.rs:1631-1644`) | `new_document_id/number/status, total_ttc_centimes (String), items_count, customer_new_balance_centimes: Option<String>` (`dto.rs`, BRK-038) |
| Error variants | UNAUTHORIZED; empty command_id; INVALID_TERMINAL_ID; SOURCE_DOCUMENT_NOT_FOUND; INVALID_CONVERSION_SOURCE_TYPE; INVALID_CONVERSION_SOURCE_STATUS; DOCUMENT_ALREADY_CONVERTED; EMPTY_SOURCE_DOCUMENT; INSUFFICIENT_STOCK_FOR:*; STOCK_NEGATIVE_CONFIRMATION_REQUIRED; IDEMPOTENCY_CONFLICT (evidence §1.3) | All of the left column (BL-side equivalents) plus EMPTY_CART; INVALID_PAYMENT_AMOUNT; INSUFFICIENT_PAYMENT (either direction); CREDIT_CUSTOMER_REQUIRED; CUSTOMER_NOT_FOUND; CREDIT_DISABLED; CREDIT_LIMIT_EXCEEDED; NO_OPEN_CASH_SESSION_FOR_TERMINAL; CONCURRENCY_INVARIANT_VIOLATED (evidence §2.3) |
| Authorized roles | All four (CreateSale: Vendeur `:121`, Caissier `:131`, Gerant `:156`, Admin `:191`); short-stock needs NegativeStockOverride (Gerant/Admin) | All four (same gate, `sales.rs:971`); credit over-limit needs CreditLimitOverride (Gerant/Admin) |
| Idempotent? | Yes — `command_id` + canonical hash + command_log replay (`sales.rs:881-907`; test `:707`) | Yes — identical pattern (`sales.rs:1101-1119`; test `bl_to_facture_conversion.rs:901-978`) |
| Frontend wrapper in `ipc.ts`? | **Absent** (zero grep hits) | **Absent** (zero grep hits) |
| Stock/payment side effects | Decrements stock now (`SALE_OUT` per line, `:382-399`); no payment input (payment-agnostic quote copy) | ZERO stock writes (ADR-046, `:489-493`); payment IS the input (exact settlement, cash-session + credit gates) |

Resulting docs are `FINALIZED` (immutable) in both flows; sources flip
`FINALIZED → CONVERTED` by a guarded single-row update in both flows
(devis `:431-437`; BL `:724-730`). No repository files were modified
(no-write discipline — investigation used reads only).
