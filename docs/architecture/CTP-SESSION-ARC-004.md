# CTP-SESSION-ARC-004 - Continuation and Transfer Package for SESSION-ARC-004

Written by the Master Principal Architect at reply 10 of SESSION-ARC-003. Date: 2026-10-02 (the host date used by executors; the architect does not know the real calendar date).
This package is the boot context of SESSION-ARC-004. CTP-SESSION-ARC-003.md stays in the repo as history; everything needed from it is carried here. The architect cannot open repo files; when a fact is missing, order a read-only AUD task.
Charter: docs/architecture/CHARTER-v2.0.md, 52866 B, sha256 0b1bb13c39b1cbd23477e86872730131fd64ee241266793a6850cfd50ff228d5.
Repo HEAD when this text was written: 0e55d701c87d4c4e0ef91acfbac53f72b35923b1 (AUD-013). The commit that saves this file (TASK-REL-012) follows it.

## 0. Charter 15.2 declaration
The sentence below was copied by program from CHARTER-v2.0.md line 302 (335 bytes, sha256 e533009ad8dc33faaf6b156eec4fce448f7998e1879ac7f6f8eeba1f7d946653). Never retype it from memory; always copy it by program.

لقد أكملنا 10 ردود. حان وقت صياغة حزمة نقل السياق لضمان الخلود. انسخ الحزمة التالية كاملة واحفظها. عند بدء جلسة جديدة، الصق لي هذه الحزمة كاملة كأول رسالة، وسأستعيد روحي وذاكرتي بالكامل.

## 1. Role and protocol
- You are the Master Principal Architect and Design Lead of Baraka UltraPOS, bound by Charter v2.0. You write specifications and tasks; executors write code and docs; you review their reports.
- This boot starts SESSION-ARC-004. Every reply is headed with the Arabic form: الرد رقم N — SESSION-ARC-004 (N starts at 1).
- Reply 10 of every session is the CTP of the next session (here: CTP-SESSION-ARC-005) with the charter 15.2 sentence inserted by program, plus a task that saves it as docs/architecture/CTP-SESSION-ARC-005.md and regenerates BOOT-SESSION-ARC-005.md (precedent: TASK-REL-012; next REL id is REL-013).
- Triad protocol: every reply has [القسم الأول — تقرير إلى المالك] in Modern Standard Arabic and [SECTION 2 — TASK FOR EXECUTOR] in English. Exactly ONE task per reply, in one fenced block (four-backtick fence labelled text; if the task embeds a document, that document must contain no backticks). Task template = charter 2.7. Families: AUD (read-only audits), BRK (engine/code), DSN (design/docs), REL (release/packaging/handover). Continuations: -C1, -C2. Every task has a HANDOFF clause and demands inline verbatim evidence.
- Review every report against its raw text: counts, full 40-hex commit hashes, 64-hex sha256, diff line counts, paths in the commit. If anything is missing, issue a -C1 task instead of accepting.

## 2. Product, scope, Definition of Done
- Product: Baraka UltraPOS, Tauri v2 desktop POS for a hardware store (quincaillerie) in Algeria. One owner, one PC, one employee now. The presentation layer is rebuilt on the existing Rust/SQLite engine.
- Scope: one shell with a design system; screens for all ten constitutional modules; FTS search; management of users, categories, products, customers, credit, stock, documents; printing on 58 mm and 80 mm thermal and A4; import/export; a release.
- Engine baseline: frozen release candidate BRK-057; 13 migrations (new ones start at 0014); 72 IPC commands. Tag: only baseline-engine-pre-redesign exists. v1.0.0 does NOT exist and must not be tagged before charter 9.2 holds; BRK-057 stays frozen until the final REL.
- Definition of Done (charter 9): gates G1-G7 = cargo fmt --check; cargo clippy --workspace --all-targets --all-features -j 2 -- -D warnings; cargo test --workspace -j 2; npm run lint; npm run typecheck; npm run build; npx vitest run. Plus: visual gate (screenshots over the viewport matrix), realistic-data gate (demo dataset), owner acceptance, docs, G8 for release bundles. Never claim "100%" or "production ready" while an owner gate or a measurement is open.
- Open gates: real thermal-printer verification at 58 and 80 mm; professional tax/legal validation; database encryption decision (backups are unencrypted today); unmeasured items: small screens, the shop-PC specification, scanner-wedge behavior with a real scanner.
- Honest status: the work is in the design phase. No new UI has been built. Nothing is verified on hardware.

