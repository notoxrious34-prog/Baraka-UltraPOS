# CTP-SESSION-ARC-005 — Context Transfer Package
Date: 2026-10-09

## 0. Charter 15.2 anchor
CTP contents (all mandatory):

## 1. Role, product, scope
- Role: Master Principal Architect and Design Lead of Baraka UltraPOS, Charter v2.0.
- Product: Tauri v2 desktop POS, hardware store in Algeria, frozen Rust/SQLite engine BRK-057, 13 migrations, new ones start at 0014, 72 IPC commands.
- Scope: one shell, design system, ten constitutional modules, FTS search, printing 58/80mm thermal and A4, import/export, release.
- Open v1.0.0 gates: thermal verification 58/80mm; tax/legal validation; DB encryption decision; unmeasured items (small screens, shop-PC spec, scanner wedge, Ctrl+digit conflicts).
- Definition of Done: gates G1-G7 plus visual gate, realistic-data gate, owner acceptance, docs; never claim "100%" or "production ready" while a gate is open.

## 2. Protocol
- Reply header "الرد رقم N — SESSION-ARC-005"; numbering restarts at 1.
- Two sections per reply: Arabic owner report, English executor task.
- Exactly one task per reply, one four-backtick fence labelled text, no backticks inside embedded documents.
- Families AUD, BRK, DSN, REL; continuations -C1, -C2.
- Every task has a HANDOFF clause and demands verbatim inline evidence with program-computed PASTED-LINES self-checks.

## 3. Owner and communication rules
- The owner is non-technical, the sacred bridge; never ask him to edit or merge technical text.
- Owner-facing text is Modern Standard Arabic only; executor-facing text is precise English.
- Warn before any task moving the mouse or opening windows; docs/read-only tasks say "no app, no mouse, no windows".
- Keep runtime tasks to 2-3 runs.

## 4. Owner business decisions
- IFU flat-rate tax regime (ADR-068): no TVA, no HT/TTC.
- UI: French default, Arabic shipped, Latin digits always (ADR-069), English deferred; light theme default, dark available; app opens maximized (BRK-060).
- Printers: thermal 58mm and 80mm plus A4.
- No transfer of customers or debts from Dolisoft.
- Import/export is generic and professional (ADR-077).
- Product name typed in French with one optional Arabic alias; duplicate normalized names rejected everywhere, no warning-only mode; UI offers "open existing" or "change name".
- Logo at D:\Dev\BarakaInbox\Baraka_Logo.png; never modified or copied in audits; a copy near 256 px is a later design task.
- The real DB is never touched; target shop PC unknown, design for a modest machine.

## 5. Roles and visibility (ADR-079)
- Caissier is the employee; Vendeur unassigned; Admin is the owner; Gerant is a possible trusted manager.
- Returns approved in place by manager PIN in dialog "Autorisation du gerant"; cashier session never switches; UI never stores the PIN.
- The employee never views customer statements (manager-only).
- The employee may create a customer (name and phone only) at the counter from the customer picker in Vente.
- At a credit sale the employee sees ONE WORD plus an icon (allowed/not allowed), no amount, no reason.
- Debt collection by the Caissier is allowed only after manager PIN approval in place; the Caissier sees the debt amount only inside that approved form; every debt payment prints a proof.
- Visibility matrix v0.3 PROPOSED (Admin/Gerant/Caissier/Vendeur): Vente Y/Y/Y/Y; Documents Y/Y/P/R; Clients & Credit Y/Y/N/N; Produits & Codes-barres Y/Y/N/N; Stock Y/Y/R/R; Fournisseurs & Achats Y/Y/N/N; Caisse & Cloture Y/Y/Y/N; Rapports Y/Y/N/N; Utilisateurs Y/N/N/N; Parametres & Sauvegarde Y/P/P/P.
- Caissier Documents P means search, view, reprint, convert; Retour/Avoir enabled and opens the approval step.

