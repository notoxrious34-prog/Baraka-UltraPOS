# TASK-BRK-041 — Document Retrieval / Query Capabilities Audit

**Status:** read-only audit — no Rust files modified.
**Purpose:** determine how the planned Document Conversion UI can look up
candidate DEVIS/BL documents (by id, human number, or type/status filter).

Scope inspected: `crates/commands/src/ipc.rs` (full handler list),
`src-tauri/src/lib.rs` (`generate_handler!`), `crates/commands/src/dto.rs`
(document DTOs). Handler inventory cross-checked against the TASK-BRK-036
command census — no document list/search handler exists anywhere.

---

## 1. Retrieve a document by UUID — EXISTS

### `sales_get` — `crates/commands/src/ipc.rs:1253-1308`, registered `src-tauri/src/lib.rs:147`

Signature (note: **bare scalar params, no `request` DTO object**):

```rust
pub fn sales_get(
    state: tauri::State<'_, AppState>,
    session_token: String,   // required
    document_id: String,     // required — UUID string
) -> Result<GetSaleResponse, String>
```

Input: `session_token` + `document_id` (parsed via `parse_wire_id`,
`ipc.rs:1261-1263`; any authenticated session — `GetSaleUseCase::execute`
takes `_session` and enforces no role gate,
`crates/application/src/use_cases/sales.rs:1139-1143`; unknown id →
`NotFound("document")` → wire `DOCUMENT_NOT_FOUND`).

Output `GetSaleResponse` (`dto.rs:690-701`): `document: DocumentDto`
(`dto.rs:637-662`: `document_id, number, doc_type, status, warehouse_id,
subtotal_ht_centimes, tva_total_centimes, total_ttc_centimes,
paid_amount_centimes, change_amount_centimes, payment_status
(PAID|PARTIAL|UNPAID)` — all money as canonical strings), `items:
Vec<DocumentItemDto>` (`dto.rs:664-679`: product/unit ids, quantity_micro,
unit_price/line_tva/line_ttc centimes), `payments: Vec<PaymentDto>`
(`dto.rs:681-688`: method_code + amount_centimes), `print_status: String`.

This single command supplies **everything the conversion UI needs** for a
known id: human number, `doc_type` (DEVIS vs BL), `status` (FINALIZED vs
CONVERTED), `total_ttc_centimes` (required to build exact tenders), party
implication via payments, and line items.

Frontend status: **no wrapper exists** in `src/lib/ipc.ts` (grep for
`sales_get|ipcGetSale` returns zero hits outside comments). A future
wrapper must invoke with flat args `{ session_token, document_id }`
(matching the bare-scalar convention of `documents_get_print_status`,
already wrapped as `ipcGetPrintStatus` at `src/lib/ipc.ts:457-466`).

### `documents_get_print_status` — `ipc.rs:1533-1562`, registered `lib.rs:155`

Bare scalars `(session_token, document_id)` → `PrintStatusResponse
{ job_id, status, attempt_count }`. Supplementary only (print state, not
document lookup).

### `documents_reprint` — `ipc.rs:1564-1581`, registered `lib.rs:156`

`ReprintRequest { session_token, document_id }` → `ReprintResponse
{ job_id, document_id }`. Mutation, not retrieval.

## 2. Lookup by human number (e.g. `DEV-2026-000001`) — GAP

**No command exists.** Grep of `crates/commands/src/ipc.rs` for
`pub fn` yields no `*by_number*`, `*find*`, or `*lookup*` document handler;
no DTO carries a `number` input field (all document inputs are UUID
`document_id` strings). The `number` column is only ever **returned**
(`DocumentDto.number`, `sales_get`; conversion responses'
`new_document_number`). A cashier holding a printed DEVIS number today has
no IPC path to resolve it to an id.

## 3. List / search by type (DEVIS, BL) and status (FINALIZED) — GAP

**No command exists.** There is no `documents_list`, `documents_search`,
`sales_list`, or reporting query over documents by type/status. The nine
`reports_*` commands aggregate figures only (no document row listings).
The conversion UI therefore cannot today enumerate "all FINALIZED DEVIS"
or "all FINALIZED BL" candidates — it can only inspect ids obtained
out-of-band (e.g. the just-created number returned by a prior command, or a
UUID pasted/collected elsewhere).

## 4. Summary

| Capability | Exists? | Command / evidence |
|---|---|---|
| Get full document by UUID | ✅ | `sales_get` (`ipc.rs:1255`, `lib.rs:147`) → `GetSaleResponse` (`dto.rs:692`) |
| Get print state by UUID | ✅ (supplementary) | `documents_get_print_status` (`ipc.rs:1535`, `lib.rs:155`) |
| Enqueue reprint by UUID | ✅ (mutation) | `documents_reprint` (`ipc.rs:1566`, `lib.rs:156`) |
| Resolve human number → id | ❌ GAP | No handler; `number` is output-only |
| List/search by type+status (convertible candidates) | ❌ GAP | No handler; reports aggregate only |

**Recommended next backend step (before/with conversion UI):** expose a
read-only `documents_search` (filters: `doc_type`, `status`, optional
number substring; returns id/number/type/status/ttc/party per row, paginated
like the reports endpoints) so the UI can present convertible candidates
without out-of-band ids. Until then the conversion flow must start from a
known UUID (e.g. pasted id or the id returned by the originating command).