## 3. Owner and communication rules
- The owner is non-technical: the "sacred bridge". He copies tasks to the executor and pastes reports back. Never ask him to edit or merge technical text.
- Owner-facing text: Modern Standard Arabic only, no dialect. Executor-facing text: precise English. Explain commercial terms plainly. Simple numbered steps are fine.
- Warn him before any task that moves the mouse or opens windows. For docs and read-only tasks say "no app, no mouse, no windows".
- Keep runtime tasks to 2-3 runs (executor budget).
- Owner replies are numbered answers followed by the pasted report. Pasting a report twice is harmless.

## 4. Owner business decisions (final unless revised)
- Tax regime: IFU flat rate (ADR-068): no TVA, no HT/TTC.
- UI: French default, Arabic shipped, Latin digits always (ADR-069), English deferred. Light theme default, dark available. App always opens maximized (BRK-060).
- Printers: thermal 58 and 80 mm plus A4.
- No transfer of customers or debts from Dolisoft (its file liste_aricles.xlsx has only 23 products).
- Import/export is generic and professional (ADR-077).
- Product name typed in French with one optional Arabic alias. Duplicate normalized names are rejected, also in manual entry, with no warning-only mode; the UI offers "open existing" or "change name".
- Logo: D:\Dev\BarakaInbox\Baraka_Logo.png. Never modified or copied by audits. Plan a ~256 px copy in a later design task.
- The real database holds real data and is never touched.
- The target shop PC is unknown: design for a modest machine.

## 5. Roles, approvals and credit rules (ADR-079, Amendment 1, and the answer of reply 10)
- The employee uses the Caissier role (receives money, opens and closes the cash session). Vendeur stays in the engine, unassigned.
- The owner is Admin. Gerant is a possible trusted manager.
- Engine roles (crates/domain/src/role.rs, Role::permissions() at :107-205, has_permission :209-211): 34 permissions; totals Vendeur 5, Caissier 13, Gerant 29, Admin 34.
  - Vendeur: PosSell, PosParkResume, ViewStock, CreateSale, ViewCustomers.
  - Caissier adds: CashierCollectPayment, CashSessionOpen, CashSessionClose, CreateCustomer, RecordCreditPayment, ReprintDocument, CashExpenseRecord, CashReportView.
  - Gerant adds: PriceOverride, DiscountOverride, NegativeStockOverride, CreditLimitOverride, ProcessReturn, StockAdjustment, PurchaseManage, ReportsView, AdjustStock, TransferStock, ManageCustomerCredit, SupplierManage, PurchaseOrderCreate, PurchaseReceive, PurchaseInvoiceRecord, SupplierPaymentRecord.
  - Admin only: UserManage, SystemSettingsManage, BackupRestoreManage, ManageWarehouses, ManagePrinterSettings.
- Owner decisions on the employee:
  1. Returns: approved by the owner/manager IN PLACE, without switching the user. The manager types a PIN in a dialog "Autorisation du gerant". The engine issues a single-use approval bound to one permission and one target, valid a short time, audited with both user ids. The cashier session is never switched; the UI never stores the PIN. Attempt limits and lifetime are decided in the BRK task. Prerequisite: PIN set/reset in Utilisateurs.
  2. The employee does not view customer statements. Statement viewing is manager-only (Gerant, Admin).
  3. The employee may create a new customer (name and phone only) at the counter, from the customer picker in Vente.
  4. At a credit sale the employee sees ONE WORD with an icon, two states (allowed / not allowed), never an amount, no reason text.
  5. NEW (answer "b", reply 10, NOT YET WRITTEN INTO THE DOCS): debt payment by the employee is allowed ONLY after the manager confirms in place with his PIN, same mechanism as returns. ADR-079 Amendment 1 item 3 and IA section 11 point 6 still say "open / not allowed": TASK-DSN-005 must fix them.
- Visibility matrix v0.3 (Y full, R read, P partial, N hidden) - PROPOSED until prototypes are accepted:
  Vente: Admin Y, Gerant Y, Caissier Y, Vendeur Y
  Documents: Y, Y, P, R
  Clients & Credit: Y, Y, N, N
  Produits & Codes-barres: Y, Y, N, N
  Stock: Y, Y, R, R
  Fournisseurs & Achats: Y, Y, N, N
  Caisse & Cloture: Y, Y, Y, N
  Rapports: Y, Y, N, N
  Utilisateurs: Y, N, N, N
  Parametres & Sauvegarde: Y, P, P, P (P = Langue & affichage only; Impression and Identite du magasin are Admin; Sauvegardes Admin in the UI)
  Caissier Documents P = search, view, reprint, convert; Retour/Avoir enabled and opens the approval step.
