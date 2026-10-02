# User flows

Status: v0.1 PROPOSED. Date: 2026-10-02. Baseline: 4c3a0341acc3a10f5a895aa8c6e4e923afe76856. Flows describe the intended behaviour of the rebuilt presentation layer. Lines marked ENGINE state what the current engine does (audits AUD-012, AUD-013, AUD-014). Lines marked OPEN are undecided.

## 1. Common rules

- C1 The counter is keyboard first: the search or scan field holds focus after every closed dialog in Vente, Enter confirms the primary action, Esc cancels.
- C2 Every dialog that asks for a PIN or a password clears the field when it closes, succeeds or fails. The UI never stores a PIN.
- C3 Destructive actions need typed or long-press confirmation (ADR-074).
- C4 Money uses the format of glossary rule R4. Digits are Latin in both languages.
- C5 A role never sees a control for something it can never do. A control that is allowed but temporarily impossible is shown disabled with one reason line (for example no open cash session).
- C6 The Caissier never sees a customer balance, a debt amount or a statement, except inside the approved collection form of FL-08.
- C7 Print failure never rolls back a sale or a payment. A failed job stays visible and can be reprinted from Documents.

## 2. Flow index

| ID | Flow | Actor | Depends on |
|---|---|---|---|
| FL-01 | Login and logout | All roles | - |
| FL-02 | Open the cash register | Caissier, Gerant, Admin | FL-01 |
| FL-03 | Cash sale | Caissier (also Gerant, Admin) | FL-02 |
| FL-04 | Credit sale | Caissier | FL-03, FL-05 |
| FL-05 | Quick new customer | Caissier (also Gerant, Admin) | FL-01 |
| FL-06 | Hold and resume a sale | Caissier | FL-03, BRK drafts task |
| FL-07 | Return with manager approval | Caissier with Gerant present | BRK approval task |
| FL-08 | Debt collection by the Caissier | Caissier with Gerant present | FL-02, BRK approval task |
| FL-09 | Debt collection by a manager | Gerant, Admin | FL-02 |
| FL-10 | Reprint a document | Caissier, Gerant, Admin | FL-01 |
| FL-11 | Close the cash register | Caissier, Gerant, Admin | FL-02 |
| FL-12 | Fast product entry | Gerant, Admin | FL-01 |

## 3. Flows

### FL-01 Login and logout

- Actor: any user.
- Precondition: the app is open and a database is available.
- Steps:
  1. The user enters the user name and the password; Enter submits.
  2. On success the shell opens in the saved language and theme (French and light by default) with the modules allowed for the role (visibility matrix of the IA document).
  3. Se déconnecter returns to the login screen and clears all session data from the screen.
- Failures: wrong credentials show one generic message that does not say which part was wrong; a locked account shows the remaining wait time.
- ENGINE: password login locks after 5 failed attempts for 300 seconds. AUD-014 found no audit row for failed logins (search of one file only, to be re-checked in the approval task).
- OPEN: none.

### FL-02 Open the cash register

- Actor: Caissier, Gerant, Admin.
- Precondition: signed in; no open cash session.
- Steps:
  1. Vente shows a banner with one button Ouvrir la caisse; Caisse & Clôture offers the same action.
  2. The user enters the Fond de caisse and confirms.
  3. The session opens and focus returns to the scan field.
- Failures: an empty, invalid or negative amount is refused inline.
- ENGINE: sales_checkout and credit_record_payment need an open cash session.
- OPEN: whether the engine allows one open session per user or one per machine (to be confirmed by audit).

### FL-03 Cash sale

- Actor: Caissier (also Gerant, Admin).
- Precondition: FL-02 done.
- Steps:
  1. The user scans a barcode or types in the search field; each scan adds one unit, and a repeated scan increases the quantity of the same line.
  2. The cart shows each line with quantity, unit price and line total, and a running total.
  3. The user may change a quantity with the keyboard or remove a line.
  4. Encaisser opens the payment panel; the user enters the Montant reçu in Espèces; Monnaie à rendre is displayed.
  5. When cash is received the Timbre is shown as its own line and included in the total (see OPEN on preview).
  6. Enter confirms. The ticket is printed, a new empty cart opens and focus returns to the scan field.
