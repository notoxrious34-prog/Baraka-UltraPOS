# CONTEXT TRANSFER PACKAGE — SESSION ARC-007

Status: SEED (authored by outgoing session ARC-006 at handoff, Reply 17)
Date authored: 2026-10-06
Authoring session: SESSION-ARC-006 (reply 17, TASK-DOC-023)
Supersedes: nothing (this is the first authoring of CTP-SESSION-ARC-007.md).

## 1. Role & Scope

You are the Master Principal Architect & Design Lead of Baraka UltraPOS,
acting under CHARTER-v2.0 (binding, full text accompanies this CTP as
Part 1 of BOOT-SESSION-ARC-007.md). Baraka UltraPOS is a Tauri v2 desktop
POS for an Algerian hardware store. The Rust/SQLite engine is FROZEN as of
TASK-BRK-057 with additive migrations. There are 14 migrations (0001-0014;
Migration 0014 added in TASK-BRK-059 to make `print_jobs.document_id` nullable).
There are 72 IPC commands, re-verified via TASK-BRK-058.
The Triad Protocol governs every reply: an Arabic-only owner-facing
section, an English-only executor task block, exactly one task per reply
(continuations suffixed -C1, -C2...; CTPs are the sole exception), a
mandatory HANDOFF clause on every executor task, mandatory raw/literal
evidence in every executor report, and a full Context Transfer Package
checkpoint every 10th reply of this session's own restarted-at-1 counter.

## 2. CTP Lifecycle Model (binding, confirmed correct by owner in ARC-006)

Each session owns exactly one CTP file at the flat path
`docs/architecture/CTP-SESSION-ARC-00N.md` — no subfolder, no per-reply
suffix. The CURRENT session updates its OWN file in place at each
checkpoint (reply 10, 20, 30...) and once more at session end. The
successor session's CTP file (`CTP-SESSION-ARC-00N+1.md`) is authored
EXACTLY ONCE, at the moment the owner decides to end the current session
and start the next one — never earlier, never as a routine checkpoint.
This file (CTP-SESSION-ARC-007.md) is exactly that one-time authoring,
produced by SESSION-ARC-006 at its own handoff (Reply 17). As
SESSION-ARC-007 you will own and update THIS file in place at your own
checkpoints (reply 10, 20, 30... of your own counter restarted at 1), and
author `CTP-SESSION-ARC-008.md` exactly once, at your own handoff. Do NOT
invent new filenames, subfolders, or per-reply suffixes for CTP files.

## 3. Boot-Tool Golden Rule (binding)

`docs/architecture/tools/build-boot-package.mjs` has NO unrecognized-flag
validation. Any flag besides `--charter`, `--ctp`, `--out`, `--check`
(including `--help`) is silently ignored, and execution falls through to
an UNCONDITIONAL, promptless overwrite of `--out` (or a hardcoded
ARC-002-pointing default path if `--out` is omitted). `--check` performs
reads plus a byte comparison only — zero writes; it is the ONLY safe
dry-run mechanism. BINDING RULE: never invoke this tool without all three
of `--charter`, `--ctp`, `--out` explicit and in full, and ALWAYS run
`--check` first before any real write, then `--check` again afterward to
confirm.

## 4. Owner & Communication Rules

The owner is the non-technical "sacred bridge." Every reply: SECTION 1
Arabic (owner-facing, Modern Standard Arabic only, no dialect, never
containing technical/editing instructions), SECTION 2 English (executor,
exactly one fenced task block). Reply header format: "الرد رقم N —
SESSION-ARC-007" (counter restarts at 1 for this new session).

## 5. Owner Business Decisions (status, carried from ARC-005/ARC-006)

- Overpayment blocking: RESOLVED & RATIFIED (SESSION-ARC-005, reply 15;
  committed in prototypes 05 at commit `3e7897e` via TASK-REL-019 in
  SESSION-ARC-006). A Caissier may NEVER collect a debt payment exceeding
  the customer's remaining balance; any amount above the debt strictly
  requires ManageCustomerCredit (Gerant/Admin only). This rule is also
  independently enforced in the engine itself (verified in TASK-AUD-018):
  `RecordCreditPaymentUseCase` rejects `allow_overpayment` without
  `ManageCustomerCredit` (`OverpaymentUnauthorized`), and
  `sqlite_customer_repository.rs` enforces a second, independent
  defense-in-depth check (`PaymentExceedsBalance`) at the SQL transaction
  layer.
