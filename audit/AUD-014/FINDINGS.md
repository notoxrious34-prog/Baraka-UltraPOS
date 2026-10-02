# AUD-014: document renderers, customer data, print jobs, numbering, timbre, PIN groundwork
Task: TASK-AUD-014. Repo HEAD at audit time: 74074110637e6a26393e8f34e44e912075ef2555. Date: 2026-10-02.
Scope: read-only facts from code for the accountant questionnaire revision, the debt-payment proof design, and the manager-approval BRK task.

## Summary table
| id | one-line answer | status |
| S1.1 | 8 DocumentType variants with TICKET..FF storage codes | CONFIRMED |
| S1.2 | One thermal raster pipeline + capture file sink + preview model, no PDF/HTML | CONFIRMED |
| S1.3 | No A4 renderer exists | REFUTED |
| S1.4 | All 8 types share the one pipeline; only prefix and timbre gating differ | CONFIRMED |
| S1.5 | Single 80 mm path (576 dots); no 58 mm path | CONFIRMED |
| S2.1 | ReceiptLine, ReceiptTotals, StoreIdentity, ReceiptDocument, PreviewLine fields listed | CONFIRMED |
| S2.2 | No print model carries customer data | CONFIRMED |
| S2.3 | No customer name/phone copy in documents, items, print payload; audit varies | CONFIRMED |
| S2.4 | Identity fields print only when non-empty, except store_name always emitted | CONFIRMED |
| S2.5 | Language in documents.language AR/FR; DUPLICATA marking exists | CONFIRMED |
| S3.1 | print_jobs schema quoted below; job_type/payload added by 0004, error cols by 0006 | CONFIRMED |
| S3.2 | Kind RECEIPT/DUPLICATA, status PENDING/PROCESSING/DONE/FAILED, code-enforced, no SQL CHECK | CONFIRMED |
| S3.3 | A job cannot be inserted without a documents row (NOT NULL + REFERENCES) | CONFIRMED |
| S3.4 | Payload is a small JSON id snapshot; hydrate_receipt rebuilds the model from rows | CONFIRMED |
| S3.5 | Enqueued in the sale transaction (checkout step 8, same tx handle) | CONFIRMED |
| S3.6 | Reprint needs ReprintDocument perm + existing row; DUPLICATA job watermarks | CONFIRMED |
| S3.7 | A document-less job needs a migration (NOT NULL + FK + worker row requirement) | CONFIRMED |
| S4.1 | document_sequences(type, fiscal_year); allocate_in_tx upserts; cap 999999 | CONFIRMED |
| S4.2 | Key is free text, no CHECK; number is PREFIX-YEAR-06d | CONFIRMED |
| S4.3 | Allocator needs no documents row; a new key needs no migration | CONFIRMED |
| S4.4 | AVOIR uses the same allocator; cash-session report has no gapless number | CONFIRMED |
| S5.1 | compute_timbre called only from price_document; never from credit-payment or returns | CONFIRMED |
| S5.2 | Timbre base is the WHOLE HT+TVA total, not the cash portion | CONFIRMED |
| S5.3 | Both worked examples give 10000 centimes = 100.00 DA | CONFIRMED |
| S5.4 | 6 pricing tests; no test covers timbre on a mixed cash+credit tender | CONFIRMED |
| S5.5 | Timbre is its own receipt line, labels Timbre / الطابع | CONFIRMED |
| S5.6 | Cheque/Card-only tender yields timbre 0 (only Cash counts) | CONFIRMED |
| S5.7 | Returns never refund timbre (hardcoded 0) | CONFIRMED |
| S6.1 | No amount-to-words function anywhere | REFUTED |
| S7.1 | users: password_hash NOT NULL, pin_hash nullable; lockout columns added by 0002 | CONFIRMED |
| S7.2 | VerifyPin: PinNotSet/InvalidCredentials, no counter, lockout or rate limit | CONFIRMED |
| S7.3 | Password set at bootstrap only; no production PIN setter exists | CONFIRMED |
| S7.4 | Argon2id OWASP defaults, same hasher for password and PIN, PHC strings | CONFIRMED |
| S7.5 | Failed logins/PIN checks write no audit row (counters only) | REFUTED |
| S7.6 | No approval table or approval audit event; override audit rows exist | CONFIRMED |
| S8.1 | Receipt prints payment method lines incl. CREDIT; no customer or balance | CONFIRMED |
| S9 | Five blocking-relevant extras listed below | CONFIRMED |