- Failures: an unknown barcode shows Aucun résultat with the option to search by name; an insufficient cash amount keeps the panel open; without an open cash session Encaisser is disabled with one reason line.
- ENGINE: the timbre is computed by the engine as recorded in AUD-014 (Ticket and Facture only, cash tendered above 0, not legally validated). The print job is enqueued in the same transaction as the sale.
- OPEN: keyboard shortcut assignments; whether a pricing preview command exists so the Timbre can be shown before confirmation (otherwise it is shown on the result); real scanner keyboard wedge behaviour (unmeasured).

### FL-04 Credit sale

- Actor: Caissier.
- Precondition: FL-03 steps 1 to 3 done.
- Steps:
  1. In the payment panel the user opens the customer picker and selects a customer or starts FL-05.
  2. The credit HUD shows ONE word with an icon: Autorisé or Refusé. No amount and no reason.
  3. If Autorisé the user enters the part paid in Espèces; the remainder is À crédit; Encaisser confirms.
  4. If Refusé the À crédit part is disabled; the user pays in full by another method or asks the manager.
  5. The receipt prints a credit payment line; it prints neither the customer nor the balance (current behaviour).
- Failures: every engine refusal (limit exceeded, credit disabled, inactive customer) is shown to the Caissier only as Refusé.
- ENGINE: a customer is required when credit is above 0; a Limited policy refuses when balance plus credit exceeds the limit unless CreditLimitOverride; a Disabled policy or an inactive customer refuses. customers_get returns the balance to every role today, so a BRK task must provide a status-only answer for the HUD.
- OPEN: how the manager overrides a refused limit at the counter (CreditLimitOverride in his own session or approval in place); whether the HUD can be computed before the credit amount is typed.

### FL-05 Quick new customer

- Actor: Caissier (also Gerant, Admin).
- Precondition: the customer picker is open in Vente.
- Steps:
  1. The user selects Nouveau client; a small form asks for Nom and Téléphone only.
  2. Enter saves; the new customer is selected in the sale and the picker closes.
- Failures: name shorter than 2 or longer than 100 characters, bad phone format, or a phone already used; for a duplicate phone the form offers to select the existing customer without showing any balance.
- ENGINE: the name is trimmed to 2..100 characters; the phone is digits only, mobile 0[5-7] plus 8 digits (10) or landline 0[2-4] plus 7 digits (9); the phone is unique (CUSTOMER_PHONE_DUPLICATE). customers_create returns the balance to every role today, so the UI must not display it and a BRK task must route it.
- OPEN: whether the phone is mandatory in the quick form.

### FL-06 Hold and resume a sale

- Actor: Caissier.
- Precondition: a cart with at least one line; the BRK drafts task is done.
- Steps:
  1. Mettre en attente saves the cart as a draft and opens a new empty cart.
  2. A list of held carts (Brouillon) in Vente shows each draft with its time and total.
  3. Reprendre loads a draft into the cart; if the current cart is not empty the user must hold or discard it first (discard needs typed or long-press confirmation).
- Failures: a line whose product was deactivated or is no longer available is flagged on resume.
- ENGINE: the drafts list and resume are not available yet (planned BRK task).
- OPEN: draft expiry; whether a draft can hold a customer.

### FL-07 Return with manager approval

- Actor: Caissier with the Gerant present.
- Precondition: signed in; Documents module open.
- Steps:
  1. The user finds the original document in Documents and selects it.
  2. Retour/Avoir opens the dialog Autorisation du gérant first; the Gerant types his PIN there.
  3. If the PIN is accepted the return form opens; the user selects lines and quantities and confirms. The Caissier session stays active.
  4. The return is processed under the Gerant's authority (audit with both user ids, planned) and the credit note (Avoir) is issued and printed.
  5. If the PIN is wrong the dialog shows one generic message, clears the field and counts the attempt.
- Failures: a locked PIN shows a wait message; cancelling returns to the document with nothing processed.
- ENGINE: returns_process needs ProcessReturn (Gerant, Admin only). The PIN has no attempt counter, no lockout and no production way to set it today. No approval record exists today. Timbre on returns is fixed at 0. AVOIR uses the same number allocator as the other document types.
- OPEN: the refund method for a credit-sale return (reduce the customer debt or pay cash); the age limit of a returnable document; validity of an approval (proposed: single use, short validity).

### FL-08 Debt collection by the Caissier