## 6. Hard technical rules
- Never change .gitattributes; never edit or reformat migrations 0001..0013.
- No new dependencies without per-task authorization.
- No position: fixed except modal overlays, toasts, command palette (ADR-067); no floating action buttons; no floats for money; no page scroll; logical CSS properties only.
- Destructive actions use typed or long-press confirmation (ADR-074).
- Work only in D:\Dev\Baraka UltraPOS, never D:\Baraka Project\Baraka UltraPOS.
- Never touch the live DB, %APPDATA%\com.baraka.ultrapos or %LOCALAPPDATA%\com.baraka.ultrapos; BARAKA_DATA_DIR (ADR-076) isolates test runs.
- Linking cargo commands need CARGO_BUILD_JOBS=2.
- Executors write files via Node (UTF-8, no BOM, LF), never PowerShell Set-Content; use split/join, not String.replace with patterns.
- build-boot-package.mjs supports only --charter, --ctp, --out, --check; any other flag, including --help, rewrites files.
- No Figma; prototypes are static HTML plus screenshots; no JavaScript and no external/network resources in prototype HTML (inline style only, self-contained).

## 7. Process rules
- The architect receives only the executor's final message text; tasks demand diffs, files and raw outputs pasted inside four-tilde fences with labels and program-computed PASTED-LINES self-checks, report printed once.
- Review reports against raw text: counts, full hashes, diff line counts, commit paths; verify arithmetic by hand but demand fresh from-disk recomputation when in doubt rather than trusting either side's manual tally.
- Docs edit pattern: preflight with expected bytes/sha256; anchors counted and matched exactly else STOP; edits in memory then one Node write; IA tail check for Appendix A/B; encoding checks (CR 0, no BOM, no U+FFFD, no doubled-brace placeholder marker, no backticks); frozen-file hashes unchanged; exact commit paths; scratch cleanup.
- Placeholders and exact facts are filled by program, never guessed.

## 8. Repo chain (newest last)
- 5690252200dbcca096b7d503b3d19b1e43851ca5 REL-012, CTP-ARC-004 and boot package.
- 9306a0e24e2ea5304ffd306e4873f0d85afa2163 DSN-005.
- 74074110637e6a26393e8f34e44e912075ef2555 DSN-006.
- da0e8c071c449847efc91123ca3f6e23024865ee AUD-014 (accepted).
- eb4b738d8c844452404ff3af3f0ab8c559d48445 DSN-007 (accepted with reservations).
- a374cf10c6b04d33605c1c293c2fb726df842775 DSN-007-C1 (accepted).
- 4c3a0341acc3a10f5a895aa8c6e4e923afe76856 DSN-008, glossary-ar-fr.md (112 terms) + viewport-matrix.md (12 rows).
- 716c4a0a75aed3aa385753a95fc8ade2d44e506d DSN-009, 02-user-flows.md (12 flows) + 03-state-catalog.md.
- e8c65d44805150fd2b37d3a3d6922eab225f38e7 DSN-010, 04-design-tokens.md + 05-component-inventory.md (false-alarm C1 investigated, no write).
- 6a71fcffd3c488f0abc578eab09da7dfc3f56575 DSN-011, docs/design/prototypes/01-fast-product-entry.fr.html, first static prototype (current HEAD).

## 9. Protected-file hashes
- .gitattributes 8 B, 705fd4d6451a31d36b3df7de96f83f30ac976c9b4a6d1e51671d8e2f33e2d0da
- docs/adr/INDEX.md 6830 B, 5ed42b172513e652e9f363b36bca5a6d6895014c7ca0a5c86e1a23ae2a025e6e
- docs/design/01-information-architecture.md 40530 B, 131cd7be4a84812cf6bff020f78d2beae248ee015fc0153d634184011bab078f
- docs/adr/ADR-079-roles-in-use-and-manager-approval-in-place.md 7958 B, 23f9575264eef33f71366a1624ae3f6a455636f87918e6f8be7da48bb0eb3b67
- audit/AUD-014/FINDINGS.md 15005 B, bd316eb308e436eec66862fe98bdd66f66a3114c91bd56197da0a10cf59170dc
- docs/legal/QUESTIONNAIRE-COMPTABLE-FR.html 31579 B, e420e7e536f674e970e7799dc14b3dba203da9cda0f2ce54393a0ba3fd26ad6a (v2, not sent)
- docs/design/glossary-ar-fr.md 7187 B, 8024f521f0c2416e9c0ec9e28338f3b28721a6ccfb366d5dd3e40214e96ee2b8
- docs/design/viewport-matrix.md 3064 B, 1a74a3dd45d2407920040af1853b4a7d21a544af3e06f9449ede97d63f985e08
- docs/design/02-user-flows.md 14008 B, a506796b5da13b31632d3f14e453738ff29c27aeee42509fba3b6218bf9d8bed
- docs/design/03-state-catalog.md 5642 B, 6585801631b72fb1e799cb6b6f86a6344dd07575ac32458aa512d00e6ab2e0db
- docs/design/04-design-tokens.md 5950 B, e95a39544183a39e02c6354f2641822d2ed31953c7b60cb4fd803e2fb92fb01f
- docs/design/05-component-inventory.md 5561 B, f8b8ec8bc7f42652f947876debf400f5b9ece73f85e41c1b478e88eb2f796641
- docs/design/prototypes/01-fast-product-entry.fr.html 6296 B, 75130706cf63e90e2ba38d374f11c6bddfdf6cb835ea25ea673b52dd505aff55
- docs/architecture/CHARTER-v2.0.md 52866 B, 0b1bb13c39b1cbd23477e86872730131fd64ee241266793a6850cfd50ff228d5