## S1 document types and renderers
S1.1 CONFIRMED | crates/domain/src/sales/document.rs:16 | 8 variants Ticket Facture Bl Devis Avoir Bc Br Ff;
  > codes TICKET FACTURE BL DEVIS AVOIR BC BR FF at :40, prefixes TICK DEV AVO BC BR FF at :53.
S1.2 CONFIRMED | renderers: render_canvas rasterizer.rs:220 ReceiptDocument to GrayImage; receipt_lines rasterizer.rs:226 ReceiptDocument to Vec LayoutLine;
  > encode_gs_v0 rasterizer.rs:233 canvas to ESC/POS bytes; render_lines rasterizer.rs:161 lines to image;
  > build_identity_lines blocks.rs:116 and build_footer_lines blocks.rs:156 StoreIdentity/footer to lines; write_capture capture.rs:50 bytes to file path;
  > send_raster transport_serial.rs:59 bytes to serial; hydrate_receipt print_worker_service.rs:63 DB rows to ReceiptDocument; sample_test_receipt printing.rs:233 diagnostics sample;
  > decode.rs structural decoder for tests.
S1.3 REFUTED | searched A4, a4, PDF, pdf, html, page_size, paper, Facture layout over crates and src-tauri;
  > pdf zero hits, html zero hits, A4 zero hits, page_size only report pagination reports.rs:18;
  > paper only paper_width_dots in 0006 and rasterizer.rs:19, Facture only the enum variant. No PDF/HTML/A4 renderer exists.
S1.4 CONFIRMED | one shared pipeline for every type: worker hydrates any documents row by id (print_worker_service.rs:63) and renders via receipt_lines (rasterizer.rs:226).
  > Per-type differences found: number prefix (document.rs:53) and timbre gating to Ticket/Facture (pricing.rs:161). No per-type renderer branch found in the printing directories.
  > Table: Ticket Facture Bl Devis Avoir Bc Br Ff -> thermal rasterizer each;
  > A4/PDF/HTML -> none found for all.
S1.5 CONFIRMED | single width: 576 dots for 80mm at 203dpi (rasterizer.rs:19, canvas comment :3) and setting printer.paper_width_dots 576 (0006). 58 mm search (58mm, 58 mm, 384): zero hits.
  > No distinct 58 mm path.
## S2 customer data and identity
S2.1 CONFIRMED | ReceiptLine receipt_model.rs:33 designation quantity_text unit_price_text total_text; ReceiptTotals :46 subtotal_ht tva discount timbre total_ttc paid change;
  > StoreIdentity :70 store_name store_address store_phone nif nis rc ai ticket_footer_message;
  > ReceiptDocument :161 language shop_name number created_at lines totals payments is_duplicata store_identity footer_message; PreviewLine :110 kind centered size_px text.
S2.2 CONFIRMED | no field above carries customer name, phone, address, fiscal id or balance;
  > the thermal renderer consumes the same ReceiptDocument (rasterizer.rs:226), so it prints no customer data either.
S2.3 CONFIRMED | documents 0001:171 holds party_type/party_id only, no name or phone copy; document_items 0001:199 holds product snapshot only;
  > print payload holds document_id/number/total only (checkout :473);
  > payments.customer_id is a nullable FK (0001:219). Audit row contents are per-action (example bootstrap row carries username/role only); exhaustive customer-data-in-audit is UNKNOWN, see register.
S2.4 CONFIRMED | thermal prints NIF NIS RC AI address phone only when non-empty, each as CAPTION value (blocks.rs:137). store_name is emitted unconditionally even when empty (:124).
  > Empty footer falls back to the legacy thanks line (rasterizer.rs:96). Footer keeps at most 3 non-empty lines (blocks.rs:156).
S2.5 CONFIRMED | language stored in documents.language with CHECK AR|FR (0007) and read into Document.language (sqlite_document_repository.rs:173, :83);
  > legacy document_language column also written (0001:179).
  > Renderer maps Ar/Fr to Arabic/French layout (print_worker_service.rs:244).
  > DUPLICATA marking exists: is_duplicata receipt_model.rs:177 renders star DUPLICATA star lines in both languages (rasterizer.rs:41).