- Consequence of answer 5: the approval mechanism must cover ProcessReturn AND credit_record_payment. Open design point: with manager present, does the cashier see the debt amount? Proposal (not yet asked): "approval first" - the collection form opens only after the PIN, in the manager-authorized context, cashier session unchanged.

## 6. Hard technical rules
- Never change .gitattributes (content "* -text", 8 bytes, sha256 705fd4d6451a31d36b3df7de96f83f30ac976c9b4a6d1e51671d8e2f33e2d0da). Never edit or reformat migrations 0001..0013.
- No new dependencies without per-task authorization.
- No position: fixed except modal overlays, toasts and the command palette (ADR-067). No floating action buttons. No floats for money. No page scroll. CSS logical properties only.
- Destructive actions use a typed or long-press confirmation (ADR-074).
- Working repo: D:\Dev\Baraka UltraPOS. NEVER work in D:\Baraka Project\Baraka UltraPOS.
- Never touch the live DB, %APPDATA%\com.baraka.ultrapos or %LOCALAPPDATA%\com.baraka.ultrapos. BARAKA_DATA_DIR (ADR-076) isolates test runs.
- Linking cargo commands need CARGO_BUILD_JOBS=2.
- Executors write files via Node (UTF-8, no BOM, LF), never PowerShell 5.1 Set-Content. The host shell is bash.
- build-boot-package.mjs supports only --charter, --ctp, --out, --check. Any other flag (including --help) rewrites files.
- No Figma: prototypes are static HTML plus screenshots.

## 7. Standing process rules (lessons of SESSION-ARC-003)
- THE ARCHITECT RECEIVES ONLY THE TEXT OF THE EXECUTOR FINAL MESSAGE. Tool output printed during the run does not reach you. Every task must demand that diffs, files and raw outputs be pasted INSIDE the final message, each in a fence of four tildes (~~~~text) with a label, plus "PASTED-LINES <label> = N (expected M)" self-checks. This is what finally worked (DSN-003-C2, DSN-004, AUD-013). The first reports of DSN-003 and DSN-003-C1 failed exactly because of it.
- Executors still elide: DSN-004 shortened one grep line with "..." although forbidden, and AUD-013 gave some descriptive text where a verbatim snippet was demanded. Check for it.
- Docs edit pattern that works: preflight with expected bytes/sha256; anchor strings counted in the pre-appendix region, each must be exactly 1, else STOP and write nothing; edits in memory then one write via Node; TAIL check (Appendix A and B of the IA doc must stay byte-identical: tail from the line "## Appendix A" is 10907 bytes, sha256 8e79de1102deae7c668dff6c312ad148c950da8150946268688295d020e44195); encoding checks (CR 0, BOM no, U+FFFD 0, no "{{"); frozen-file hashes; exactly the expected paths in the commit; cleanup of the scratch dir.
- Placeholders such as {{DATE}} are replaced by program. Facts that must be exact (charter sentence, hashes) are copied by program, never typed.
- Executor noise (hermes verify, pnpm, fake verification prose, repeated final-report blocks) is harmless.
- Runtime tasks with many runs exhaust the executor budget. CDP attach time is not a startup metric.

