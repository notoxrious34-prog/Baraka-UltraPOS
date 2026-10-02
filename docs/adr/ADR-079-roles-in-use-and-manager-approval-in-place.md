# ADR-079: Roles in use and manager approval in place

Status: ACCEPTED (direction). Engine parameters (approval lifetime, attempt limits, storage, audit fields) are fixed in the BRK task that implements it.
Date: 2026-10-02
Deciders: Architect (Charter v2.0), owner answers of 2026-10-02.

## Context
AUD-012 verified the permission required by each of the 72 IPC commands. The owner answered: the employee receives money and opens and closes the cash session; the employee only chooses the customer when selling and neither views debts nor collects them; returns are approved by the owner without switching the user. The engine today has no in-place approval, a PIN check with no IPC command, and it lets every Caissier view customer statements, collect debts and create customers.

## Decision
1. The shop's employee uses the Caissier role. The Vendeur role stays in the engine and is not assigned today.
2. Viewing a customer statement and collecting debt payments are manager functions (Gérant, Admin). The UI hides them from the Caissier now; a later BRK task enforces this in Rust by splitting the permission (a new read permission for the credit statement; RecordCreditPayment withdrawn from the Caissier). Any change to the existing role matrix tests is made in that task.
3. Restricted actions may be authorized in place by a manager credential. The first action is ProcessReturn. The manager enters a PIN at the counter; the engine verifies it and issues a single-use approval bound to one permission and one target, valid for a short time, recorded in the audit log with both user identifiers. The requesting user session is never switched.
4. Rust remains the authority: no approval without a PIN set on the approving account; failed attempts are limited; an approval cannot be reused, transferred or persisted across a restart; the UI never stores the PIN.
5. Prerequisite: PIN set and reset for users, delivered with the Utilisateurs engine task. The existing fast-switch PIN check (VerifyPinUseCase) may be reused but is not exposed today.

## Consequences
- New BRK tasks: manager approval in place (first for ProcessReturn), permission split for credit statement and debt collection, tightening of catalog_define_kit after confirmation, PIN management in Utilisateurs.
- The design document docs/design/01-information-architecture.md (v0.2) carries the visibility matrix and the approval flow.
- Settled by Amendment 1: customer creation at the counter and the credit HUD word. The visual details of both are settled in the prototypes.

## Amendment 1 (2026-10-02, owner answers)
1. The Caissier may create a new customer (name and phone only) at the counter, from the customer picker in Vente. CreateCustomer is already held by the Caissier, so no engine change is needed for creation. Editing customers stays in Clients & Crédit (Gérant, Admin) at UI level; the engine also lets the Caissier update a customer through customers_update (same permission), and whether to split it is left to the permission-split BRK task.
2. At a credit sale the Caissier sees one word only (allowed / not allowed) and no debt amount. The engine returns balances to every role, so this is a UI rule; the permission-split BRK task may add a status-only command so that the amount never reaches the Caissier session.
3. Open: debt payment while no manager is present. Until the owner answers, the Caissier cannot collect (Decision 2). The approval mechanism of Decision 3 could later cover it.