## S3 print jobs and reprint
S3.1 CONFIRMED | 0001:279 CREATE TABLE print_jobs with id PK and document_id NOT NULL REFERENCES documents(id);
  > status TEXT NOT NULL, attempt_count DEFAULT 0, is_duplicate DEFAULT 0, payload_json NOT NULL, created_at, updated_at;
  > 0004:18 adds job_type TEXT, :19 adds payload TEXT; 0006 adds last_error and claimed_at. No CHECK, no extra index.
S3.2 CONFIRMED | kinds RECEIPT/DUPLICATA (print_job.rs:58) and states PENDING/PROCESSING/DONE/FAILED (:23); enforced by from_code parsers (:33, :66), no SQL CHECK exists.
S3.3 CONFIRMED | exact text document_id TEXT NOT NULL REFERENCES documents(id) (0001:279). A job without a documents row violates NOT NULL and the foreign key, so it cannot be inserted today.
S3.4 CONFIRMED | payload is document_id/number/total_ttc_centimes JSON (sqlite_checkout_repository.rs:473), not a rendered model.
  > hydrate_receipt rebuilds ReceiptDocument from document, items and payments rows (print_worker_service.rs:63).
S3.5 CONFIRMED | enqueue_in_tx with RECEIPT inside the sale transaction, comment 8. Transactional outbox print job (sqlite_checkout_repository.rs:481), same tx handle as the sale writes.
S3.6 CONFIRMED | ReprintDocumentUseCase printing.rs:18 requires ReprintDocument permission (:43);
  > requires an existing documents row (:45) then enqueue_reprint with DUPLICATA (sqlite_print_job_repository.rs:187);
  > watermark applied when job_type is DUPLICATA (print_worker_service.rs:251).
S3.7 CONFIRMED | migration required: the NOT NULL plus REFERENCES constraint (0001:279) forces it, and the worker rejects a missing row with PrintJobNotFound (print_worker_service.rs:240).
## S4 numbering
S4.1 CONFIRMED | document_sequences(type, fiscal_year, last_sequence) PK on both (0001:271).
  > allocate_in_tx (sqlite_document_sequence_repository.rs:37) inserts 1 or increments, rejects above 999999 (:64). Checkout calls it in-tx (sqlite_checkout_repository.rs:251).
S4.2 CONFIRMED | type column is free text, no CHECK anywhere in 0001..0013. Number format PREFIX-YEAR-06d e.g. TICK-2026-000001 (sequence.rs:22, cap MAX_SEQUENCE_NUMBER :10).
S4.3 CONFIRMED | allocate_in_tx only upserts the sequences row, takes any type string, no documents row needed (:42). A new key needs no migration; the only cap is 999999 per type-year.
S4.4 CONFIRMED | AVOIR allocates with the same allocator and AVO prefix (sqlite_return_repository.rs:165, :175).
  > CashSessionReport carries session, movements, totals, is_final and no number field (ports.rs:1336);
  > no number/sequence/allocate found in sqlite_cash_session_repository.rs, so the Z-report has no gapless number.
## S5 stamp duty
S5.1 CONFIRMED | compute_timbre defined pricing.rs:156, called only at pricing.rs:337 inside price_document.
  > price_document callers are cart verify (sales.rs:115), draft preview (sales.rs:692) and tests. No call from any credit-payment path;
  > returns hardcode timbre 0 (returns.rs:388).
S5.2 CONFIRMED | base is the whole total: timbre_base equals subtotal_ht plus total_tva (pricing.rs:333), while cash_tendered sums only is_cash payments (:327).
  > Rule quote: Applies only to TICKET/FACTURE documents with a cash tender greater than 0 (:155). So a 100 DA cash plus 9900 DA credit sale stamps the full 10000 DA base.
S5.3 CONFIRMED | arithmetic by hand, all-integer formula ((base+9999)/10000)*100 clamped to 500..250000: (a) base 1000000c gives (1009999/10000)=100, times 100 = 10000c = 100.00 DA.
  > (b) same base gives the same 10000c = 100.00 DA;
  > the cash amount only gates greater than 0.