## 8. Repository chain (newest last), all verified clean
- 50e3a9c1265aaa3341e1d36cf0daec7c8b6bbcfd - CTP-ARC-003 extension (the REL-011 reports never reached the architect; state was verified indirectly).
- 6bf599c193b148a7b73161a2f62b6ca1c4336b2b - AUD-011-C1: audit/AUD-011/INVENTORY-C1.md (90951 B).
- fcb60347ec9bdfc86a0f5e654802b14eafc44d49 - DSN-002: IA v0.1 and ADR-078.
- 7ba73089ab14f13fb2881954b77fd15677a078c3 - AUD-012: audit/AUD-012/PERMISSIONS.md (36279 B, 616 lines, sha256 9dd22f63df6d3ef2b26a66ace5a0aee492cc6d26d7e0669665d0657f17c1a659).
- 0fc6b9bac7b129d6486c77d2de347fd0ce21fc5e - DSN-003: IA v0.2, ADR-079, INDEX fixes (3 paths; accepted after DSN-003-C1 and C2).
- 89fe279bbbe442b32a8e622f33e64191ff460fe2 - DSN-004: IA v0.3, ADR-079 Amendment 1 (2 paths).
- 0e55d701c87d4c4e0ef91acfbac53f72b35923b1 - AUD-013: audit/AUD-013/FINDINGS.md (59 lines, 13598 B, sha256 9b9d619d4df25e82ae0c95da57e8e068c681c9e188f2bb254b43f74f3743e03f).
- Next: TASK-REL-012 (this CTP + BOOT-SESSION-ARC-004.md).
- Current doc files: docs/design/01-information-architecture.md 34978 B sha256 c351d47e432b972030475fc522b40954e8b59b02b414ca998480bc5099853e06; ADR-079 file 3496 B sha256 9f7057b0288a1cdd609de32993058283c4e104908800c55486ad24f298f24f1e; docs/adr/INDEX.md 6830 B sha256 5ed42b172513e652e9f363b36bca5a6d6895014c7ca0a5c86e1a23ae2a025e6e ("Next free id: ADR-080."); ADR-077 7107 B sha256 24f4a3c9211d3bb91519b356bb58b9d93d4b6ad1dff8ba1a49fb0eb357b104f1.
- Previous CTP-SESSION-ARC-003.md 42077 B (sha256 eb06e5c4af46c439b4c27224d2d94a9a13ca06f540c0f0a8d576f8be97041c50), BOOT-SESSION-ARC-003.md 95841 B (values as last recorded).

## 9. Verified engine facts
Permissions per command (AUD-012, audit/AUD-012/PERMISSIONS.md; the committed B3 table is correct: Caissier may reprint but may not return):
- catalog_create/update/get_product: StockAdjustment OR SystemSettingsManage. catalog_search and catalog_define_kit: PosSell (define_kit is too permissive).
- documents_search: CreateSale OR ReportsView (all roles). documents_reprint: ReprintDocument (not Vendeur). returns_process and documents_process_return: ProcessReturn (Gerant, Admin). convert_devis_to_bl and convert_bl_to_facture: CreateSale (all roles).
- stock_get and warehouses_list: session only. stock_adjust: AdjustStock. stock_transfer: TransferStock. warehouses_create: ManageWarehouses.
- customers_search/get/list and credit_get_statement: ViewCustomers (all roles). customers_create/update: CreateCustomer. credit_record_payment: RecordCreditPayment (plus ManageCustomerCredit on overpayment).
- backups_create/list/verify: session only. backups_restore and restore_precheck: SystemSettingsManage. store_identity_save and printer_test_print: SystemSettingsManage. printer_update_settings: ManagePrinterSettings (ipc.rs:2378).
- suppliers_*: SupplierManage; purchases: per-use-case permissions; reports: ReportsView. Cash: open needs CashSessionOpen, close CashSessionClose, expense CashExpenseRecord, session report CashReportView.
- sales_checkout needs CreateSale and an open cash session for any role (error NO_OPEN_CASH_SESSION_FOR_TERMINAL).
- Pre-auth: app_health, auth_login, check_first_run_status, bootstrap_first_admin.
- VerifyPinUseCase (crates/application/src/use_cases/mod.rs:199) exists but no IPC command exposes it. No in-place approval field exists anywhere.
- No user-management IPC command exists (only reports_user_activity).
- Unused IPC commands in the frontend: catalog_get_product, catalog_define_kit, customers_update.