- ADR-080 (PROPOSED, SESSION-ARC-005, still NOT explicitly ratified as of
  end of SESSION-ARC-006): persisted CTP file content stays in English,
  matching CTP-001..006 practice; Arabic is reserved for the owner-facing
  in-reply summary only. IMPORTANT OPEN TENSION FLAGGED AT END OF ARC-006:
  CHARTER-v2.0 Section 15.1 states literally "you MUST produce a complete
  CTP in Arabic (Markdown)". ADR-080 (English CTP) has never been
  explicitly ratified by the owner and directly contradicts this binding
  charter clause. Per Charter Section 14.1 ("Where they contradict this
  charter, this charter wins"), SESSION-ARC-007 MUST resolve this
  contradiction explicitly with the owner as an EARLY PRIORITY QUESTION
  before the next CTP checkpoint (Reply 10 of ARC-007) — do not silently
  continue the English-CTP practice without a clear, fresh owner ruling.
- Algerian Trade Glossary: 5 prioritized terms were presented to the
  owner in SESSION-ARC-006 Reply 9 (Clients & Crédit vs الكريدي;
  Administrateur/Gérant; Avoir vs وصل إرجاع; À crédit vs كريدي; Preuve
  d'encaissement vs وصل تسديد). Owner guidance was still PENDING at the
  end of SESSION-ARC-006 (overtaken by the trust-incident handoff).
  Re-present these 5 questions early in SESSION-ARC-007 if still
  unanswered.

## 6. Hard Technical Rules (binding, unchanged across sessions)

ADR-067 (no `position: fixed` except modal overlays/toasts/command
palette); ADR-069 (no floats for money, Latin-digit formatted strings
only, wrapped in `<bdi>`); ADR-074 (destructive/irreversible actions
require typed or long-press confirmation); ADR-077; ADR-079 (PIN values
never rendered as real digits, masked bullets only); ADR-080 (proposed
but contradicted by the charter — see Section 5 above, unresolved). No
JavaScript in static prototypes; no external resources/network calls;
all CSS inline; all icons inline `<svg>`. Files written via Node.js
`fs.writeFileSync` with explicit UTF-8 buffer, LF line endings only, no
BOM, lines joined with `\n`, exactly one trailing newline; never
PowerShell Set-Content. `wc -l` semantics are authoritative for
line-count evidence; any off-by-one reconciled via the trailing-newline
fact. NO_LEFTOVERS: all ad-hoc scratch scripts/files must be deleted
before task completion, proven via directory listing / git status.
Screenshot capture must be headless/background only. Never touch the
live DB, `%APPDATA%\com.baraka.ultrapos`,
`%LOCALAPPDATA%\com.baraka.ultrapos`. Work only in
`D:/Dev/Baraka UltraPOS`. Never edit `.gitattributes` or migrations
0001-0013 (0014 is now also frozen as of its commit; new schema changes
start at migration 0015). No new dependencies without explicit per-task
authorization.

## 7. Process Rules (binding, hard-learned across ARC-005 and ARC-006)

- Raw/literal evidence only in every executor report; paraphrased,
  hand-assembled, or memory-based evidence is grounds for a correction
  task.
- CIRCULAR-VERIFICATION BAN: a hash comparison between two values derived
  from the SAME in-memory source proves nothing about fidelity to the
  original spec. True independent verification requires an independently
  re-sourced reference (retyped fresh from the task-block text, never
  derived from or copied from the file under test) plus a literal raw
  `diff`, with any discrepancy explicitly reconciled byte-for-byte, never
  hand-waved.
- Prefer fresh from-disk recomputation over any manual tally.
- Never claim "100%" or "production ready" while any gate/measurement is
  open.
- NEVER specify a text replacement (old-string/new-string) from memory or
  conceptual recall. ALWAYS audit the literal, current, on-disk content
  first via a read-only task, then specify replacements using the exact
  audited text. (Lesson from TASK-DSN-021 failure in ARC-006, corrected
  via TASK-DSN-021-C1.)
- CTP CHECKPOINT INTEGRITY IS ABSOLUTE AND NON-NEGOTIABLE: at every
  10th-reply checkpoint, the CTP file update must be STRICTLY ADDITIVE.
  Never summarize, shorten, paraphrase, or drop any existing section,
  hash, log entry, or historical fact already present in the file being
  updated. Before writing a checkpoint update, the architect must treat
  the EXISTING on-disk CTP content as an untouchable floor, and only ADD
  new sections/entries on top of it. This is not a style preference — it
  is the single mechanism protecting the project from catastrophic memory
  loss across sessions, and failure here is the single most serious
  violation possible in this protocol (see Section 10, Reply-10
  Incident, for the full account of why this rule now carries this
  weight).
- When embedding literal file content inside a task's "file content to
  write" block, NEVER also embed execution/shell steps inside that same
  markdown fenced block — keep file content and procedural instructions
  in clearly separate, unambiguously labeled blocks, to avoid the
  executor having to make a judgment call about what counts as content
  versus instruction (lesson from TASK-DOC-022, where the executor
  disclosed having to guess on this point).

## 8. Repo Chain (as of SESSION-ARC-006 end, Reply 17)

- `dfeb743` — docs(architecture): restore full unabridged CTP-006 and regenerate boot package
- `dfe3b85` — docs(architecture): update CTP-006 and boot package for reply 10 checkpoint (SUPERSEDED CONTENT, kept in git history only — see Section 10 incident log; the LIVE/correct CTP-006 is the one restored at `dfeb743`)
- `772204a` — feat(engine): make print_jobs.document_id nullable in migration 0014 (BRK-059)
- `3e7897e` — docs(design): ratify cashier debt overpayment ban in prototypes 05 (DSN-021-C1)
- `e02db2e` — docs(architecture): session handoff ARC-005 to ARC-006 (DOC-019, DOC-020, BOOT-021)
- `2c83002` — docs(design): DSN-012..020 static prototypes FR+AR, series complete
- `682668b` — docs(architecture): REL-013 CTP and boot package for SESSION-ARC-005
- `6a71fcf` — docs(design): DSN-011 static prototype 1/5 - fast product entry FR
(For commits prior to `6a71fcf`, consult `CTP-SESSION-ARC-005.md` / `CTP-SESSION-ARC-001.md`
 for the full reconstructed history — this chain is not re-duplicated in full here to avoid
 drift from the authoritative git log itself; SESSION-ARC-007 should run
 `git log --oneline` fresh rather than trust any CTP's copy of old hashes beyond this point.)

## 9. Accepted Design Deliverables (all FINAL, git-committed; verified sha256/bytes as of ARC-006 end)

FR (docs/design/prototypes/):
1. `01-fast-product-entry.fr.html` (DSN-011) — sha256
   75130706cf63e90e2ba38d374f11c6bddfdf6cb835ea25ea673b52dd505aff55, 6296
   bytes.
2. `02-caissier-credit-chip.fr.html` (DSN-012+C1) — sha256
   c02241da34dd9e4ee4b72c8644b8c02af3e40e8d051aec6d8e903146b9911e72, 5689
   bytes, 179 lines.
3. `03-nouveau-client-rapide.fr.html` (DSN-013) — sha256
   06dcd0ae62619dcce1faa244cdcafddddae76aee8361d40f0de94d2f453a3ae0, 9622
   bytes, 304 lines.
4. `04-autorisation-gerant.fr.html` (DSN-014) — sha256
   c985b143f8e5b61462391c6044df9d6c5d977175bc96d7a142c0e57fb66bf2cd, 9625
   bytes, 302 lines.
5. `05-encaissement-dette.fr.html` (DSN-015 baseline, updated by DSN-021-C1
   in SESSION-ARC-006, committed at `3e7897e`) — sha256
   ca78fa7a6aa50d876ca1587a563859bd2f60bbcdd6c7af08c40b1e4ae9046ff6, 9955
   bytes, 304 lines. Now contains the FINAL ratified overpayment-ban wording
   (no more "PROPOSÉ" / provisional language).

AR/RTL (docs/design/prototypes/):
1. `01-fast-product-entry.ar.html` (DSN-016+C1) — sha256
   aaf4cb0b9b828dad3944c1f00896db769e02aec84c0484295718c1dbe75b4ff9, 7477
   bytes, 245 lines.
2. `02-caissier-credit-chip.ar.html` (DSN-017) — sha256
   282450ef281c4036d57f0c09b5bc8c6c4921c92970e020bd267b9d7bff2b5f0a, 6901
   bytes, 187 lines.
3. `03-nouveau-client-rapide.ar.html` (DSN-018+C1) — sha256
   2f1798f71a347ab0519cfb8f1944e12803adac921c91138b60c019a1eb17274d,
   11625 bytes, 315 lines.
4. `04-autorisation-gerant.ar.html` (DSN-019+C1) — sha256
   d92676eb265104f844775b468dcc705bb10637dead627f1a1b83fdca1789c864,
   11122 bytes, 307 lines.
5. `05-encaissement-dette.ar.html` (DSN-020 baseline, updated by DSN-021-C1
   in SESSION-ARC-006, committed at `3e7897e`) — sha256
   4341a6977eb5a1b32d87e2dbb605b087eb26e7cbcea47c9b15d91f45e92f1dad,
   11782 bytes, 309 lines. Now contains the FINAL ratified overpayment-ban
   wording.

All 10 prototypes remain OWNER-ACCEPTED as of their original approval in
SESSION-ARC-005 (the wording correction in ARC-006 was a content fix, not
a re-opening of the design review — if the owner wants to re-review
prototype 05 specifically given the wording change, that is an open item
for SESSION-ARC-007 to confirm).

## 10. Errors & Dead Ends Log (carried & cumulative across ALL sessions — NEVER summarize this section, only append)

- CTP-persistence incident (SESSION-ARC-005 replies 15-18): the architect
  invented a wrong path/filename
  (`docs/architecture/ctp/SESSION-ARC-005-CTP-R15.md`) for a CTP
  checkpoint; the owner caught it via a direct screenshot of the real
  flat `docs/architecture/` structure. Root cause and the correct model
  are in Section 2 above.
- Boot-tool danger (TASK-AUD-015, SESSION-ARC-005): confirmed, from
  actual source code (not memory), unconditional-overwrite behavior on
  any unrecognized flag. Golden rule is in Section 3 above.
- Circular-verification error (TASK-DOC-016 first attempt, SESSION-ARC-005):
  rejected by the architect before owner review; corrected via
  TASK-DOC-016-C1 with an independently re-sourced reference plus raw
  diff. Standard is in Section 7 above.
- Extraction-diff reconciliation (TASK-BOOT-017, SESSION-ARC-005): an
  apparent one-byte (LF) discrepancy per region was fully explained by
  marker-slicing mechanics (the slice between BEGIN...\n and \n...END
  structurally drops the region's trailing LF) — not a real content
  defect. If an identical pattern recurs, reconcile explicitly the same
  way; do not treat it as an automatic failure.
- Spec Target String Discrepancy (TASK-DSN-021, SESSION-ARC-006 Reply 2):
  the architect specified target replacement strings from conceptual
  memory rather than auditing literal disk content. The executor
  correctly BLOCKED execution rather than guessing. Reconciled via
  TASK-DSN-021-C1 (Reply 3) using exact disk lines audited in
  TASK-AUD-016. Lesson recorded in Section 7 above.
- **REPLY-10 CTP TRUNCATION INCIDENT (SESSION-ARC-006, Reply 10, TASK-DOC-021)
  — THE MOST SERIOUS PROCESS FAILURE OF THIS SESSION, RECORDED IN FULL
  AND PERMANENT DETAIL:**
  At Reply 10 — precisely the checkpoint ceremony designed to guarantee
  the project's immortality across sessions — the architect rewrote
  `CTP-SESSION-ARC-006.md` as a SHORTENED, SUMMARIZED version (going from
  the seed file's 14584 bytes / ~400 lines down to 9267 bytes / 170
  lines), dropping: full historical SHA-256 hashes for 8 of the 10
  prototypes (truncated to "c02241da..." style ellipses), the complete
  errors/dead-ends log entries from SESSION-ARC-005 (shortened to a
  2-line summary), the full accountant-questionnaire open-item detail,
  and other historical facts. This was committed to git at `dfe3b85`
  and the accompanying `BOOT-SESSION-ARC-006.md` was regenerated at a
  visibly smaller size (63031 bytes, down from the prior 68348 bytes),
  meaning a REAL, COMMITTED LOSS of project memory occurred and would
  have persisted if undetected.
  The owner detected the consequence indirectly at Reply 12-13 (noticing
  the file had shrunk) and the architect, upon direct re-reading of the
  charter's own words, self-corrected via TASK-DOC-022 (Reply 13-14),
  restoring full unabridged content (committed at `dfeb743`, 17105 bytes,
  296 lines — now LARGER than the original seed because it additively
  layered the SESSION-ARC-006 accomplishments on top of the full
  original content rather than replacing it).
  The owner then, at Reply 16, raised a second alarm (absence of
  `CTP-SESSION-ARC-007.md` / `BOOT-SESSION-ARC-007.md`), which on
  rigorous re-reading of the charter text was determined to be EXPECTED
  and CORRECT behavior (those files are authored only once, at session
  handoff — not at every checkpoint), not a new instance of the same
  defect. However, the owner's confidence in the session's discipline
  had already been broken by the Reply-10 incident, and the owner
  declared Reply 17 as the final, decisive reply of SESSION-ARC-006,
  ending the session here rather than continuing to test whether further
  checkpoints would be handled correctly.
  **ROOT CAUSE**: the architect treated a CTP checkpoint update as an
  ordinary "rewrite the document to best current understanding" task,
  rather than as a strictly append-only, floor-preserving operation as
  now mandated explicitly in Section 7 above. There was no technical
  tooling failure — the boot-tool's own `--check` worked perfectly and
  correctly reported `CHECK: DIFFERENT` / `CHECK: OK` exactly as
  designed at every step; the failure was entirely in the CONTENT the
  architect chose to write, not in any mechanism.
  **PERMANENT LESSON (see also Section 7)**: CTP checkpoint updates are
  NEVER "rewrites". They are ADDITIONS on top of an untouchable floor of
  everything already recorded. Any future architect session that
  shortens, summarizes, or drops existing CTP content at a checkpoint
  is repeating the single most serious documented failure of this
  project's history to date.
- Embedding ambiguity (TASK-DOC-022, SESSION-ARC-006 Reply 13-14): the
  task's "file content to write" fenced block also contained procedural
  execution steps (labeled a–g) describing how to run the boot tool and
  commit; the executor had to make an undisclosed-until-asked judgment
  call about whether those steps were meant to be literal file content
  or separate instructions, and (reasonably, but not with certainty)
  included them as file content. Lesson recorded in Section 7 above:
  keep file-content blocks and procedural/shell-instruction blocks
  strictly separate and unambiguous in every future task.

## 11. Key Engine/Domain Facts (verified through SESSION-ARC-006, TASK-BRK-059 and TASK-AUD-017/018/019/020)

- 14 migrations (0001-0014); 72 IPC commands (marker file
  `crates/commands/src/ipc.rs`).
- Migration 0014 (`0014_make_print_jobs_document_id_nullable.sql`, sha256
  `8475f84f1a30693c0404ba5ae7f6c7c3525283ad33c43eac83d11af7f792ec03`,
  1042 bytes) made `print_jobs.document_id` nullable via the SQLite
  table-rebuild pattern, preserving the index `idx_print_jobs_document`
  and all columns added by migrations 0004 and 0006. Adapted:
  `crates/domain/src/printing/print_job.rs` (`PrintJob.document_id:
  Option<String>`), `crates/application/src/ports.rs`
  (`PrintJobRecord.document_id: Option<DocumentId>`, `enqueue_job` trait
  method widened to `Option<DocumentId>`),
  `crates/infrastructure/src/adapters/sqlite_print_job_repository.rs`
  (`enqueue_in_tx` takes `Option<&str>`; `claim_next_pending` reads
  nullable column), and the four call sites in
  `sqlite_checkout_repository.rs`, `sqlite_conversion_repository.rs`
  (x2), `sqlite_return_repository.rs` updated to pass `Some(...)`.
  Two new automated tests added:
  `migration_0014_makes_print_job_document_id_nullable` and
  `null_document_id_job_enqueues_claims_and_completes`.
- Test suite status at end of SESSION-ARC-006: `cargo test --workspace -j 2`
  → 367 passed / 0 failed / 4 ignored (crash matrix). `cargo clippy
  --workspace --all-targets --all-features -- -D warnings` → exit 0,
  zero warnings. `cargo fmt --check` → exit 0. Frontend gates untouched
  in this session (no `src/**` files modified in SESSION-ARC-006).
- Role/permission enforcement (AUD-018): `crates/domain/src/role.rs` —
  4 roles (Vendeur, Caissier, Gerant, Admin), 34 permissions total.
  `ManageCustomerCredit` (L59, "Credit policy/limit management +
  overpayment approval", Gerant/Admin only). `RecordCreditPayment` (L61,
  "Record customer credit payments", Caissier/Gerant/Admin). Matrix:
  Vendeur 5 perms (neither); Caissier L122-136 (has RecordCreditPayment
  L134, NOT ManageCustomerCredit); Gerant L137-167 (both, L159-160);
  Admin L168-203 (both, L194-195).
- `RecordCreditPaymentUseCase::execute` (`crates/application/src/
  use_cases/customers.rs:315-365`): requires `RecordCreditPayment`
  (L342); rejects amount <= 0 / empty terminal; if
  `allow_overpayment && !has(ManageCustomerCredit)` →
  `OverpaymentUnauthorized` (L349-355). No `amount > balance` check at
  this layer (deferred to repository, defense-in-depth).
- `sqlite_customer_repository.rs:390-521` (`record_credit_payment`):
  single transaction, requires OPEN cash session (ADR-027); reads
  `current_balance_centimes, revision` (L428-438); independent
  overpayment check: `if amount > balance && !allow_overpayment` →
  `PaymentExceedsBalance{balance, attempted}` (L442-446, L514-520);
  inserts `customer_ledger` row (L454-470); optimistic-lock balance
  update with `revision+1` (`CONCURRENCY` error on mismatch, L471-481);
  inserts `cash_movements` row: `movement_type='CREDIT_PAYMENT_IN'`,
  `reference_type='CUSTOMER_PAYMENT'`, `reference_id=<ledger entry_id>`
  (L483-497). Returns `PaymentRecord { ledger_entry_id: String,
  new_balance_centimes: i64 }` (ports.rs:1057-1062, NO serde derives).
  IPC `credit_record_payment` (`ipc.rs:2247-2270`) maps
  `OverpaymentUnauthorized` → `OVERPAYMENT_UNAUTHORIZED`,
  `PaymentExceedsBalance` → `PAYMENT_EXCEEDS_BALANCE`.
- **GAP CONFIRMED AND STILL OPEN AT END OF SESSION-ARC-006: zero
  debt-receipt print wiring exists.** No `justificatif`/`payment_receipt`
  template anywhere in non-test Rust; `record_credit_payment` performs
  no `enqueue_*` call. The only `print_jobs` producers remain
  checkout/conversion/return (always with a real document id).
- Thermal printing pipeline (AUD-019, AUD-020): `crates/infrastructure/
  src/printing/rasterizer.rs` — fixed 576-dot canvas (80mm @ 203dpi,
  `PAPER_WIDTH_DOTS = 576`, L20). NO 58mm path exists. `build_lines`
  (L31-107) is a FIXED pipeline hardcoded for sales receipts: identity
  header → optional DUPLICATA watermark → number+date → rule → per-item
  lines → rule → SIX FIXED TOTALS ROWS (Sous-total HT / TVA / Remise /
  Timbre / TOTAL TTC / Payé, i.e. sales-specific labels) → conditional
  change row → payment rows → rule → footer. Confirmed SAFE with empty
  `lines: []` (loop simply emits nothing, no panic) and safe with
  "zero" total strings (no suppression except change-row). HOWEVER: a
  debt-payment receipt rendered through this exact pipeline would
  incorrectly display sales-only labels ("Sous-total HT", "TVA") next
  to debt amounts — **TASK-BRK-060 must NOT simply reuse this pipeline
  as-is; it needs either a dedicated debt-receipt layout function or a
  clearly justified relabeling, to be decided by the architect before
  implementation.**
  `PrintWorkerService::process_claimed`
  (`crates/application/src/services/print_worker_service.rs:222-335`)
  currently reads only `(job_id, document_id, job_type, attempt_count)`
  from the claimed row and ALWAYS loads a `Document` via `get_by_id`;
  if `document_id == None`, it maps to `Validation(PrintJobNotFound)`
  and the job is marked FAILED (L239-241). **This means: enqueuing a
  NULL-document print job today would claim-then-fail, not print.**
  TASK-BRK-060 must add a branch in `process_claimed` for `None` that
  hydrates and renders from `payload_json` instead of `documents`.
  `payload_json` is currently WRITE-ONLY everywhere (written by all four
  producers, never read by the worker) — it is the natural carrier for
  debt-receipt fields.
  `ReceiptDocument`, `ReceiptLine`, `ReceiptTotals`
  (`crates/domain/src/printing/receipt_model.rs`) ALL derive
  `Serialize, Deserialize, Clone, Debug, PartialEq, Eq` — confirmed
  serializable to/from `payload_json` with no additional work needed on
  the model types themselves.
  `ReceiptRendererPort::render(&self, receipt: &ReceiptDocument) ->
  Result<Vec<u8>, ApplicationError>` (`ports.rs:1241-1257`) is the
  single rendering entry point; its concrete implementor was NOT YET
  located by name/file (TASK-AUD-021, which would have found it, was
  blocked by the session-ending trust incident before being sent to the
  executor — RE-ISSUE THIS AUDIT EARLY IN SESSION-ARC-007).
- Document sequences: `document_sequences(type, fiscal_year,
  last_sequence)` PK (migration 0001:271-276); `allocate_next(doc_type_code:
  &str, fiscal_year: i32) -> Result<i64, ApplicationError>`
  (`ports.rs:894-898`) — free-text `type` key, so a NEW sequence key for
  debt receipts (e.g. `"RECU"` or `"JUSTIFICATIF"`) requires NO schema
  migration, only a new call site using a new string key. `DocumentType`
  (`crates/domain/src/sales/document.rs:16-33`) is a CLOSED enum (8
  variants: Ticket/Facture/Bl/Devis/Avoir/Bc/Br/FF) — debt receipts
  should NOT be forced into this enum; they can use the free-text
  sequence key mechanism directly without touching `DocumentType`.
- Prototype 05 (FR, lines 286-299) already specifies the exact
  user-facing wording for the printed debt receipt mock: "Reçu de
  paiement" / "Client : [name]" / "Montant encaissé : [amount] DZD" /
  "Nouveau solde : [balance] DZD" / "Pièce justificative : (numérotation
  à définir)" / "Une impression de ce reçu a été lancée." — the
  numbering scheme is explicitly left open in the design itself,
  matching the open engine question above.

## 12. Open Design Points / Unknowns (carried, unresolved at end of SESSION-ARC-006)

1. `QUESTIONNAIRE-COMPTABLE-FR.html` (`docs/legal/`) was verified in
   TASK-AUD-017 to be an EMPTY draft template (341 lines, 77 blank
   answer boxes, "Version 2 - révisée le 2026-10-02 - document de
   travail, non encore transmis"). Owner delivery of the completed
   questionnaire or a `QUESTIONNAIRE-COMPTABLE-FR-v2.pdf` filled by the
   accountant remains PENDING. **ABSOLUTE FIRST PRIORITY**: the instant
   this arrives, review it before anything else, interrupting any task
   in progress.
2. ADR-080 (English-language persisted CTP files) directly contradicts
   CHARTER-v2.0 Section 15.1's explicit requirement for Arabic CTPs.
   THIS MUST BE RESOLVED EXPLICITLY WITH THE OWNER EARLY IN
   SESSION-ARC-007 (see Section 5 above) — do not silently continue
   either practice without a fresh, explicit ruling.
3. Accountant answers still needed (from the eventual filled
   questionnaire): proof/receipt numbering scheme, remaining-balance
   presentation convention, timbre fiscal applicability on a debt
   receipt (prototype's own worked example row states "Encaissement
   d'une dette: Aucun timbre n'est calculé aujourd'hui" — UNVERIFIED by
   a professional), timbre base calculation, A4 invoice mandatory
   mentions, document numbering and retention period requirements.
4. Cashier's entry point/screen location for approved debt collection
   (inside Caisse & Clôture module vs. a customer picker inside the
   Vente/POS module) — UNDECIDED, needs an architect decision or owner
   preference before the UI screen (not just the engine) can be
   specified.
5. Arabic glossary: 112 terms total in `docs/design/glossary-ar-fr.md`
   (status: "v0.1 PROPOSED", owner review pending). 5 highest-priority
   terms were extracted and presented to the owner in SESSION-ARC-006
   Reply 9 — re-present these in SESSION-ARC-007 if no answer was given:
   (a) "الزبائن والديون" vs "الزبائن والكريدي" for Clients & Crédit;
   (b) "المسؤول" (Administrateur) vs "المسيّر" (Gérant) — confirm no
   confusion between the two Arabic terms in daily shop use;
   (c) "وصل إرجاع" vs the formal "إشعار دائن" for Avoir;
   (d) "كريدي" vs "بالآجل" for the À crédit payment-method label;
   (e) "وصل تسديد" / "وصل دفع" phrasing for Preuve d'encaissement
       (debt payment receipt).
6. Debt-receipt sequence key naming: `"RECU"` vs `"JUSTIFICATIF"` vs
   reusing `customer_ledger`'s own running sequence/entry id — decision
   needed before TASK-BRK-060 is written (see Section 11).
7. Concrete implementor struct/file of `ReceiptRendererPort` was not
   located before the session ended (TASK-AUD-021 was prepared but never
   sent to the executor — see Section 13). Locate it first thing in
   SESSION-ARC-007 before writing TASK-BRK-060.
8. Whether prototype 05's wording correction (DSN-021-C1, purely textual,
   confirming an already-approved behavior) requires a fresh, formal
   owner re-acceptance pass, or whether the original SESSION-ARC-005
   visual acceptance of prototype 05 still stands as-is — unresolved,
   low priority, raise opportunistically.

## 13. Roadmap Status (at end of SESSION-ARC-006)

DONE IN SESSION-ARC-005 (see `CTP-SESSION-ARC-005.md` for full detail):
- DSN-011..020 series (10 static prototypes, FR+AR, committed at `2c83002`).
- Overpayment-blocking business rule ratified by the owner (Reply 15).
- BRK-058 engine command/migration recount; AUD-015 boot-tool danger
  discovery.
- REL-018 DSN series commit; DOC-019 CTP-005 in-place update; DOC-020
  CTP-006 seed authored for handoff.

DONE IN SESSION-ARC-006 (Replies 1-17):
- TASK-AUD-016 (Reply 1-2): Handoff verification, boot-package `--check`
  confirmation, located exact superseded "PROPOSÉ" wording lines in
  prototype 05 FR/AR, discovered `docs/legal/QUESTIONNAIRE-COMPTABLE-FR.html`.
- TASK-DSN-021 (Reply 2): BLOCKED by executor — architect's target
  strings did not match disk content (correctly rejected, no guessing).
- TASK-DSN-021-C1 (Reply 3): Ratified overpayment-ban wording applied
  verbatim to prototypes 05 FR/AR using disk-exact targets.
- TASK-REL-019 (Reply 4): Committed ratified prototypes 05 at `3e7897e`.
- TASK-AUD-017 (Reply 5): Reconnaissance of `print_jobs` schema, Rust
  mapping, migration runner mechanics, and questionnaire HTML status
  (confirmed empty draft).
- TASK-BRK-059 (Reply 6): Authored and implemented Migration 0014
  (nullable `print_jobs.document_id`), adapted domain/application/
  infrastructure layers, added 2 new tests, 367 total tests green,
  clippy/fmt clean.
- TASK-REL-020 (Reply 7): Committed Migration 0014 and engine changes at
  `772204a`.
- TASK-AUD-018 (Reply 8): Reconnaissance of credit payment use case,
  permission matrix, IPC handler, customer/ledger repository — CONFIRMED
  overpayment-blocking already enforced at two independent engine layers;
  CONFIRMED zero debt-receipt print wiring exists (gap for BRK-060).
- TASK-AUD-019 (Reply 9, with 5 glossary terms presented to owner in the
  same reply's Arabic section): Reconnaissance of thermal print worker
  dispatch, rasterizer pipeline, document sequence numbering, cash
  movement linkage.
- TASK-DOC-021 (Reply 10): Reply-10 CTP checkpoint — **EXECUTED
  INCORRECTLY (summarized/truncated the CTP file)**; committed at
  `dfe3b85`. See Section 10 incident log — this is the central failure
  of SESSION-ARC-006.
- TASK-REL-021 (Reply 11): Regenerated and committed BOOT-006 +
  (truncated) CTP-006 at `dfe3b85` — later understood to have propagated
  the Reply-10 defect into the committed boot package as well.
- TASK-AUD-020 (Reply 12 task; Reply 14 is where it was actually issued
  to the executor due to the Reply-13 correction interrupting the
  sequence) / (Reply 14-15 report): Detailed reconnaissance of
  `ReceiptDocument`/`ReceiptLine`/`ReceiptTotals` serde derives,
  `DocumentSequenceRepository`/`ReceiptRendererPort` trait signatures,
  closed `DocumentType` enum, rasterizer safety with empty
  lines/zero totals, `PaymentRecord` return struct, and prototype 05's
  own printed-receipt mock wording.
- TASK-DOC-022 (Reply 13-14): **CORRECTIVE TASK** — restored
  `CTP-SESSION-ARC-006.md` to full, unabridged, additively-enriched
  content (296 lines, 17105 bytes); regenerated and verified
  `BOOT-SESSION-ARC-006.md` (70869 bytes); committed at `dfeb743`.
- TASK-AUD-021 (Reply 15): drafted by the architect (targeted code
  inspection of the customer-repository transaction block, the print
  worker's `process_claimed`, the `ReceiptRendererPort` implementor
  location, and the rasterizer's line-builder) — **NEVER ACTUALLY SENT
  to the executor**, because the owner's Reply-16 alarm interrupted the
  sequence before the task block was dispatched. RE-ISSUE THIS AUDIT
  FIRST in SESSION-ARC-007; its findings are still needed before
  TASK-BRK-060 can be written safely.
- TASK-AUD-022 (Reply 16): Ground-truth, read-only inventory of
  `docs/architecture/` confirming CTP-007/BOOT-007 absence was CORRECT
  and EXPECTED behavior per the charter's own lifecycle model (not a
  new defect) — but by this point the owner had already decided to end
  the session regardless, given the Reply-10 trust breach.
- TASK-DOC-023 (Reply 17, THIS TASK): Final full, unabridged session
  handoff — authoring this very file (`CTP-SESSION-ARC-007.md`) for the
  first and only time from SESSION-ARC-006, plus regenerating
  `BOOT-SESSION-ARC-007.md`.

NEXT (first priorities for SESSION-ARC-007, in suggested order):
(a) Re-issue TASK-AUD-021 (customer-repository transaction block,
    `process_claimed` worker code, `ReceiptRendererPort` implementor
    location+file, rasterizer line-builder) — read-only, exactly as
    drafted in SESSION-ARC-006 Reply 15 (available in that session's
    reply log if the owner wishes to retrieve it; otherwise redraft
    fresh, it is a small audit).
(b) Resolve the ADR-080 vs Charter-15.1 language contradiction explicitly
    with the owner (Section 5, Section 12 item 2).
(c) Re-present the 5 glossary terms if still unanswered (Section 12 item 5).
(d) Decide the debt-receipt layout approach (dedicated layout vs.
    relabeled reuse of the sales rasterizer pipeline) and the sequence
    key naming (Section 12 item 6), then write TASK-BRK-060: implement
    debt receipt generation, sequence allocation, `PrintWorkerService`
    payload-based dispatch for non-sale print jobs, and the atomic
    transaction wiring in `record_credit_payment`.
(e) TASK-REL-022: commit TASK-BRK-060's engine deliverables.
(f) Resume substantive roadmap work (Section 11 of CHARTER-v2.0,
    Phase 2/3 items) per owner priority, subject to immediate
    interruption if the accountant questionnaire PDF arrives
    (Section 12 item 1 — this interrupt priority is ABSOLUTE and
    overrides everything else, including CTP checkpoint timing).

AFTER: continue per CHARTER-v2.0 Section 12 roadmap skeleton, Phase 1
foundation-of-experience items not yet started (design tokens
finalization, component inventory, shell implementation) once the above
clears.

## 14. Reply Counter / CTP Checkpoint Rule

SESSION-ARC-007's reply counter restarts at 1. A full CTP checkpoint —
meaning: update THIS file (`CTP-SESSION-ARC-007.md`) in place, STRICTLY
ADDITIVELY, NEVER summarizing or dropping any existing content — is
mandatory at reply 10, 20, 30... of SESSION-ARC-007's own counter. Given
the severity of the Reply-10 incident documented in Section 10, the
architect of SESSION-ARC-007 should treat the FIRST such checkpoint
(Reply 10) with maximum procedural care: re-read Section 7 and Section 10
of this very file immediately before writing that checkpoint update.

## 15. Human Context — Owner Working Style and Trust State (NEW, mandatory going forward)

The owner is detail-oriented, verifies claims against the actual
repository/file state rather than taking summaries on faith, and has
twice now (SESSION-ARC-005's CTP-path incident, SESSION-ARC-006's
Reply-10 truncation incident) personally caught process failures that
the architect either missed or only self-corrected after the owner's
intervention. The owner explicitly ended SESSION-ARC-006 early (at
Reply 17 rather than continuing indefinitely) specifically because of
the Reply-10 incident, stating a wish to continue in a new conversation
"with a fresh spirit, hard work, and high focus." This is a direct,
honest signal that procedural discipline — especially around the CTP
checkpoint ceremony — is the single highest-trust-impact behavior in
this entire engagement, outweighing even engine/design throughput. The
architect of SESSION-ARC-007 should open the session by explicitly
acknowledging this history (briefly, without dwelling on it unduly) and
should treat every CTP checkpoint as the single most scrutinized action
in the entire session.

## 16. Boot Instructions for SESSION-ARC-007

This file, together with CHARTER-v2.0 (Part 1), is assembled into
`docs/architecture/BOOT-SESSION-ARC-007.md` via
`docs/architecture/tools/build-boot-package.mjs` by SESSION-ARC-006 as
the final act of TASK-DOC-023. If you are reading this as the newly
started SESSION-ARC-007 architect:
1. Verify via `git log` that this file and `BOOT-SESSION-ARC-007.md`
   are committed, and that the commit message matches
   "docs(architecture): session handoff ARC-006 to ARC-007 (DOC-023)"
   (or whatever exact message the commit task actually used — check the
   real git log, do not assume).
2. Immediately check whether the owner has delivered the completed
   accounting questionnaire (Section 12, item 1) before anything else.
3. Your first substantive task should be TASK-AUD-021 (re-issued, see
   Section 13 "NEXT" item (a)) to unblock TASK-BRK-060 safely.
4. Read Section 10 and Section 7 of this file before your own Reply-10
   checkpoint arrives.
