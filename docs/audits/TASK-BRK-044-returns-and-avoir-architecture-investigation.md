# TASK-BRK-044 — Returns & Avoir Architecture Investigation

**Status:** fact-finding only — zero files modified, no fix proposed, no UI built.
**Method:** read-only grep + targeted file reads across `crates/domain`,
`crates/application`, `crates/infrastructure` (adapters + migrations),
`crates/commands`, and `src-tauri`. Every claim carries file:line evidence.

---

## 1. Domain primitives & enums

### 1.1 `DocumentType` — AVOIR EXISTS (fully wired code/prefix/parse)

**File:** `crates/domain/src/sales/document.rs:16-33`

```rust
pub enum DocumentType {
    Ticket, Facture, Bl, Devis,
    /// Credit note / return.
    Avoir,                                          // :25-26
    Bc, Br, Ff,
}
```

Storage code `"AVOIR"` (`:44`), gapless prefix `"AVO"` (`:59`), parse arm
`"AVOIR" => Ok(Self::Avoir)` (`:73`). It is a first-class kind for numbering
(`UNIQUE(type, fiscal_year, number)` accepts it with zero schema change) and
is explicitly excluded from conversion: only `Finalized DEVIS/BL` may enter
`Converted` (`:137-144`); a `Finalized Avoir` is a *type* error
(`InvalidConversionSourceType`), and `Converted` is terminal for all eight
kinds including Avoir (`:312-327`). **No `BonDeRetour`/`Return` document
variant exists** — AVOIR is the sole return-document kind.

No dedicated `Return` domain entity/value object exists anywhere in
`crates/domain` (grep for `struct .*Return|ReturnRequest|ReturnLine`:
no matches).

### 1.2 `StockMovementType` — RETURN_IN EXISTS (positive restock)

**File:** `crates/domain/src/inventory/movement.rs:21-36`

```rust
/// Customer return restock (positive delta).
ReturnIn,                                           // :24-25
```

Code `"RETURN_IN"` (`:44`), sign `+1` (`:57`), parse arm (`:72`).
The generic engine already applies it: `compute_new_quantity` works for any
type (`stock_movement_engine.rs:68-77`), the policy matrix covers it
(`stock_movement_engine.rs:89`), and the backend test
`all_seven_types_flow_through_the_engine` drives
`(ReturnIn, 40_000, 40_000)` through the real adjust path
(`crates/infrastructure/tests/stock_movements.rs:555`). `apply_one` in the
stock repository accepts any `StockMovementType`, so a `RETURN_IN` row is
writable today — but only via the **generic manual-adjust flow**, never via
a return-specific flow (zero `ReturnIn` writes in `infrastructure/src`;
grep `ReturnIn|ReferenceType::Return|"RETURN"` there: no matches).

Companion: `ReferenceType::Return` ("Return document (future engine)")
exists at `movement.rs:104-105` but is **never constructed** in
`infrastructure/src` (same grep: no matches).

### 1.3 Cash movements — NO refund/payout type

**File:** `crates/domain/src/cash/cash_movement.rs:13-28` — the closed set is
`OpeningFloat, SaleCashIn, CreditPaymentIn, ExpenseOut, ManualIn, ManualOut,
SupplierPaymentOut` with codes (`:35-42`) and strict sign invariants
(`:63-79`; `validate_movement_amount`, `:110-131`, mirroring the migration
0008 CHECK widened by 0009). **No `RETURN_CASH_OUT` / `REFUND_OUT` exists.**
Nearest usable primitive for a cash refund from the drawer is `ManualOut`
(< 0 stored, `:25; :76`). A dedicated refund type would require a domain
variant + migration CHECK widening (0009 precedent:
`SUPPLIER_PAYMENT_OUT`).

## 2. Application layer — NO RETURN USE CASE EXISTS

- `crates/application/src/use_cases/` contains exactly: `cash, catalog,
  customers, inventory, mod, printing, purchases, reports, sales, suppliers`
  (directory listing; **no returns/avoir module**).
- Grep `avoir|Avoir|AVOIR|refund|Refund|reversal|Reversal` over all of
  `crates/application/src`: **no matches**.
- Grep `ReturnIn|RETURN_IN|return_in|ProcessReturn|Return\b`: only two
  English-prose hits (`ports.rs:138` "Return the live context",
  `inventory.rs:147` "Return active warehouses") plus the domain
  `ReturnIn` matrix entry (`stock_movement_engine.rs:89`). **No use case
  references `ProcessReturn`.**
- Grep `Avoir|DocumentType::Avoir` over `crates/application/src`: **no
  matches** — checkout accepts only Ticket/Facture/Bl
  (`sales.rs:203-208`, else `UNSUPPORTED_DOCUMENT_TYPE`), conversions
  accept only Devis/Bl sources. An AVOIR cannot be created through any
  current use case.
- `ports.rs` grep `[Rr]eturn|[Aa]voir|[Rr]efund`: 13 hits, **all English
  "return/returns" in doc comments** (`ports.rs:37,46,138,364,372,554,779,
  828,845,969,1006,1011,1017`) — zero return-specific port methods.

Reserved gate, currently dangling: `Permission::ProcessReturn`
(`role.rs:37`) is held by **Gerant** (`role.rs:149`) and **Admin**
(`role.rs:180`) only (Vendeur's 5-perm set at `:121` and Caissier's set at
`:122-136` lack it) — but **no use case checks it** (see grep above).
The role model anticipates returns; the logic does not exist.