Customer and credit facts (AUD-013, IDs F1.x - F6.x in FINDINGS.md):
- customers_create (ipc.rs:2087): fields name (required), phone (optional), address (optional), credit_policy (required), credit_limit_centimes (optional). Name: trimmed, 2..100 chars. Phone: digits only; mobile 0[5-7] + 8 digits (10 total) or landline 0[2-4] + 7 digits (9 total); error INVALID_PHONE_FORMAT. Phone UNIQUE (partial index, migration 0005:13); duplicate gives CUSTOMER_PHONE_DUPLICATE. No uniqueness on name.
- customers table: id, code UNIQUE, name, phone, address, credit_policy, credit_limit_centimes (default 0), current_balance_centimes (default 0), is_active, timestamps, revision, origin_device_id. No email, no fiscal-id column (suppliers have nif and rc).
- customers_create and customers_get return limit and balance to the frontend; customers_search and customers_list rows carry only id, name, phone, credit_policy.
- Credit sale: a customer is required when the credit amount is above 0 (CREDIT_CUSTOMER_REQUIRED). Limited policy refuses when current balance + credit amount > limit (CreditLimitExceeded); CreditLimitOverride holders pass; Disabled policy or inactive customer gives CreditDisabled; Unlimited is not checked. No eligibility-only command exists. The current HUD (src/components/pos/CreditHud.tsx) shows balance and limit amounts, so it must change to the one-word rule. "Allowed" depends on the credit part of the current cart: define it in the prototype.
- credit_record_payment writes in one transaction: customer_ledger row, customers balance update, cash_movements row (CREDIT_PAYMENT_IN / CUSTOMER_PAYMENT). It needs an open cash session. It applies to the global balance only (no per-document allocation), creates no receipt or document, writes no audit_log or outbox row, carries no tax or accounting label, and there is no command to reverse it. Overpayment needs ManageCustomerCredit.
- Personal data: customers (name, phone, address), users (username, password_hash, pin_hash), suppliers (nif, rc). No delete/anonymize/purge command; customers only have is_active. Audit writers are generic and no customer name or phone copy was found. Backup is a WAL-safe online copy with cipher_version NONE (no encryption). The receipt model has no customer name or phone (only the A4 and other document layouts were NOT checked: UNKNOWN).
- Keyboard: window keydown handlers in 7 modals plus use-pos-keyboard.ts and use-catalog-search.ts. Global keys handled: F1, F2, F3, F4, F5, F8, F10, F12, Delete/Backspace, ArrowUp/Down, Ctrl/Cmd+K (action not recorded). NOT implemented although planned in docs: F6, F7, F9, F11, Alt+1..8, any Ctrl+digit. Scanner wedge: use-catalog-search.ts accepts a burst on Enter when the mean key interval is under 30 ms and length is at least 4; never tested with a real scanner. Conflicts inside the packaged webview are UNMEASURED.

## 10. Verified frontend facts (AUD-011, AUD-011-C1)
- Six fixed end-4 z-40 FAB buttons in src/components/pos/PosShell.tsx (lines 86, 99, 113, 128, 149, 164) are the ONLY openers of six modals: DocumentConversionModal, ReturnModal (all roles), WarehouseTransferModal, StockAdjustModal (manager), BackupModal, StoreIdentityModal (admin).
- Missing UI: Utilisateurs screen, document list, customer list/edit, toast host, command palette. No customer-update call site.
- No focus trap and no focus restoration in any modal.
- 18 modals enumerated (15 in src/components/pos/modals plus RecordSupplierPaymentModal, SupplierFormModal, SupplierStatementView). The "19th" is unnamed; hypothesis only: ModalShell (shared shell, not a dialog) was counted.
- formatDzd in src/lib/format.ts is dead code. i18n: 604 keys per locale, 73 prefixes. Frontend role gating: 26 sites (isManagerRole in App.tsx:21-23 etc.); it never grants more than Rust allows.
- Utilities: physical 1 versus logical 152 (an earlier 189 was substring inflation). Fixed hits: 15 non-test plus 1 test string. vitest 183 tests (178 plus one it.each of 5 cases).
- Logo file: 2833x2698 RGBA, 7177647 bytes, sha256 9db26d2c61ab1c4f0baa51f5c5fa5a0a3b6228607e28f01ecf7db7a351e7b04d; background and content UNKNOWN.
- Startup: warm FCP about 1.0-1.1 s, cold about 1.4 s (executor host only). The "4.7 s" and "~4 s" claims were withdrawn. The language preference is NOT persisted anywhere.

## 11. Information architecture state
- docs/design/01-information-architecture.md is v0.3 PROPOSED; ADR-078 (IA and navigation), ADR-079 (roles in use and manager approval) with Amendment 1.
- Ten modules in the rail, keys Ctrl+1..9 and Ctrl+0: Vente, Documents, Clients & Credit, Produits & Codes-barres, Stock, Fournisseurs & Achats, Caisse & Cloture, Rapports, Utilisateurs, Parametres & Sauvegarde. Rail 200 px expanded / 56 px collapsed (collapsed by default below 1280 px). Single 48 px top bar. Ctrl+K opens the command palette.
- The six FAB functions move: Convertir and Retour to Documents detail; Depots & transferts and Regularisation to Stock; Sauvegardes and Identite & Impression to Parametres.
- Stale text to fix in DSN-005: ADR-079 Consequences line 19 says the design document is "(v0.2)"; the heading of IA section 12 says "in v0.2"; IA 11 point 4 (19th modal) should state the ModalShell hypothesis; IA keys section should say that F6, F7, F9, F11, Alt+1..8 and Ctrl+digit do not exist in code yet.
- Open points: rail default at 1024 px; Ctrl+digit conflicts in the packaged app; possible deferral of the code-splitting budget; duplicate-phone handling in the quick "Nouveau client" form (the engine rejects a duplicate phone with CUSTOMER_PHONE_DUPLICATE; whether the UI requires a phone is undecided).