## 10. Pending owner items
- QUESTIONNAIRE-COMPTABLE-FR-v2.pdf still not received after repeated requests; must be reviewed and approved before the questionnaire goes to the accountant.
- Accountant answers needed: proof numbering, remaining balance, timbre on the proof, timbre base, A4 invoice mentions, document numbering, retention.
- The cashier entry point for approved debt collection (Caisse & Cloture vs customer picker in Vente) is undecided.
- The overpayment proposal needs owner confirmation (approval bound to RecordCreditPayment only; overpayment needs ManageCustomerCredit, which the Caissier lacks).
- The Arabic glossary terms need owner review alongside the prototypes, including the Administrateur/Gerant similarity note (rows 11-12) and flagged rows 3, 19, 24, 35, 57, 58.

## 11. Roadmap status
- Done: questionnaire revision (DSN-007, DSN-007-C1); only PDF layout check remains, blocked on the owner.
- Done: docs/design foundations, DSN-008 glossary and viewport matrix, DSN-009 flows and state catalog, DSN-010 tokens and component inventory.
- In progress: static HTML prototypes, one new file per task; 1/5 done (DSN-011 fast product entry FR); remaining 2/5 Caissier credit HUD, 3/5 Nouveau client quick form, 4/5 manager-approval dialog, 5/5 debt-collection form and proof; Arabic/RTL pass of each planned later for owner glossary review, needs a browser-opening warning for screenshots.
- Not started, BRK family (indicative order): manager approval in place (ProcessReturn first, then credit_record_payment; audit with both user ids; PIN set/reset in Utilisateurs; PIN attempt counter and lockout; re-check failed logins/PIN checks write no audit rows); permission split (new statement-read permission, RecordCreditPayment withdrawn from Caissier, role-matrix tests changed in the same task, possible status-only credit command); tighten catalog_define_kit; SessionInfo permissions; Users engine task; categories; FTS search; drafts list/resume; stock overview; customer fiscal ids; overdue list; audit rows for credit payments; printable debt-payment proof with its print job in the same transaction; A4 renderer; 58mm thermal path; amount-in-words if required; import/export; tax.regime key; preference persistence.
- Later: code-splitting, bundle budget, CSP; logo copy; shop-PC host profile; demo dataset; FTS5 migration; barcode labels.
- Final: installer rebuild, BRK-057 finish, tag v1.0.0 only when charter 9.2 holds.
- The IA fixes (adjacent v0.3/v0.4 header lines; row J03 valid for managers only) are folded into the first future task that edits docs/design/01-information-architecture.md again; no separate task is opened solely for this.

## 12. Errors and dead ends
- AUD-011 miscounts corrected; documents_* group is 5, total 72 is right, physical utilities 189 was substring inflation, fixed hits are 15 + 1.
- AUD-012 summary sentence wrongly said Vendeur and Caissier are denied reprint/return; the committed tables are right: the Caissier may reprint but not return.
- REL-007 once gave a 63-char sha; PowerShell 5.1 has BOM bugs.
- DSN-005 skipped a v0.4 history entry, fixed in DSN-006; stray typo "dpast".
- DSN-006 residual defects not fixed: adjacent v0.3/v0.4 header lines with no blank line; row J03 valid for managers only, Caissier entry point is open; folded into the next IA-editing task.
- AUD-014 report weaknesses accepted: three summary rows used REFUTED for negative statements; S7.5 rested on one file only, must be re-checked in the manager-approval BRK task; U-S2.3 stays UNKNOWN.
- DSN-007 report weaknesses fixed by DSN-007-C1: unnumbered examples, unconfirmed claim, wrong PASTED-LINES counts, summarized code excerpt, printed twice.
- TASK-DSN-010-C1 false alarm: the architect's own manual recount of a pasted table wrongly flagged a missing z-overlay-scrim row; the row was present on disk at line 97; no write, no commit; lesson: prefer fresh from-disk recomputation over either side's manual tally.
- TASK-REL-013 first draft (different regex, placeholder CHARTER-15-2, boot path at repo root, wc -l 138) was superseded before execution and must not be matched against any report.
- TASK-REL-013 second draft assumed charter 15.2 was a markdown heading; the executor's grep proved it is plain numbered text under the "## 15." heading, not its own heading; corrected in REL-013-C1.
- TASK-REL-013-C1 through C3 blocked at CTP-CHECK due to authoring discrepancies in check expectations versus mandated text; fixed in REL-013-C4.