S5.4 CONFIRMED | tests in pricing.rs: line_gross_and_tva_shapes (:371);
  > allocation_sums_exactly_with_tiebreak (:383), timbre_bounds_and_gating (:438), tender_change_rules (:463);
  > full_document_prices_end_to_end (:503), empty_cart_and_bad_discounts_rejected (:537);
  > the mixed test covers settle_payments only; no test prices timbre on a mixed cash plus credit tender.
S5.5 CONFIRMED | timbre is its own totals line from receipt.totals.timbre (rasterizer.rs:68), labels Timbre in French and الطابع in Arabic.
S5.6 CONFIRMED | is_cash is true only for Cash (payment.rs:47) and compute_timbre needs cash_tendered greater than 0 (:162); a Cheque-only or Card-only tender yields 0.
S5.7 CONFIRMED | returns.rs:365 comment timbre is never refunded always 0, hardcoded timbre_centimes 0 at :388.
## S6 amount in words
S6.1 REFUTED | searched en toutes lettres, in_words, to_words, lettres, amount_words, tafqeet, number_to_words, words_for across crates, src-tauri and src: zero hits.
  > No amount-to-words function exists.
## S7 PIN and approval groundwork
S7.1 CONFIRMED | users 0001:21 has password_hash NOT NULL and pin_hash nullable, no lockout columns at baseline; 0002 adds failed_login_attempts NOT NULL DEFAULT 0 and locked_until.
S7.2 CONFIRMED | VerifyPinUseCase mod.rs:198 checks the stored PIN hash (:219);
  > yields PinNotSet when none (:222) and InvalidCredentials when wrong, with the note no lockout interaction in this task (:197);
  > password login locks after MAX_LOGIN_ATTEMPTS 5 (:35) for LOCKOUT_SECONDS 300 (:38). The PIN path has no counter, lockout or rate limit.
S7.3 CONFIRMED | bootstrap hashes the password only (mod.rs:322 BOOTSTRAP_PASSWORD_REQUIRED) and inserts pin_hash NULL (sqlite_user_repository.rs:185).
  > No set_pin, change_pin or create-with-pin exists in non-test code (only test fakes at mod.rs:432). So today no production path ever sets a PIN.
S7.4 CONFIRMED | Argon2id with crate-default OWASP parameters; passwords and PINs are two slots with the identical algorithm, hashes stored as PHC strings (argon2_hasher.rs:1, :14).
S7.5 REFUTED | searched audit and Audit in use_cases/mod.rs: zero hits. Failed password logins only bump counters (record_login_failure ports.rs:56, reset :63); failed PIN checks write nothing.
S7.6 CONFIRMED | no approval table or event in any migration (approval/override search empty) and no single-use approval mechanism in crates (only a role comment at role.rs:58).
  > Override audit rows do exist for F7 leaves, e.g. NEGATIVE_STOCK_OVERRIDE on stock_levels (sqlite_audit_log.rs:66).
## S8 credit sale on receipt
S8.1 CONFIRMED | hydrate_receipt prints one tender line per stored payment as method code plus amount (print_worker_service.rs:84), so a credit part prints as a CREDIT line.
  > The model has no customer and no balance field (receipt_model.rs:161), so neither prints anywhere.
## S9 extras (max 5)
S9.1 | Item designation is not snapshotted: the renderer uses live product name_fr with product-id fallback;
  > even for Arabic receipts, so a renamed product changes old reprints. File: print_worker_service.rs:76.
S9.2 | Retried RECEIPT jobs also watermark: any job with attempt_count above 0 renders DUPLICATA (print_worker_service.rs:251), not only explicit reprints.
S9.3 | from_document_language (receipt_model.rs:22) has no production callers;
  > the worker maps the DocumentLanguage enum instead (print_worker_service.rs:244), and the test-print IPC maps ar strings directly (ipc.rs:2461).
S9.4 | An empty store_name still emits an emphasis header line (blocks.rs:124 has no is_empty guard), unlike every other identity field.
S9.5 | Change prints only when not 0.00 (rasterizer.rs:79), so exact-tender receipts show no change line while paid still shows.

## UNKNOWN register
U-S2.3 | exhaustive customer data inside audit_log rows: row contents are per-action JSON and were not enumerated action by action in this audit.
UNKNOWNs: the single item above; everything else is CONFIRMED or REFUTED with cited evidence.