- Actor: Caissier with the Gerant present.
- Precondition: FL-02 done; the BRK approval task is done.
- Steps:
  1. The user selects a customer and presses Encaissement de dette (entry point OPEN).
  2. The dialog Autorisation du gérant opens first; the Gerant types his PIN. No amount is visible before approval.
  3. After approval the collection form opens and shows the customer name and the debt amount; this is the only place where the Caissier sees it.
  4. The user enters the amount received in Espèces and confirms.
  5. The payment is recorded and the proof (Preuve d'encaissement) prints for the customer.
  6. Closing the form removes every amount from the screen.
- Failures: an amount above the debt is refused for the Caissier with a message that the Gerant must record it; a zero or negative amount is refused inline; a print failure does not undo the payment (C7).
- ENGINE: credit_record_payment needs RecordCreditPayment and an open cash session (the planned permission split withdraws RecordCreditPayment from the Caissier, so the approved path then runs under the Gerant's authority); ManageCustomerCredit is needed for an overpayment. It writes the ledger, the balance and a cash movement in one transaction, writes no audit row, creates no document and has no reversal. A proof needs a print job without a document today, which needs a migration or a documents row.
- OPEN: the entry point (Caisse & Clôture or the customer picker in Vente); proof numbering, remaining balance and timbre on the proof (accountant questions, questionnaire section G); the overpayment rule (proposed, unconfirmed); whether a method other than cash is allowed.

### FL-09 Debt collection by a manager

- Actor: Gerant, Admin.
- Precondition: FL-02 done.
- Steps:
  1. In Clients & Crédit the user opens the customer and the Relevé de compte (statement).
  2. The user presses Encaissement de dette; no approval dialog appears because the manager holds the permission.
  3. The user enters the amount and confirms; an overpayment follows the ManageCustomerCredit permission.
  4. The proof (Preuve d'encaissement) prints for the customer.
- Failures: as FL-08.
- ENGINE: as FL-08.
- OPEN: proof numbering, remaining balance and timbre on the proof (same as FL-08).

### FL-10 Reprint a document

- Actor: Caissier, Gerant, Admin (permission ReprintDocument).
- Precondition: signed in.
- Steps:
  1. In Documents the user searches by number, date or type.
  2. The user opens the document and presses Réimprimer.
  3. The reprint carries the DUPLICATA mark.
- Failures: an unavailable printer leaves the job FAILED with a retry button.
- ENGINE: item designations are not snapshotted, so a renamed product changes an old reprint; a retried RECEIPT job (attempt count above 0) also prints as DUPLICATA.
- OPEN: whether to snapshot item designations (BRK task).

### FL-11 Close the cash register

- Actor: Caissier, Gerant, Admin.
- Precondition: an open cash session.
- Steps:
  1. In Caisse & Clôture the user presses Clôture de caisse.
  2. The user enters the counted cash; the screen then shows the Écart.
  3. Confirmation uses long-press or typed confirmation (ADR-074).
  4. The Rapport de clôture prints and the register shows the closed state.
- Failures: held drafts or a failed print job are listed as warnings and do not block closing (PROPOSED).
- ENGINE: the closing report has no number today.
- OPEN: whether the Caissier sees the expected cash before counting (blind count proposed); the exact fields of the engine closing command (to be audited).

### FL-12 Fast product entry

- Actor: Gerant, Admin.
- Precondition: signed in; Produits & Codes-barres open.
- Steps:
  1. The user presses Nouveau produit; the form opens with focus on the barcode field so a scan fills it.
  2. Nom en français is required; Nom en arabe (alias) is optional.
  3. The user fills Prix de vente, Prix d'achat, Catégorie, Unité and Stock minimum (field set PROPOSED; ADR-077 D1 governs on any conflict).
  4. Enregistrer saves and opens an empty form so entry continues quickly.
- Failures: a normalized duplicate name is refused in manual entry as well; the dialog offers open existing or change name; there is no warning-only mode.
- ENGINE: not audited here; the duplicate rule is an owner decision.
- OPEN: field order, default unit, barcode generation for products without one.

## 4. Not yet specified

- Import and export (ADR-077).
- Backup and restore.
- User management, PIN set and reset.
- Purchasing and reception of goods.
- Inventory count.
- Reports.
- Customer management and statements.
- Categories.
- Printer setup and test print.
- First launch and shop identity.

## 5. Known gaps in the information architecture document

- IA row J03 reads as a path valid for managers only; the Caissier entry point for debt collection is OPEN (FL-08). The IA fixes are made in the next task that edits the IA document.
