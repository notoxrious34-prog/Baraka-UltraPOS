# Component inventory

Status: v0.1 PROPOSED. Date: 2026-10-04. Baseline: 716c4a0a75aed3aa385753a95fc8ade2d44e506d. Component names here are working identifiers for the design and prototype work; they are not yet final code names. Each entry lists the states it must support, defined in docs/design/03-state-catalog.md, and the tokens it draws from docs/design/04-design-tokens.md.

## 1. Rules

- CI-R1 A component is listed once; a screen composes components, it never redefines one.
- CI-R2 Every component that shows data implements the generic states it needs from the state catalog (section 2 there); this inventory names which ones apply.
- CI-R3 No component uses position: fixed except Modal shell, Toast and Command palette (ADR-067, token section 7).
- CI-R4 Every component mirrors for Arabic (RTL) through logical properties only; no component has a separate RTL variant.
- CI-R5 A component that shows money uses the Money display primitive; no other component formats an amount on its own.

## 2. Primitives

| ID | Name | Purpose | Key states | Token refs |
|---|---|---|---|---|
| P01 | Button | Primary, secondary, danger and ghost actions | G07 Saving, G06 Forbidden (not rendered) | color-accent, color-danger, radius-md, duration-fast |
| P02 | Text input | Single-line text entry | G09 Validation error | color-border, color-focus-ring, radius-sm |
| P03 | Number input | Quantity and money entry, Latin digits only | G09 Validation error | font-feature-money, color-border |
| P04 | Phone input | Algerian phone formats (FL-05) | G09 Validation error | color-border |
| P05 | Select | Single choice from a short list | G09 Validation error | color-border, radius-sm |
| P06 | Toggle | Boolean preference (for example theme) | - | color-accent |
| P07 | Badge | Short status word plus icon, never colour alone | D04, D09, D10 | color-success, color-danger, color-warning, color-info |
| P08 | Icon | Paired with every semantic colour (rule S-R7) | - | - |
| P09 | Toast | Transient feedback, auto-dismiss | G08 Saved, G04 Error retryable | z-toast, elevation-2, duration-normal |
| P10 | Tooltip | Short help text on focus or hover | - | elevation-1, z-dropdown |
| P11 | Skeleton row | Loading placeholder for a list or table | G01 Loading | - |
| P12 | Data table | Sortable, scrollable list of records | G01, G02, G03, G04 | density-row-height |
| P13 | Tabs | Switch between related views without navigation | - | color-accent |
| P14 | Field label | Label plus optional required mark for a form field | G09 Validation error | color-text-secondary |
| P15 | PIN pad | Digit entry for a PIN, Latin digits only, never stores the value | D05 | radius-md, color-border |
| P16 | Money display | Formatted amount per glossary rule R4 | - | font-feature-money |
| P17 | Confirmation control | Typed-text or long-press confirmation (ADR-074) | G11 | duration-slow |

## 3. Composite components

| ID | Name | Purpose | Flows | Key states |
|---|---|---|---|---|
| C01 | Navigation rail | Module navigation, collapsed or expanded by tier | FL-01 | - |
| C02 | Scan and search bar | Barcode scan and text search, always focused by default | FL-03 | G03 Empty no result |
| C03 | Cart | List of sale lines with quantity and totals | FL-03, FL-06 | D02 Cart |
| C04 | Payment panel | Tender entry, change, timbre line, credit split | FL-03, FL-04 | D03 Payment panel |
| C05 | Credit HUD badge | One word plus icon, no amount, no reason | FL-04 | D04 Credit HUD |
| C06 | Customer picker | Search and select a customer, or open Quick new customer | FL-04, FL-05 | G01, G03 |
| C07 | Quick new customer form | Name and phone only | FL-05 | G09 Validation error |
| C08 | Draft list | Held carts with time and total | FL-06 | G02 Empty first use |
| C09 | Approval dialog (Autorisation du gérant) | Manager PIN entry before a sensitive action | FL-07, FL-08 | D05 Approval dialog |
| C10 | Collection form | Debt amount and payment entry, shown only after approval | FL-08, FL-09 | D06 Collection form |
| C11 | Document list | Search, filter and open documents | FL-10 | G01, G03 |
| C12 | Document viewer | Read a document and trigger Réimprimer | FL-10 | D07 Print job |
| C13 | Cash session banner | Open-register prompt when no session is open | FL-02 | D01 Cash session |
| C14 | Close-register form | Counted cash and écart entry | FL-11 | G11 Destructive confirmation |
| C15 | Product quick-entry form | Fast product creation | FL-12 | G09 Validation error |
| C16 | Return form | Line and quantity selection for a return | FL-07 | - |
| C17 | Modal shell | Frame for every dialog, overlay and focus trap | FL-07, FL-08, all modals | z-modal, elevation-3 |
| C18 | Command palette | Keyboard-first command access (ADR-067) | - | z-command-palette |

## 4. Relation to the existing modals

- CI-M1 AUD-012 and AUD-013 enumerated 18 modals in the current code and left a 19th unidentified (ModalShell hypothesis recorded, unconfirmed).
- CI-M2 Mapping each of those 18 modals one by one to the components of section 3 is a later audit task; this inventory only fixes the target shape, it does not claim the mapping is done.
- CI-M3 Component C17 Modal shell is the working name for the ModalShell hypothesis; it is not confirmed as the 19th modal.

## 5. Open points

- O1 Final code names for every component are set in the implementation task, not here.
- O2 Whether Tabs (P13) is needed anywhere in v1 or only Document list filters use plain buttons.
- O3 The exact field set of the Return form (C16) depends on the BRK approval task.
