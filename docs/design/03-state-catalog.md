# State catalog

Status: v0.1 PROPOSED. Date: 2026-10-02. Baseline: 4c3a0341acc3a10f5a895aa8c6e4e923afe76856. Row numbers refer to docs/design/glossary-ar-fr.md. Flow identifiers refer to docs/design/02-user-flows.md.

## 1. Rules

- S-R1 Every data view implements loading, empty, error and ready states from this catalog. No view invents its own.
- S-R2 Loading is shown only if the wait exceeds 200 ms (PROPOSED) to avoid flicker. Lists use skeleton rows; the counter never shows a full-screen spinner.
- S-R3 A control is disabled only for a temporary condition and always shows one reason line. A control the role can never use is not rendered (flow rule C5).
- S-R4 Messages are written in the active language from the message catalog (later task). Raw engine error codes are never shown.
- S-R5 The Caissier never receives an amount, a balance or a reason for a credit refusal (flow rules C6 and FL-04).
- S-R6 A state change never moves the cart or the payment panel (no layout jump while scanning).
- S-R7 Colour is never the only signal: every state has an icon or a word.

## 2. Generic states

| ID | State | When | Display | Interaction |
|---|---|---|---|---|
| G01 | Loading | Data requested and slower than 200 ms | Skeleton rows or placeholder blocks | Controls acting on the data are disabled |
| G02 | Empty, first use | No record exists yet | One short sentence and one primary action | Primary action focused |
| G03 | Empty, no result | A search or filter returns nothing | Aucun résultat (row 110) and the search words | Clear-filter action |
| G04 | Error, retryable | A read failed | Message and a Réessayer button | Retry keeps filters and scroll position |
| G05 | Error, blocking | Database unavailable or damaged | Full-view message, no data, instruction to call the owner | None; the app does not continue |
| G06 | Forbidden | The role lacks the permission | The action is not rendered; if reached by a shortcut, one neutral message | Return to the previous view |
| G07 | Saving | A write is in progress | The primary button shows progress | All submit actions disabled (no double submit) |
| G08 | Saved | A write succeeded | Toast Opération réussie (row 112) or a specific line, auto-dismiss | The toast never takes focus |
| G09 | Validation error | A field is invalid | Message under the field, field marked, focus on the first invalid field | Correct and resubmit |
| G10 | Conflict | The record changed since it was loaded | Message and a Recharger button | Reload shows the current record |
| G11 | Destructive confirmation | Delete, cancel or discard | Typed or long-press confirmation (ADR-074) | Esc cancels |
| G12 | Degraded viewport | Viewport tier T0 of the viewport matrix | One dismissible notice recommending a lower display scale | Dismissal lasts for the session |

## 3. Domain states

| ID | Domain | States | Notes |
|---|---|---|---|
| D01 | Cash session | None open, Open, Closing, Closed | None open disables Encaisser and Encaissement de dette with the reason line |
| D02 | Cart | Empty, Has lines, Paying, Paid, Held | Paid returns to Empty with focus on the scan field; Held is a draft (FL-06) |
| D03 | Payment panel | Idle, Cash entered, Credit entered, Insufficient, Ready, Submitting, Failed | Insufficient keeps Encaisser disabled and shows the missing amount |
| D04 | Credit HUD | Unknown, Autorisé, Refusé | Unknown while no customer is selected or while loading, and then no word is shown; only one word (row 53 or 54) is ever shown |
| D05 | Approval dialog | Idle, Verifying, Accepted, Wrong PIN, Locked, Cancelled | The PIN field is cleared at every state change; Wrong PIN and Locked show a generic message; Locked shows the remaining time only |
| D06 | Collection form | Closed, Open after approval, Submitting, Done, Failed | The debt amount exists only in Open after approval and Submitting; closing clears it (FL-08) |
| D07 | Print job | PENDING, PROCESSING, DONE, FAILED | Engine states; FAILED shows a Réimprimer action; a retried RECEIPT job also prints DUPLICATA |
| D08 | User | Active, Locked | Locked after 5 failed logins for 300 seconds (engine); the remaining time is shown |
| D09 | Product stock | In stock, Below minimum, Rupture de stock | Rows 71 and 72; always a word plus an icon, never colour alone |
| D10 | Printer | Ready, Unavailable, Unknown | Unknown until the printer verification gate (58 and 80 mm) is closed; no claim of readiness is shown |

## 4. Error mapping (PROPOSED)

| ID | Engine condition | Caissier sees | Gerant and Admin see |
|---|---|---|---|
| E01 | Credit refused (limit exceeded, credit disabled, inactive customer) | Refusé only | Refusé with the specific reason |
| E02 | CREDIT_CUSTOMER_REQUIRED | Select a customer to sell on credit | Same |
| E03 | INVALID_PHONE_FORMAT | Format hint: mobile 10 digits starting 05 to 07, landline 9 digits starting 02 to 04 | Same |
| E04 | CUSTOMER_PHONE_DUPLICATE | Offer to select the existing customer, no balance shown | Offer to select the existing customer |
| E05 | A print job ends FAILED | Print failed, reprint from Documents | Same |
| E06 | No open cash session | Disabled controls with the reason line | Same |
| E07 | Wrong or locked PIN | Generic message, field cleared | Same |
| E08 | Permission denied | One neutral message | Same |

## 5. Open points

- O1 The timing values (200 ms) are proposals to be tuned on the shop PC.
- O2 The wording of every message is defined later in the message catalog, with owner review of the Arabic.
- O3 Mapping the 18 existing modals to the dialogs of this catalog is a later task (a 19th modal is unidentified).