## 3. Schema & persistence

- `documents.source_document_id TEXT REFERENCES documents(id)`
  (`0001_baseline_schema.sql:178`) + partial index
  `idx_documents_source_document` (`0011:27-28`) — an AVOIR can reference
  its source invoice with zero schema change. Migration 0011's own comment
  (`0011:5-9`) confirms the column predates conversions.
- `document_items.source_item_id TEXT` (`0001:204`) — per-line return
  linkage exists as a bare nullable column with **no FK, no trigger, no
  writer**.
- **No trigger enforces a return quantity ceiling.** All 34 CHECK/RAISE
  matches in `0001` are append-only guards (`customer_ledger`,
  `supplier_ledger`, `stock_movements`, `cash_movements`, `payments`,
  `audit_log`, `command_log`) plus finalized-document immutability
  (`prevent_documents_update_finalized`, `:430-437`;
  `prevent_document_items_update/delete_finalized`, `:439-457`). Nothing
  compares a return line against its source line — that check must live in
  application logic (to be built).
- `prevent_documents_update_finalized` (as amended by 0011) permits exactly
  one sealed-row mutation: `FINALIZED → CONVERTED` (`0011:30-39`); **it
  applies to AVOIR rows identically** (status-based, not type-based), so a
  finalized AVOIR is immutable from birth with no code changes.
- `customer_ledger` representation: writers today emit `DEBIT/'SALE'`
  (sale charge, `sqlite_customer_repository.rs:95-98`) and
  `CREDIT/'PAYMENT'` (versement, `:456-459`); the statement projector
  computes `DEBIT adds, CREDIT subtracts` and **rejects any other
  entry_type** (`:564-585`). A credit-side avoir (debt reduction) fits the
  `CREDIT` leg arithmetically, but **no code writes an avoir-linked ledger
  row** (`document_type` is free text — `'AVOIR'` would be accepted by the
  schema yet break the strict projector unless handled).

## 4. IPC & registration — NO RETURN COMMAND EXISTS

- Grep `[Rr]eturn|[Aa]voir|AVOIR|[Rr]efund` over `crates/commands/src`
  (dto.rs + ipc.rs): 9 hits, **all English prose** ("returns the…",
  "Prefix/multilingual query") — zero return DTOs, zero return handlers.
- Grep over `src-tauri/src/lib.rs`: 1 hit, English prose (`:52`
  "after `run` returns"). **Zero return registrations.**

## 5. Summary table

| Question | Current Codebase Status (evidenced) | File:Line |
|---|---|---|
| DocumentType enum has AVOIR? | YES — variant + `"AVOIR"` code + `"AVO"` prefix + parse arm; excluded from conversions | `document.rs:25-26,44,59,73` |
| StockMovementType has RETURN_IN? | YES — `+1` restock; engine-appliable, test-driven via generic adjust path; zero return-flow writes | `movement.rs:24-25,44,57,72`; `stock_movement_engine.rs:89`; `stock_movements.rs:555` |
| Cash movement has refund/payout type? | NO — nearest is `ManualOut` (<0); dedicated type needs domain + CHECK widening (0009 precedent) | `cash_movement.rs:13-28` |
| Application Use Case exists for return? | NO — no module, no references to Avoir/ProcessReturn in any use case | `use_cases/` listing; greps §2 |
| Schema supports source_document_id? | YES — `documents.source_document_id` + partial index; `document_items.source_item_id` bare nullable | `0001:178,204`; `0011:27-28` |
| Schema enforces return item qty ceiling? | NO — triggers are append-only/immutability only; ceiling must be application logic | `0001:345-457` (absence) |
| IPC command exists for returns? | NO — zero DTOs/handlers/registrations | `commands/src` + `lib.rs` greps §4 |

## 6. Build-vs-reuse assessment

**Already exist (reuse as-is):** `DocumentType::Avoir` (+`AVO` numbering via
the existing gapless allocator + `UNIQUE(type,fiscal_year,number)`);
`StockMovementType::ReturnIn` (+1 sign, engine + test coverage);
`ReferenceType::Return` (declare-only); `documents.source_document_id` +
partial index; `document_items.source_item_id` (column only);
FINALIZED-immutability covering Avoirs; `Permission::ProcessReturn`
(Gerant/Admin) awaiting its first `require()` call; ledger `CREDIT`
arithmetic (debt-reducing direction correct).

**Must be built from scratch:** return/avoir use case(s) — source
validation (FINALIZED TICKET/FACTURE/BL only), per-line qty ceiling vs
source lines, atomic bundle (AVOIR header FINALIZED + byte-linked items +
`RETURN_IN` movements + cash leg (`ManualOut` or a new refund type) and/or
`CREDIT` ledger leg + print outbox + idempotent `command_id`), DTOs +
`process_return`-style IPC command + registration, TS wrappers, and UI.
Open design decisions: dedicated cash refund movement type vs `ManualOut`;
whether an avoir-linked `CREDIT` row keeps `document_type='PAYMENT'` or
introduces `'AVOIR'` (projector at `sqlite_customer_repository.rs:567-574`
must accept whatever is chosen); whether converted/partially-returned
sources need a status beyond FINALIZED.