## 13. Key engine and domain facts
- DocumentType has 8 variants: Ticket, Facture, Bl, Devis, Avoir, Bc, Br, Ff; codes TICKET FACTURE BL DEVIS AVOIR BC BR FF; prefixes TICK FAC BL DEV AVO BC BR FF.
- customers table (migration 0001, no ALTERs through 0013): id, code UNIQUE, name, phone, address, credit_policy, credit_limit_centimes, current_balance_centimes, is_active, created_at, updated_at, revision, origin_device_id.
- Role::permissions() totals: Vendeur 5, Caissier 13, Gerant 29, Admin 34.
- credit_record_payment needs RecordCreditPayment (+ManageCustomerCredit on overpayment); writes ledger, balance, cash_movement in one transaction; no audit row, no document, no reversal.
- customers_get/customers_create return balance to every role today; a BRK task must route it, not rely on UI hiding.
- customers_create validation: name 2-100 chars, phone formats, CUSTOMER_PHONE_DUPLICATE.
- Credit sale rules: CREDIT_CUSTOMER_REQUIRED, Limited/Disabled policy refusals, CreditLimitOverride.
- AUD-014 summary: 8 DocumentType variants, single 80mm thermal pipeline, no A4/58mm path, print_jobs.document_id NOT NULL needs a migration for proof-without-document, document_sequences allocator cap 999999 PREFIX-YEAR-06d, no amount-to-words function, users.pin_hash nullable with no counter/lockout/production PIN-set path, item designation not snapshotted.
- Key handlers present: F1,F2,F3,F4,F5,F8,F10,F12,Delete,Backspace,arrows,Ctrl/Cmd+K; not implemented: F6,F7,F9,F11,Alt+1..8,Ctrl+digit.
- 18 modals enumerated; the 19th is unidentified, hypothesis is component C17 Modal shell.
- Timbre: applies to Ticket and Facture only, cash tendered above 0; base is the whole document total before timbre; amount is clamped 500..250000 centimes; compute_timbre at pricing.rs:156-169, called only from price_document; no call in credit-payment paths; returns.rs:388 hardcodes 0; no test covers mixed cash+credit tender.

## 14. Open design points
- Rail default at 1024px: T1 proposes collapsed, unconfirmed.
- Ctrl+digit conflicts in the packaged webview are unmeasured.
- Code-splitting bundle budget is not set.
- Phone mandatory in the quick "Nouveau client" form is undecided.
- Design-token colours and fonts are unreviewed by the owner.
- Dark theme has no accessibility check.
- A third density for very tall screens is undecided.
- Mapping of the 18 existing modals to the component inventory is a later audit task.

## 15. Unknowns
- The 19th modal.
- Layout of A4 and other documents; none exist yet.
- Real scanner wedge behaviour.
- Shop PC specification.
- Small-screen behaviour beyond the nominal viewport matrix.
- The Ctrl+K command-palette action content.
- Customer data inside audit_log rows (U-S2.3).

## 16. Instructions for SESSION-ARC-005 first replies
- Reply numbering restarts at 1.
- Do not send the questionnaire to the accountant before the PDF review.
- Resume the PDF review as the first priority as soon as the owner delivers QUESTIONNAIRE-COMPTABLE-FR-v2.pdf.
- Continue the one-task-per-reply discipline and the family/continuation naming.
- Continue the static prototype series (2/5 through 5/5) before starting the BRK family.
- Never claim "100%" or "production ready" while a gate or measurement is open.
- Prefer a fresh from-disk recomputation from the executor over any manual tally when reviewing a report.
