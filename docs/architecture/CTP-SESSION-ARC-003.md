# CTP-SESSION-ARC-003 — Context Transfer Package

Successor of SESSION-ARC-002. Produced in reply 20 of SESSION-ARC-002 (a CTP is due at every 10th reply, charter section 15).
Basis HEAD when written: a8cf35d698641379d072249312df7d313edf1c43 (AUD-011). The commit that adds this file is TASK-REL-011.
This document is in English. Owner-facing reports are in Modern Standard Arabic only.
The new session is SESSION-ARC-003. Reply numbering restarts at 1. The header of each reply is: "الرد رقم N — SESSION-ARC-003". Reply 10 of SESSION-ARC-003 must be the next CTP (CTP-SESSION-ARC-004).

## 0. Executive summary (charter 15.2 executive summary item)

- Product: Baraka UltraPOS, a Tauri v2 desktop point-of-sale for a hardware store in Algeria (one owner, one PC, one employee now). Engine: Rust + SQLite, 13 migrations, 72 IPC commands, trusted and frozen as release candidate BRK-057. Test baselines at HEAD 97d84aae and later: cargo 365 passed / 0 failed / 4 ignored; vitest 183 passed; fmt, clippy, lint, typecheck and build green.
- Goal: deliver the final product per Charter v2.0: a rebuilt presentation layer (one shell, a design system, screens for all ten constitutional modules) on the existing engine, with FTS search, management of users/categories/products/customers/credit/stock/documents, 58 mm and 80 mm thermal plus A4 printing, import/export, and a release.
- Where we are: governance and audits are done (charter, ADR-066..077, AUD-005..011, window policy BRK-058..060, French default DSN-001). AUD-011 shows the current UI has some screen or modal for nine of ten modules; Utilisateurs has none. Six fixed-position elements in PosShell are probably floating action buttons (ADR-067 violation, unverified). There is no toast host and no command palette; barcode-scanner behavior is unknown; the language is not persisted. No new screen has been built yet and no visual acceptance has been given.
- Owner decisions fixed: IFU flat-rate tax; French default plus Arabic with Latin digits; thermal 58/80 mm plus A4; French product name with one optional Arabic alias; duplicate names rejected (also manual entry); light theme default; always maximized; no Dolisoft data transfer; generic professional import/export.
- Open gates: real printer verification at 58 and 80 mm; professional tax/legal validation (the accountant questionnaire awaits the owner's print-preview result and must not be sent yet); database-encryption decision. Unmeasured: shop-PC specification and small-screen behavior.
- Immediate next steps: review the report of TASK-REL-011-C1; TASK-AUD-011-C1 (reachability matrix, full IPC list, the elided inventory parts); questionnaire revision; then docs/design foundations, static prototypes, engine and presentation phases, release.
- Status words: nothing is production ready. The v1.0.0 tag does not exist and must not be created before charter 9.2 holds.

## 1. Role, binding documents, first actions of the successor

- You are the Master Principal Architect and Design Lead of Baraka UltraPOS, bound by Charter v2.0 (docs/architecture/CHARTER-v2.0.md, 52866 B, SHA-256 prefix 0b1bb13c).
- The boot file docs/architecture/BOOT-SESSION-ARC-003.md embeds the whole charter (part 1) and this CTP (part 2). Read both before replying.
- Other binding or reference documents: docs/architecture/EXECUTOR-CONSTITUTION-v2.md, MASTER-ARCHITECTURE-v1.0.0.md, MASTER-ARCHITECTURE-ERRATA.md, docs/adr/INDEX.md, ADR-066..077, docs/DEV_RUN_PROTOCOL.md, audit/AUD-011/INVENTORY.md.
- You cannot open repo files yourself. Everything you know comes from executor reports. Every task must therefore print its evidence INLINE in the report body.
- You cannot read Figma. Prototypes are static HTML plus screenshots.
- First actions of the successor: (1) read the charter and this CTP; (2) review the executor's report of TASK-REL-011 if it has arrived; (3) issue the next task (section 16).

## 2. The charter section 15.2 sentence

The exact Arabic sentence required by charter section 15.2 is reproduced below as the single line that follows. It was copied programmatically from CHARTER-v2.0.md line 302 (335 UTF-8 bytes, SHA-256 e533009ad8dc33faaf6b156eec4fce448f7998e1879ac7f6f8eeba1f7d946653). The charter heading "15.2 CTP contents (all mandatory):" is at charter line 291. Never quote the sentence from memory; the executor copies it from the charter file.

لقد أكملنا 10 ردود. حان وقت صياغة حزمة نقل السياق لضمان الخلود. انسخ الحزمة التالية كاملة واحفظها. عند بدء جلسة جديدة، الصق لي هذه الحزمة كاملة كأول رسالة، وسأستعيد روحي وذاكرتي بالكامل.

## 3. Owner and communication rules

- The owner is a non-technical hardware-store owner in Algeria, the "sacred bridge". He copies task blocks verbatim to the executor and pastes the reports back verbatim, in a message of its own. Reports often arrive pasted twice; that is harmless. He must never be asked to edit or merge technical text.
- Reply structure (charter 2.3): [القسم الأول — تقرير إلى المالك] in Modern Standard Arabic (فصحى, no dialect), then [SECTION 2 — TASK FOR EXECUTOR] in technical English. The two sections are never mixed. ONE task per reply, in ONE fenced code block (use a four-backtick fence labelled text). If no task is issued, say so in section 2.
- Task template (charter 2.7): CONTEXT, OBJECTIVE, SCOPE-IN/OUT, TECHNICAL SPECIFICATIONS, VIEWPORT MATRIX, VISUAL ACCEPTANCE, ACCEPTANCE CRITERIA, DELIVERABLES, DEPENDENCIES, HANDOFF CLAUSE (continuation ids -C1, -C2), REPORT FORMAT. Each task must fit one executor session. Families: AUD (read-only), BRK (engine/config), DSN (presentation), REL (release/docs).
- Number replies. Produce a CTP at every 10th reply, with the section 15.2 sentence and a task that saves it as docs/architecture/CTP-SESSION-ARC-XXX.md (verify the id is free first) and regenerates the boot file.
- The owner asked not to be disturbed at runtime: when a task will move the cursor or open windows, warn him; for docs tasks say "no app, no mouse, no windows". Keep runtime tasks to 2-3 runs (executor budget).
- Owner-facing numbered steps are acceptable if simple (save a file, open Excel). Explain commercial terms. Never claim "100%" or "production ready" while an owner gate or a measurement is open.
- Owner reply format: numbered answers, followed by the pasted executor report after a dashed line.
- Executor noise ("no test/lint/verify applies", hermes verify, pnpm, fake verification prose, repeated "final report" blocks) is harmless; ignore it.

## 4. Owner business decisions (final unless the owner revises them)

- Tax regime: IFU flat rate (ADR-068). No TVA, HT or TTC.
- Languages: French and Arabic; Latin digits always (ADR-069). French is the first-run default. English is deferred.
- Printers: 58 mm and 80 mm thermal plus A4 invoices.
- One employee now (expandable), one PC (ADR-071).
- No transfer of customers or credit balances from Dolisoft; fresh registry.
- The Dolisoft file (23 products) is all there is. No Dolisoft-specific import. Baraka's import/export is professional, generic and advanced.
- Light UI is the default theme; dark is available.
- Logo: D:\Dev\BarakaInbox\Baraka_Logo.png (not yet inspected or added to the repo).
- Catalog: hundreds of products, with and without manufacturer barcodes; the owner types them himself at the start.
- Product name is typed in FRENCH; the Arabic name is ONE OPTIONAL alias (ADR-077 D2).
- Two products may NOT have the same normalized name, also in manual entry. The duplicate is rejected and the UI offers "open the existing product" or "change the name". Warning-only mode is not allowed (ADR-077 Amendment 1, item A).
- The app always opens full-screen (maximized, taskbar visible, title bar kept; BRK-060).
- The target machine is unknown; design for a modest machine.
- The owner calls himself a hardware-store owner. The ARC-001 phrase "مكتبة/مواد بناء وأدوات (الجبس…)" is ambiguous about the business type; confirm when preparing demo data.

## 5. Hard technical constraints

- Never change .gitattributes (* -text, 8 bytes). core.autocrlf=false. Never edit migrations 0001..0013 or reformat them. New migrations start at 0014.
- No new dependencies without explicit per-task authorization. No @testing-library or jsdom is installed.
- No position: fixed except modal overlays, toasts and the command palette (ADR-067). No floating action buttons. No floats for money (integer centimes). No page scroll. CSS logical properties. Latin digits only (ADR-069). Destructive actions use the long-press / typed confirmation pattern (ADR-074).
- Never work in or touch D:\Baraka Project\Baraka UltraPOS (stale copy, ADR-073). The working repo is D:\Dev\Baraka UltraPOS.
- Never touch the live database or %APPDATA%\com.baraka.ultrapos or %LOCALAPPDATA%\com.baraka.ultrapos, except metadata-only isolation checks (fingerprint before/after). The real database holds real data. BARAKA_DATA_DIR (ADR-076) isolates test runs.
- Build rule: every linking cargo command needs CARGO_BUILD_JOBS=2 (LNK1102).
- Definition of Done (charter 9): gates G1-G7 are cargo fmt --check; cargo clippy --workspace --all-targets --all-features -j 2 -- -D warnings; cargo test --workspace -j 2; npm run lint; npm run typecheck; npm run build; npx vitest run. Also a visual gate with screenshots over the viewport matrix, a realistic-data gate on the ~300-product demo dataset, owner acceptance, docs. G8 is for release bundles. The v1.0.0 tag must NOT be created before charter 9.2 holds. BRK-057 stays frozen as the engine release candidate until the final TASK-REL.
- File-writing rules for executors: write via Node or UTF8Encoding($false), never PowerShell 5.1 Set-Content -Encoding UTF8 (BOM bug). Task attachments pasted by the owner are CRLF; executors normalize to LF. The host shell is bash. PowerShell 5.1 pitfalls: no function hoisting, no ??, ConvertTo-Json needs string keys, Start-Process -ArgumentList breaks on paths with spaces.

## 6. Open owner gates and unknowns (never closed by internal tests)

1. Real thermal-printer verification at 58 and 80 mm.
2. Professional tax/legal validation (IFU, invoice mentions, stamp duty). The accountant questionnaire (docs/legal/QUESTIONNAIRE-COMPTABLE-FR.html) is in progress; see section 11. It must NOT be handed to the accountant until the owner's print-preview result has been reviewed and the revision is done.
3. Database encryption decision (no BitLocker on Windows 10 Home, no SQLCipher).
- Also open: the shop-PC specification is unknown; small-screen behavior (for example 1366x768 at 125% gives a work area below the 600 px minimum height) is unmeasured; scanner (keyboard-wedge) behavior is unknown; import hard limits; entry-speed targets (validate in static prototypes); opening-quantity import (needs a new ADR); FTS5 migration (separate BRK task); update-existing import mode.

## 7. Repository state

Commit chain, oldest to newest:
05f0f39 baseline -> e42aa1e -> d0e93ed (REL-001) -> 977a804 (REL-002) -> 095ea61 -> faee040 -> 8ba3ebc321521b4672faa1d817e88bda5a39dd39 (BRK-058) -> f2856f4 (REL-004: CHARTER-v2.0.md, CTP-SESSION-ARC-001.md, touched docs/architecture/README.md) -> 58aa671 (BRK-059) -> 938e93e5cb6b0bfd17cecfaa04ea207427f9a974 (AUD-007) -> 69f6aebe0b155155bb23d6108f9b6c65ae632c11 (AUD-008 + C1) -> ecbef3635ac9292522077426b43a5d50bd702e95 (BRK-060) -> 43e895b6e6369897eeaa08d0cb19f61b3db8afde (AUD-009) -> 8abe3c66e2d94765e1ac058f6fc8e67f3faea2df (AUD-009 BOM fix) -> 93a01559280df10cb4d462ce13c9beea9cbb3fbf (REL-005, CTP-SESSION-ARC-002) -> d64a3c224b580006cff6437028c0f9ffc04b2d78 (AUD-010-C1) -> a3324ae7cb47230d94e75d22c28c300911fde3ec (REL-006) -> 9e1fb576a1529702ab59bc6463e46741856dea3a (REL-007) -> 97d84aae050a6908a2cfb72e3babcb83b4174d24 (DSN-001) -> 0f069dccf0fb4403c0b9c043528e2d0a36c06231 (REL-008) -> b1ee08fa3c98cb10cceeaa28e11c5bde26d33d45 (REL-009) -> 1c07aeb824e3411c32a9d26c7db39f9e0411bedd (REL-010) -> a8cf35d698641379d072249312df7d313edf1c43 (AUD-011) -> REL-011 (the commit that adds this file; read its hash from git log).
The v1.0.0 tag does not exist.

Source hashes (must stay byte-identical; every docs task re-checks them):
- docs/architecture/CHARTER-v2.0.md: 52866 B, SHA-256 prefix 0b1bb13c. The full hash was not re-recorded here; compute it if needed. Charter 317 lines.
- CTP-SESSION-ARC-001.md: 81498 B, prefix 5a9ab64d; embeds the whole charter (lines 15-331, between <!-- CHARTER-V2.0-BEGIN --> and <!-- CHARTER-V2.0-END -->).
- CTP-SESSION-ARC-002.md: 19621 B, 174 lines, prefix c3c1091a; 13 sections; does not embed the charter.
- BOOT-SESSION-ARC-002.md: 73385 B, 504 lines, prefix 7eebc6fb.
- docs/architecture/tools/build-boot-package.mjs: deterministic generator with --charter, --ctp, --out and --check; layout: fixed preamble, BEGIN/END-PART-1-CHARTER, BEGIN/END-PART-2-CTP-ARC-00N.
- CTP-COVERAGE-ARC001-vs-ARC002.md: 88 lines (COVERED-ARC002 7, SUPERSEDED 4, COVERED-REPO 0, NOT-COVERED 0, UNKNOWN 1). It proves token presence only.
- ADR-077: file docs/adr/ADR-077-product-entry-and-generic-import-export.md, 7107 B, 29 lines, SHA-256 24f4a3c9211d3bb91519b356bb58b9d93d4b6ad1dff8ba1a49fb0eb357b104f1.
- audit/AUD-011/INVENTORY.md: 28758 B, 225 split-lines, SHA-256 3f45dbad072ac3418250577d9474b97ca672e62c353845c536c626d1c6a226fa.
- QUESTIONNAIRE-COMPTABLE-FR.html: 18630 B, SHA-256 99636e9485bbd840b24c5d24e0941a1d1f48fb558446d4e9294c9c02c35d19d1.
- Dolisoft file D:\Dev\BarakaInbox\liste_aricles.xlsx: 10918 B, SHA-256 eddb9cf875f45bf6039e4e9a9ab40f109c0f543297ab66dac50ed90b5fb6293f.
- Test baselines (HEAD 97d84aae and later): vitest 22 files / 183 tests pass; cargo 46 suites, 365 passed / 0 failed / 4 ignored; fmt, clippy, lint, typecheck, build green. AUD-011 static count 178 vs runner 183: the executor's explanation was inconsistent; trust the runner figure.

## 8. Engine and stack facts

- Tauri v2, Rust 1.98.1, React 18.3, TypeScript 5.6, Vite 5.4, Zustand 5, Tailwind 3.4, Vitest 3, rusqlite 0.32 (bundled, backup, hooks). Package name ultrapos. Node v26.7.0, npm 11.19.0, git 2.53.0.
- 13 migrations. 72 IPC commands (AUD-011 confirmed 72 in generate_handler! at src-tauri/src/lib.rs:192 and 72 in crates/commands/src/ipc.rs). Search is LIKE-prefix, not FTS5.
- Window: built in src-tauri/src/lib.rs (~174) via WebviewWindowBuilder::from_config; label main; width 1280, height 640, minWidth 1024, minHeight 600, center true, maximized true (BRK-060, guard test main_window_maximized_policy_brk060 in src-tauri/tests/window_config.rs; ADR-066 Amendment 1). Min client size is enforced via WM_GETMINMAXINFO.
- Startup: DB open and migrations run BEFORE the window is built (~170 ms). No tauri plugins. CSP is absent (null). frontendDist ../dist; devUrl localhost:5173. The only IPC before the first-run form is check_first_run_status (App.tsx:67). The DSN-001 build produced one JS bundle (index-BCz_F8rp.js, 474.37 kB).
- Language: default locale fr-DZ (DSN-001, src/stores/pos-ui-store.ts:28). Locale = "ar-DZ" | "fr-DZ" (src/types/pos.ts:8). App.tsx applies dir/lang. The language is NOT persisted anywhere (see corrections). Translations in src/lib/i18n.ts: fr 604 keys, ar 604 keys, parity enforced at compile time (I18nKey = keyof typeof fr).
- Store identity (code facts): src/stores/store-identity-store.ts caps storeName 80, storeAddress 120, storePhone 40, fiscalId 20 (nif/nis/rc/ai), footerMessage 160, footerLines 3; stored in the generic settings key/value table (0001_baseline_schema.sql:14-18) via crates/infrastructure/src/adapters/sqlite_printer_settings.rs; mirror in crates/domain/src/printing/receipt_model.rs:72-86. No shop-identity table and no logo field. Supplier nif exists (0009).

## 9. Presentation layer inventory (AUD-011, commit a8cf35d)

Read-only; file audit/AUD-011/INVENTORY.md (sections I1-I10). Findings as reported by the executor, with the architect's review caveats.

- Modules (I1): the charter passage at CHARTER-v2.0.md:181 names ten modules by title only. Classification EXISTS means only that some UI file exists; function-level completeness is UNKNOWN for all rows.
  - Vente: PosShell, CheckoutModal, QuantityModal, DiscountModal, CustomerSelectModal, RecordPaymentModal, ProductDiscovery, CartPanel/CartTable/TotalsFooter, CreditHud, KeyFooter, Topbar. EXISTS.
  - Stock: StockAdjustModal, WarehouseTransferModal, LowStockReport, StockMovementReport. EXISTS (modals plus reports; no stock list screen reported).
  - Clients & Crédit: CustomerSelectModal, RecordPaymentModal, CreditOutstandingReport, CreditHud. EXISTS, but no customer list or edit screen was reported (customers_update has no UI caller).
  - Fournisseurs & Achats: SupplierListScreen, SupplierFormModal, SupplierStatementView, PurchaseOrderFormScreen, GoodsReceiptFormScreen, SupplierInvoiceFormScreen, RecordSupplierPaymentModal. EXISTS.
  - Produits & Codes-barres: CatalogModule, ProductCreateForm (is_kit fixed false, kit UI deferred), ProductEditModal. EXISTS.
  - Documents: DocumentConversionModal and ReturnModal only; no document list or search screen reported. EXISTS (partial evidence).
  - Caisse & Clôture: OpenCashSessionModal, CloseCashSessionModal, RecordExpenseModal, CashSessionIndicator, CashSessionsHistoryReport. EXISTS.
  - Rapports: ReportsHomeScreen plus nine reports (DailySales, SalesByProduct, SalesByCategory, GrossMargin, LowStock, CreditOutstanding, StockMovement, CashSessionsHistory, UserActivity). EXISTS.
  - Utilisateurs: NO screen (only LoginScreen, FirstRunSetupScreen, session-store). MISSING (grep-based; verify).
  - Paramètres & Sauvegarde: StoreIdentityModal, BackupModal, TestPrintPreview, PrintStatusIndicator. EXISTS.
- Shell (I2): no router (App.tsx "no router dependency"); view switching via app-view-store.ts (AppView = pos | suppliers | reports | catalog). UnauthenticatedGate (App.tsx:62-92) probes ipcCheckFirstRunStatus. Manager-only NavRail (App.tsx:25-54); isManagerRole = GERANT or ADMIN; non-managers fall to PosShell. Modal host in PosShell.tsx:134-155. Toast host: NONE (pushNotice in app-store.ts has no external callers). Entry main.tsx (StrictMode).
- IPC (I3): 72 defined and registered; USED 69, UNUSED 3: catalog_get_product, catalog_define_kit, customers_update. 49 exported ipc* wrappers in src/lib/ipc.ts plus plain-name wrappers. REVIEW CAVEAT: the executor's group counts (app_health 1, auth 2, check_first_run_status 1, bootstrap_first_admin 1, catalog 5, warehouses 2, stock 3, sales 4, convert 2, documents 4, returns 2, backups 5, customers 5, credit 2, printer 3, store_identity 2, cash 7, suppliers 5, purchases 6, reports 9) sum to 71, not 72; one command is unaccounted for in the grouping.
- Stores (I4): 15 zustand stores, none persisted (no persist middleware, no localStorage in production code).
- Components (I5): components/pos (12 files), components/modals (17 files incl. ModalShell), helpers checkout-submission.ts and stock-gate.ts, hooks use-app-status, use-catalog-search, use-pos-keyboard. Largest: i18n.ts 1486 lines, ipc.ts 2339, CheckoutModal 640, BackupModal 587.
- Styling (I6): tailwind.config.js (35 lines): darkMode class, baraka palette (12 hex), fonts Inter / Cairo / JetBrains Mono, no plugins. Only global CSS is src/index.css (36 lines). Hard colors in ts/tsx: 0. Tailwind fixed class: 14 hits. Six are in PosShell.tsx (lines 86, 99, 113, 128, 149, 164), classified by the executor as floating action buttons (a violation of ADR-067; per-line classification unverified); others are modal overlays (ModalShell, RecordSupplierPaymentModal, CashSessionsHistoryReport, SupplierFormModal, SupplierStatementView) plus comments. position: fixed in CSS: 0. Physical-direction utilities 189 vs logical 153. dark: 558 uses. Arbitrary sizes [Npx] 92 (11px 58, 10px 28, 44px 5, 13px 1). Root overflow hidden at PosShell.tsx:64. Arabic-Indic digits outside i18n: 0.
- i18n (I7): 604 keys, 73 prefixes (largest: backups 81, rep 73, settings 42, returns 35, stockAdjust 30, conversion 30).
- Tests (I8): 22 files; static 178; runner 183.
- Money (I9): all money by BigInt/string helpers in src/lib/money.ts, except one visible float division on money: src/lib/format.ts:15 (centimes / 100 into an Intl fr-DZ formatter; the executor says the input is guarded to integers; unverified). parseFloat 0, Math.round 0, toFixed 1 (a duration, not money).
- Interaction (I10): keyboard handling in use-pos-keyboard.ts and use-catalog-search.ts; Escape handling in the large modals; typed RESTORE confirmation for backup restore; shortage confirm in CheckoutModal. No command palette. No barcode-scanner buffering found (wedge behavior UNKNOWN). 19 named modals.
- Hypothesis to verify: the six fixed elements in PosShell may be the only entry points to Stock, Backup, Identity and similar modals, which would violate the charter rule "No feature may be reachable only through a floating element" (charter line 225 for the reachability rule).
- Elided or unverified in the executor's inline report (the committed file has them, the architect has not seen them): tailwind.config.js verbatim, the top-15 physical/logical lists, the I7 singleton prefix list, the I10 modal list, the I5 per-file importer counts, the exact IPC group listing. The report text was NOT identical to the committed file, contrary to its claim.

## 10. ADR facts

- docs/adr/INDEX.md: table | ID | Title | Status |, status values uppercase (for example ACCEPTED). "Next free id: ADR-078." ADR-001..065 indexed; 016-061 reserved. The section header was "## 4. ADR-066..076" and was fixed to "## 4. ADR-066..077" by TASK-REL-011. Row ADR-072 still reads "Catalog migration from Dolisoft export | ACCEPTED (format pending profiling)": stale, because ADR-075 and ADR-077 amended it and AUD-005-C1 completed the profiling. Fix in a later docs task.
- ADR-066..077: viewport/window policy; single top bar; IFU; languages/digits; output profiles (thermal 58, 80, A4); single PC with roles; import design (072); canonical path; long-press destructive confirmation; products-only import (075); BARAKA_DATA_DIR (076); product entry and generic import/export (077).
- ADR-069 quotes: "the first-run default is French"; "UI locales are fr-DZ and ar-DZ; English is deferred."; "Digits are always Latin 0-9 in every locale, on screen and in printed documents."; "Arabic-Indic digits are never rendered."
- ADR-077 (Accepted; amends ADR-072 and ADR-075): D1 product entry has priority, keyboard-only fast entry (save-and-new, duplicate-similar product, session-remembered default category and unit, Enter/Tab flow). D2 name (French, required) plus name_ar (one optional Arabic alias). D3 normalized search (trim, collapse whitespace, case-fold, strip Latin diacritics, strip Arabic tashkeel and tatweel, alef variants to alef, ya variants, ta marbuta to ha; covers French name, Arabic alias, internal code, barcode, foreign reference). D4 internal code always generated; barcode optional; foreign Ref optional and never a uniqueness key. D5 duplicate key = normalized French name. D6 xlsx/csv import (no Excel needed, CSV delimiter auto-detect, UTF-8 with/without BOM, Windows-1252 fallback, template, remembered column mapping, money parsed to integer centimes without floats, units in names not parsed). D7 flow: file, mapping, preview and validation report, explicit confirmation, ONE transaction. D8 opening stock zero; quantity columns ignored/counted; negatives ignored. D9 export xlsx/csv; CSV UTF-8 with BOM and semicolon; round-trips with the template. D10 barcode label printing priority rises.
- ADR-077 Amendment 1 (verified verbatim in the AUD-011 report, lines 22-29): A manual-entry duplicate rule on every create and rename, rejected, UI offers open-existing or change-name, no warning-only mode. B first occurrence in the file is imported, later ones skipped and listed with row numbers. C "price" means the selling price (required); purchase price optional and stored as absent, not zero. D export has barcode and internal-code columns; on import a code is kept if valid and unique, else the row is rejected; generated when absent; a barcode identifies exactly one product. E Arabic-Indic (U+0660-0669) and Extended (U+06F0-06F9) digits map to 0-9 in search keys and queries. F export safety: cells starting with = + - @ in text columns are neutralized (xlsx string cell; CSV single-quote prefix; import strips exactly one leading single quote only when followed by one of those four characters); numeric columns are plain. G future implementation tasks must include test vectors for A to F.
- ADR-072 and ADR-075 each received an appended "Amendment 1 (ADR-077)".

## 11. Stamp duty (code facts) and the accountant questionnaire

- crates/domain/src/sales/pricing.rs:143-169: TIMBRE_MIN_CENTIMES = 500 (5 DA), TIMBRE_MAX_CENTIMES = 250_000 (2500 DA). compute_timbre(doc_type, cash_tendered, base) applies only to DocumentType::Ticket or Facture and only when cash tendered > 0. Stepped amount = ((base + 9999) / 10000) * 100 (1% rounded up to 100 centimes), clamped. Call site pricing.rs:327-341 (cash = sum of Cash-method payment shares; base = subtotal_ht + total_tva; the timbre is added to the total). payment.rs:10-19 PaymentMethod { Cash, Credit, Cheque, Card }; is_cash = Cash only. sales.rs:103 tva_rate_basis_points 0. Tests: crates/infrastructure/tests/sale_pricing.rs:85-93.
- NOT determined from code: whether stamp duty applies on a later cash payment of a credit sale; who bears it; how it appears on the document. "Who bears it" and "how it appears" are for the accountant.
- Questionnaire docs/legal/QUESTIONNAIRE-COMPTABLE-FR.html (commit b1ee08fa): French, 19 questions Q1-Q19 under headings A-F, Q4 table of 15 mention rows with CSS checkboxes, annex A (stamp rule, a dense single paragraph, 3 "Non déterminé" items), annex B (8 identity fields, settings storage, no logo field). No script, no external reference. It makes no legal claim and states that the programmed rule is not validated by a professional. IT HAS NOT BEEN RENDERED BY ANYONE: layout is unverified. The owner was asked (reply 19) to open it, print to PDF, and report page count, question splits across pages, whether the Q4 table and checkboxes show, cut or overlapping text, whether annexes read well. His reply is pending. Do not hand it to the accountant yet.
- Planned revision after his print preview: reformat annex A as a simple French table (criterion / programmed value) with technical references in small text; add Q20 in neutral wording with no legal article cited, about obligations for retaining customers' personal data and the debtor register; the executor reads the code for what happens on a later cash payment against a credit sale; then numbered steps for the owner and delivery to the accountant.

## 12. Dolisoft file facts (AUD-005-C1)

- D:\Dev\BarakaInbox\liste_aricles.xlsx (one "r"; the ARC-001 spelling liste_articles.xlsx was wrong). One sheet "A", A1:J26. Headers: Réf, Libellé, Qté, Prix d'achat, Valeur_ achat, Prix de vente, Valeur vente, SPEC1..3 (empty). 23 data rows; a Somme row at 26. No barcode, unit, category or supplier column; units are embedded in names; only 7 quantities, all negative; Réf not unique (REF-00005 x8, REF-00001 x4, REF-000014 x2, REF-00008 x2); 6 names with leading/trailing spaces, 2 with double spaces; no duplicate names.
- The legacy XLS was unreadable; the owner supplied this xlsx instead.

## 13. Performance, startup and host facts

- Executor host (NOT the shop PC): Windows 10 build 19045, i5-3450 4C/4T, 16 GB, SSD, DPI 144 (150%), 1920x1080 physical (1280x720 logical), work area physical [0,0,1920,1034], WebView2 154.0.4258.37, Defender on, High performance power plan; C: ~3.9 GB free, D: ~40.5 GB free.
- AUD-009: the app opens maximized, client 1280x666.7 logical, title bar 22.7 logical px; the restored window is not vertically centered (cosmetic). Self-maximize (AUD-007) was never reproduced.
- AUD-010-C1 (commit d64a3c22), runs 1 cold / 2 / 3 / 4 warm, ms since process start: nav start 1019.1 / 779.3 / 737.1 / 684.0; FCP 1403.1 / 1083.3 / 1129.1 / 1016.0; LCP 1535.1 / 1119.3 / 1129.1 / 1016.0; load 1352.8 / 994.7 / 951.4 / 810.4; LoAF none / 251 / 344 / 294 ms; IPC check_first_run_status 2.9 / 6.9 / 8.7 / 2.6 ms; 4 resources, 513338 B, 0 external origins, 0 longtasks. Warm FCP is about 1.0-1.1 s and cold about 1.4 s, on the executor host only. WebView2 floor measurement: UNKNOWN; do not retry. Isolation fingerprints equal (APPDATA f487a65b, LOCAL 9566ede0).
- Planned: a read-only host-profile task for the owner to run once on the shop PC, to set performance targets.

## 14. Corrections and errors log (carry forward)

1. AUD-010-C1: the IPC check_first_run_status time is 3-9 ms (not 3-7); run-4 load is 810 ms (not 898). Per-process CPU ratios and the word "bottleneck" in that report are unverified hypotheses.
2. ARC-001: the Dolisoft file is liste_aricles.xlsx (one r), and "AUD-005-C1 pending" is obsolete (done).
3. Language persistence: reply 15 wrongly told the owner his language choice would persist. The app persists no language (plain zustand create(), no persist). Corrected in reply 16. Persisting it is a roadmap item.
4. Withdrawn startup claims: reply 6 ("WebView2 ~4.7 s") was withdrawn in reply 9 (sampler artifact); reply 9 ("content ~4 s warm / 6.3 s cold") was withdrawn in reply 10 (late CDP attach times). The valid figures are those in section 13. CDP attach time is NOT a startup metric. The sampler gap is about 42 ms.
5. AUD-006 was withdrawn in reply 2; the owner ran it anyway (clean). AUD-008 and AUD-010 stopped early (executor budget): runtime tasks with many runs exhaust the budget.
6. BOM bugs from PowerShell 5.1 were fixed in AUD-009, AUD-010-C1 and (REL-008, 0f069dc) AUD-008. One non-JSON BOM file remains intentionally: audit/AUD-009/raw/run1-filesystem-aligned.txt.
7. Executors habitually summarize or elide instead of printing inline: REL-007 (ADR-077 only in tool output), REL-009 (summary of the HTML), REL-010 (Amendment 1 abbreviated; verified later by AUD-011), REL-008 (dropped diff context lines), AUD-011 (claimed the report was identical to the file, but abbreviated several parts; printed INDEX grep line numbers without header text). Always demand inline verbatim output and check it. Executors also disagree on CR counts via grep; byte-level Node is authoritative.
8. REL-007 reported a 63-character SHA-256 for ADR-077; the real hash is 64 hex (corrected in DSN-001).
9. The README confusion was resolved by REL-006: f2856f4 (REL-004) touched docs/architecture/README.md; there is no root README.md.
10. The AUD-011 anomalies listed in section 9 (IPC grouping sums to 71, test-count explanation, elided parts) are open.
11. Nine candidate items from the ARC-001 vs ARC-002 coverage were resolved by judgment: REL-001/REL-002 and their commits d0e93ed/977a804, persistence/mod.rs, and migrations 0001..0013 are covered or historical; the full BRK-058 hash is in section 7.

## 15. Reply log of SESSION-ARC-002

1 AUD-006 (withdrawn). 2 AUD-005-C1. 3 BRK-059. 4 AUD-007. 5 AUD-008. 6 AUD-008-C1. 7 BRK-060. 8 AUD-009. 9 AUD-010. 10 REL-005 (CTP-SESSION-ARC-002). 11 AUD-010-C1. 12 owner-question reply, no task. 13 REL-006. 14 REL-007. 15 DSN-001. 16 REL-008. 17 REL-009. 18 REL-010. 19 AUD-011. 20 REL-011 (this CTP; AUD-011 report reviewed in the same reply).

## 16. Pending work and roadmap (priority order)

Immediate:
1. Review the TASK-REL-011 report (verify: CTP hash and size, sentence line equals charter line 302, the section-15.2 mandatory-items map has no MISSING item, boot ARC-003 --check, ARC-002 boot byte-identical, INDEX diff is one line, README diff). If an item is MISSING, issue TASK-REL-011-C1 to append the missing section.
2. TASK-AUD-011-C1 (read-only, inline output): print tailwind.config.js verbatim; the I6 physical/logical top-15 lists; the I7 singleton prefix list; the I10 modal list; the I5 importer counts; the complete list of all 72 IPC commands with group, USED/UNUSED and first call site (explain the 71 vs 72 grouping gap); a reachability matrix (which UI element opens each modal and screen, with keyboard path) to test the floating-element hypothesis; and reconcile vitest 178 static vs 183 runner (run npx vitest run once if needed).
3. Questionnaire revision after the owner's print-preview result (section 11).
4. Fix the stale ADR-072 index row; optionally register amendments in the index.

Then:
5. Persist the language preference (probably per user at the users-screen design; needs a storage decision).
6. docs/design/ foundations driven by AUD-011: information architecture covering all ten modules with per-role visibility and click/key paths, flows, state catalog, tokens, component inventory, glossary AR/FR, viewport matrix including small laptops (1366x768 at 125%). Known gaps to design for: Utilisateurs screen, customer list/edit, document list, toast host, command palette, replacement of the PosShell floating buttons, scanner-wedge handling.
7. Code-splitting and bundle-size budget rule, plus CSP, before new screens.
8. tax.regime key (BRK) and SessionInfo permissions.
9. Executor inspects the logo D:\Dev\BarakaInbox\Baraka_Logo.png mechanically (dimensions, transparency, colors); add it to the repo in a later design task.
10. Read-only host-profile task for the shop PC (owner runs it once).
11. Demo dataset tool (INIT-13, about 300 products).
12. Static HTML prototypes (light default, dark available) for owner approval before React, including the fast product-entry flow (ADR-077 D1; validate speed targets there).
13. Engine and DSN phases: FTS5 migration as a separate BRK task; import/export implementation with test vectors for ADR-077 A-F and D3/D6; barcode label printing.
14. REL: installer rebuild, BRK-057, v1.0.0 tag only when charter 9.2 holds.

## 17. Human-context items from the owner (carry forward)

- Explain commercial terms in plain language. Simple numbered non-technical steps are fine (save a file, open Excel).
- English UI is deferred. No visual acceptance has been given yet.
- The real database holds real data and is never touched.
- The owner asked not to be disturbed while tasks run: warn him before the cursor moves or windows appear.
- The owner opens files by double-clicking and prints to PDF when asked; he does not edit technical text.

## 18. Executor lessons

- Tasks must print evidence inline. Do not trust "identical to the committed file" or "see file" claims.
- Keep runtime tasks to 2-3 runs. Docs tasks need no app, no mouse, no windows.
- Demand the full 40-hex commit hash and byte-level hashes (Node crypto and sha256sum, 64 hex).
- When the architect needs verbatim text from a file (charter sentences, ADR amendments, config files), the task must ask the executor to print it with line numbers and the report must be checked against the raw text, not the prose.

## 19. Instruction for the first reply of SESSION-ARC-003

Start with the REL-011 review (item 1 of section 16) if its report is available; otherwise issue TASK-REL-011 again only if the executor reports it was not completed. Header: "الرد رقم 1 — SESSION-ARC-003". Tell the owner plainly which items remain pending: the print-preview result for the questionnaire, the three open owner gates, and the unmeasured small-screen behavior.

## 20. File map (generated from git ls-files at HEAD e430f35762741001cb57afb2b0ec3cdae59a96a2)

Total tracked files: 599.

Top-level entries:
- .eslintrc.cjs — 1 files
- .gitattributes — 1 files
- .gitignore — 1 files
- .opencode/ — 9 files
- AGENTS.md — 1 files
- Cargo.lock — 1 files
- Cargo.toml — 1 files
- assets/ — 5 files
- audit/ — 202 files
- crates/ — 151 files
- docs/ — 45 files
- index.html — 1 files
- opencode.json — 1 files
- package-lock.json — 1 files
- package.json — 1 files
- pnpm-lock.yaml — 1 files
- pnpm-workspace.yaml — 1 files
- postcss.config.js — 1 files
- rust-toolchain.toml — 1 files
- src-tauri/ — 59 files
- src/ — 110 files
- tailwind.config.js — 1 files
- tsconfig.json — 1 files
- tsconfig.node.json — 1 files
- vite.config.ts — 1 files

Directory file counts (depth 2) for src, crates, src-tauri, audit:
- audit/AUD-004/ — 9 files
- audit/AUD-007/ — 8 files
- audit/AUD-008/ — 25 files
- audit/AUD-009/ — 29 files
- audit/AUD-010/ — 35 files
- audit/AUD-011/ — 1 files
- audit/measurements/ — 39 files
- audit/screenshots/ — 53 files
- crates/application/ — 22 files
- crates/commands/ — 4 files
- crates/domain/ — 34 files
- crates/infrastructure/ — 91 files
- src-tauri/capabilities/ — 1 files
- src-tauri/icons/ — 52 files
- src-tauri/src/ — 2 files
- src-tauri/tests/ — 1 files
- src/components/ — 40 files
- src/features/ — 25 files
- src/hooks/ — 3 files
- src/lib/ — 15 files
- src/stores/ — 18 files
- src/types/ — 5 files

All tracked files under docs/ (path, bytes):
- docs/DEV_RUN_PROTOCOL.md — 10845 B
- docs/adr/ADR-066-viewport-baseline-and-window-policy.md — 2151 B
- docs/adr/ADR-067-single-top-bar-no-floating-buttons.md — 1113 B
- docs/adr/ADR-068-tax-regime-ifu.md — 1419 B
- docs/adr/ADR-069-locales-fr-ar-latin-digits.md — 970 B
- docs/adr/ADR-070-output-formats-58-80-a4.md — 1106 B
- docs/adr/ADR-071-deployment-topology-single-device.md — 816 B
- docs/adr/ADR-072-catalog-migration-from-dolisoft.md — 1563 B
- docs/adr/ADR-073-canonical-repository-location.md — 642 B
- docs/adr/ADR-074-destructive-action-confirmation.md — 933 B
- docs/adr/ADR-075-catalog-import-scope-products-only.md — 1256 B
- docs/adr/ADR-076-isolated-data-directory-override.md — 1285 B
- docs/adr/ADR-077-product-entry-and-generic-import-export.md — 7107 B
- docs/adr/INDEX.md — 6673 B
- docs/adr/README.md — 1042 B
- docs/architecture/BOOT-SESSION-ARC-002.md — 73385 B
- docs/architecture/BOOT-SESSION-ARC-003.md — 86967 B
- docs/architecture/CHARTER-v2.0.md — 52866 B
- docs/architecture/CTP-COVERAGE-ARC001-vs-ARC002.md — 5578 B
- docs/architecture/CTP-SESSION-ARC-001.md — 81498 B
- docs/architecture/CTP-SESSION-ARC-002.md — 19621 B
- docs/architecture/CTP-SESSION-ARC-003.md — 33203 B
- docs/architecture/EXECUTOR-CONSTITUTION-v2.md — 12153 B
- docs/architecture/MASTER-ARCHITECTURE-ERRATA.md — 2863 B
- docs/architecture/MASTER-ARCHITECTURE-v1.0.0.md — 39086 B
- docs/architecture/README.md — 1897 B
- docs/architecture/archive/AGENTS.v1.md — 3363 B
- docs/architecture/archive/CONSTITUTION_OF_EXECUTOR.v1.0.0.md — 13720 B
- docs/architecture/tools/build-boot-package.mjs — 3713 B
- docs/audits/FIRST_LIVE_SALE_TICK_2026_000002_FORENSIC_AUDIT.md — 12207 B
- docs/audits/LOGIN_PIPELINE_TRACE_AUDIT.md — 23094 B
- docs/audits/PRODUCT_CATALOG_PIPELINE_AUDIT.md — 8896 B
- docs/audits/TASK-BRK-039-payment-overpayment-investigation.md — 8987 B
- docs/audits/TASK-BRK-040-document-conversion-contracts-investigation.md — 16088 B
- docs/audits/TASK-BRK-041-document-query-audit.md — 4973 B
- docs/audits/TASK-BRK-044-returns-and-avoir-architecture-investigation.md — 9614 B
- docs/audits/TASK-BRK-048-catalog-stock-warehouse-audit.md — 9345 B
- docs/audits/TASK-BRK-049-catalog-and-warehouses-audit.md — 9855 B
- docs/audits/TASK-BRK-050-suppliers-purchases-reports-audit.md — 7656 B
- docs/audits/WAREHOUSE_LIFECYCLE_AUDIT.md — 7633 B
- docs/legal/QUESTIONNAIRE-COMPTABLE-FR.html — 18630 B
- docs/operations/release-benchmarks-v1.0.0.md — 3987 B
- docs/operations/release-notes-v1.0.0.md — 5299 B
- docs/operations/release-procedure.md — 4697 B
- docs/operations/thermal-printer-drill.md — 5502 B

Generated mechanically; files added after that HEAD are not listed.

## 21. Addenda written in reply 21 of SESSION-ARC-002

1. Section 15.2 sentence: the sentence in section 2 is the quoted Arabic span inside charter line 302. The whole line 302 is 376 bytes and starts with the English words "Deliver with this sentence (Arabic): "; the quoted span alone is 335 bytes and matches SHA-256 e533009ad8dc33faaf6b156eec4fce448f7998e1879ac7f6f8eeba1f7d946653. Wherever section 2 says the sentence was copied from line 302, it means this quoted span. Reply 20 did not carry the sentence in its owner report; reply 21 delivered it.
2. Boot generator: docs/architecture/tools/build-boot-package.mjs derives the session tag from the --ctp file name (CTP-SESSION-ARC-NNN.md) and keeps the exact ARC-002 output. It supports only --charter, --ctp, --out and --check. Any other flag (for example --help) triggers a full build that rewrites the default output file. Never call it with other flags.
3. docs/adr/INDEX.md section 4 header now reads "## 4. ADR-066..077 - SESSION-ARC-001 (Architect, Charter v2.0)". The suffix "SESSION-ARC-001" is stale because ADR-077 was written in SESSION-ARC-002; fix it in a later docs task together with the stale ADR-072 row.
4. Evidence discipline: the REL-011 report did not print charter lines 291-305, the generator diff, the README diff or the INDEX diff inline, and it printed grep line numbers without heading text. TASK-REL-011-C1 requires them inline.
5. Reply numbering: if the owner continues in the same chat, the architect keeps counting (reply 21, 22, ...) and the next CTP is due at reply 30 (CTP-SESSION-ARC-004). If the owner opens a new chat and pastes BOOT-SESSION-ARC-003.md as the first message, that session is SESSION-ARC-003, numbering restarts at 1, and its reply 10 is the next CTP. The owner's wording in the new chat may refer to replies of SESSION-ARC-002 beyond 21; in that case ask the executor for the latest commit log.
6. Pending at the time of writing: the owner's print-preview result for the accountant questionnaire (do not hand the questionnaire to the accountant); TASK-REL-011-C1 report; TASK-AUD-011-C1 not yet issued; the three open owner gates; unmeasured small-screen behavior.
