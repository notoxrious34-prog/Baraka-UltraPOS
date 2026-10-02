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
- The design document docs/design/01-information-architecture.md (v0.4) carries the visibility matrix and the approval flow.
- Settled by Amendment 1: customer creation at the counter and the credit HUD word. The visual details of both are settled in the prototypes.

## Amendment 1 (2026-10-02, owner answers)
1. The Caissier may create a new customer (name and phone only) at the counter, from the customer picker in Vente. CreateCustomer is already held by the Caissier, so no engine change is needed for creation. Editing customers stays in Clients & Crédit (Gérant, Admin) at UI level; the engine also lets the Caissier update a customer through customers_update (same permission), and whether to split it is left to the permission-split BRK task.
2. At a credit sale the Caissier sees one word only (allowed / not allowed) and no debt amount. The engine returns balances to every role, so this is a UI rule; the permission-split BRK task may add a status-only command so that the amount never reaches the Caissier session.
3. Open: debt payment while no manager is present. Until the owner answers, the Caissier cannot collect (Decision 2). The approval mechanism of Decision 3 could later cover it. [Superseded by Amendment 2 for the debt-payment part.]

## Amendment 2 (SESSION-ARC-004, reply 1): debt collection by the Caissier requires manager approval in place

Status: items 1-3 ACCEPTED (owner decision, SESSION-ARC-003 reply 10). Items 4-6 PROPOSED or OPEN until the owner answers.

1. Rule. The Caissier may record a customer debt payment (command credit_record_payment) ONLY inside a manager-approved context. The manager types his PIN in the dialog "Autorisation du gerant". The mechanism is the same as for returns: a single-use approval, bound to ONE permission (RecordCreditPayment) and ONE target (the customer id), valid for a short time, audited with BOTH user ids. The cashier session is never switched. The UI never stores the PIN. Attempt limits and approval lifetime are decided in the BRK task that implements the mechanism.

2. Gerant and Admin collect debt payments directly in their own session, with no approval step. Customer statement viewing stays manager-only (Gerant, Admin) and the Caissier never sees a statement, approved or not.

3. Engine consequence (future BRK work, nothing implemented now). Today every role holds ViewCustomers and the Caissier holds RecordCreditPayment in Role::permissions(). The BRK task that builds the approval mechanism must (a) implement it for ProcessReturn first, then for credit_record_payment; (b) withdraw RecordCreditPayment from the Caissier's own permission set while keeping the approved path; (c) change the role-matrix tests in that same task; (d) add a new read permission for statements so ViewCustomers no longer implies statement access; (e) write an audit_log row for every credit payment (none is written today, per AUD-013). PIN set/reset in the Utilisateurs module is a prerequisite.

4. PROPOSED, not yet confirmed by the owner: "approval first" ordering. The collection form opens only after the manager's PIN is accepted, in the manager-authorized context.

5. OPEN, owner question Q-B: whether the cashier may see the customer's debt amount inside that approved form while the manager is present. Proposal: yes, inside the form only, nowhere else.

6. OPEN, owner question Q-A: whether a customer receives a printed proof when paying a debt (the engine prints nothing and creates no document for it today). OPEN, design question: the cashier's entry point for an approved collection (Caisse & Cloture or the customer picker in Vente), because the visibility matrix hides Clients & Credit from the Caissier.