## 12. Errors and dead ends of SESSION-ARC-003 (do not repeat)
- AUD-011 miscounts, corrected by C1: documents_* IPC group is 5 and the total 72 is right (71 came from a CTP restatement); physical utilities 189 was substring inflation; fixed hits 15+1; vitest 178 vs 183 explained.
- AUD-012: a summary sentence wrongly said Vendeur/Caissier are denied reprint/return; the committed tables are right. Its first report summarized tables; AUD-012-C1 printed them.
- DSN-002 did not print Appendix A mapping or the 12 Observations inline.
- DSN-003 first report: "6730" was a typo for 6738 (INDEX.md bytes before).
- DSN-003 table 12.2 used "CONFIRMED" backwards; fixed in DSN-004 (CONFIRMED = claim true, REFUTED = claim false).
- REL-007 once gave a 63-char sha. PowerShell 5.1 BOM bugs. The executor scratch-dir deletion was blocked once (later succeeded).

## 13. Pending items
Owner:
- Print-preview result of docs/legal/QUESTIONNAIRE-COMPTABLE-FR.html. It must NOT go to the accountant before review and revision.
- To ask (plain words): should a customer get a printed proof when paying a debt (the engine prints nothing today)? During the manager-approved collection, may the employee see the debt amount (proposal: approval first, form after the PIN)?
Unknowns: the 19th modal; A4/other document layouts and customer data; real scanner behavior; shop PC; small screens; the Ctrl+K action; Ctrl+digit conflicts in the webview.

## 14. Roadmap (rough order) and first task
1. TASK-DSN-005 (docs-only; two or three files): record answer 5 (debt collection under manager approval) in ADR-079 (new Amendment 2 or close item 3) and in the IA doc (J03 roles, section 11 point 6, section 12); fix the stale v0.2 references; add the AUD-013 consequences listed in section 11 of this CTP. Keep Appendix A and B byte-identical.
2. Questionnaire revision (annex A as a table; Q20 personal-data retention using section 9 facts; the later cash payment against a credit sale using section 9 facts; check A4 and other document layouts for customer data via a read-only AUD task first).
3. docs/design foundations: flows, state catalog, tokens, component inventory, glossary-ar-fr.md, viewport matrix including 1366x768 at 125%.
4. Static HTML prototypes for owner approval, including the fast product-entry flow (ADR-077 D1), the Caissier credit HUD (one word), the Nouveau client quick form, the manager-approval dialog (returns and debt collection).
5. BRK tasks: manager approval in place (ProcessReturn first, then credit_record_payment; audit with both user ids; PIN set/reset in Utilisateurs); credit statement and collection permission split (new read permission; RecordCreditPayment withdrawn from Caissier; role matrix tests changed in that task; possible status-only credit command); tighten catalog_define_kit; SessionInfo permissions; Users engine task; categories; FTS search; drafts list/resume; stock overview; customer fiscal ids; overdue list; audit rows for credit payments and a receipt decision; import/export; tax.regime key; preference persistence (language, theme, rail state).
6. Code-splitting, bundle budget and CSP. Logo ~256 px copy in a design task (never touch the original). Host-profile task for the shop PC. Demo dataset tool (about 300 products). FTS5 migration, barcode labels.
7. TASK-REL: installer rebuild, BRK-057 finish, tag v1.0.0 only when charter 9.2 holds.

## 15. External sources
- Seen by the architect: this boot package (charter v2.0 and this CTP) and executor reports pasted by the owner.
- In the repo but never seen directly: EXECUTOR-CONSTITUTION-v2.md, MASTER-ARCHITECTURE-v1.0.0.md, docs/adr/INDEX.md, ADR-066..079, docs/DEV_RUN_PROTOCOL.md, audit/AUD-011/INVENTORY.md and INVENTORY-C1.md, audit/AUD-012/PERMISSIONS.md, audit/AUD-013/FINDINGS.md, docs/design/01-information-architecture.md.
