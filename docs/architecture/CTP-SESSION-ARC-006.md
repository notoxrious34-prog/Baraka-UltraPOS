# CONTEXT TRANSFER PACKAGE — SESSION ARC-006

Status: ACTIVE (Checkpoint at Reply 10 / Restored & Enriched at Reply 13)
Date updated: 2026-10-06
Authoring session: SESSION-ARC-006 (reply 13, TASK-DOC-022)
Supersedes: the truncated CTP-SESSION-ARC-006.md draft from TASK-DOC-021 and seed CTP from ARC-005.

## 1. Role & Scope

You are the Master Principal Architect & Design Lead of Baraka UltraPOS,
acting under CHARTER-v2.0 (binding, full text accompanies this CTP as
Part 1 of BOOT-SESSION-ARC-006.md). Baraka UltraPOS is a Tauri v2 desktop
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

## 2. CTP Lifecycle Model (CORRECTED, binding)

Each session owns exactly one CTP file at the flat path
`docs/architecture/CTP-SESSION-ARC-00N.md` — no subfolder, no per-reply
suffix. The CURRENT session updates its OWN file in place at each
checkpoint (reply 10, 20, 30...) and once more at session end. The file
you are reading was authored ONCE by the outgoing session (ARC-005) at
the moment the owner decided to end ARC-005 and start ARC-006. As
SESSION-ARC-006 you will own and update `CTP-SESSION-ARC-006.md` in place
at your own checkpoints, and author `CTP-SESSION-ARC-007.md` exactly
once, at your own handoff. Do NOT invent new filenames, subfolders, or
per-reply suffixes for CTP files — this was a real, owner-caught incident
in SESSION-ARC-005 (see Section 10).

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
SESSION-ARC-006" (counter restarts at 1 for this session).

## 5. Owner Business Decisions (status)

- Overpayment blocking: RESOLVED & RATIFIED (SESSION-ARC-005, reply 15;
  committed in prototypes 05 at commit `3e7897e` via TASK-REL-019).
  A Caissier may NEVER collect a debt payment exceeding the customer’s
  remaining balance; any amount above the debt strictly requires
  ManageCustomerCredit (Gerant/Admin only).
- ADR-080 (PROPOSED, SESSION-ARC-005): persisted CTP file content stays
  in English, matching CTP-001..005 practice; Arabic is reserved for the
  owner-facing in-reply summary only. NOT YET explicitly ratified by the
  owner as of SESSION-ARC-005 end; no objection received either. Treat as
  provisionally binding; seek explicit ratification opportunistically
  without derailing priority work.
- Algerian Trade Glossary: 5 prioritized terms presented to the owner in
  Reply 9 (Clients & Crédit vs الكريدي; Administrateur/Gérant; Avoir vs
  وصل إرجاع; À crédit vs كريدي; Preuve d’encaissement vs وصل تسديد).
  Awaiting owner guidance.

## 6. Hard Technical Rules (binding, unchanged)

ADR-067 (no `position: fixed` except modal overlays/toasts/command
palette); ADR-069 (no floats for money, Latin-digit formatted strings
only, wrapped in `<bdi>`); ADR-074 (destructive/irreversible actions
require typed or long-press confirmation); ADR-077; ADR-079 (PIN values
never rendered as real digits, masked bullets only); ADR-080 (proposed,
see Section 5). No JavaScript in static prototypes; no external
resources/network calls; all CSS inline; all icons inline `<svg>`. Files
written via Node.js `fs.writeFileSync` with explicit UTF-8 buffer, LF
line endings only, no BOM, lines joined with `\n`, exactly one trailing
newline; never PowerShell Set-Content. `wc -l` semantics are authoritative
for line-count evidence; any off-by-one reconciled via the
trailing-newline fact. NO_LEFTOVERS: all ad-hoc scratch
scripts/files must be deleted before task completion, proven via
directory listing / git status. Screenshot capture must be
headless/background only. Never touch the live DB,
`%APPDATA%\com.baraka.ultrapos`, `%LOCALAPPDATA%\com.baraka.ultrapos`.
Work only in `D:/Dev/Baraka UltraPOS`. Never edit `.gitattributes` or
migrations 0001-0013. No new dependencies without explicit per-task
authorization.

## 7. Process Rules (binding, hard-learned this session)

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

## 8. Repo Chain (as of SESSION-ARC-006 Reply 13)

- `dfe3b85` — docs(architecture): update CTP-006 and boot package for reply 10 checkpoint
- `772204a` — feat(engine): make print_jobs.document_id nullable in migration 0014 (BRK-059)
- `3e7897e` — docs(design): ratify cashier debt overpayment ban in prototypes 05 (DSN-021-C1)
- `e02db2e` — docs(architecture): session handoff ARC-005 to ARC-006 (DOC-019, DOC-020, BOOT-021)
- `2c83002` — docs(design): DSN-012..020 static prototypes FR+AR, series complete
- `682668b` — docs(architecture): REL-013 CTP and boot package for SESSION-ARC-005
- `6a71fcf` — docs(design): DSN-011 static prototype 1/5 - fast product entry FR

## 9. Accepted Design Deliverables (all FINAL, git-committed at 2c83002 and updated at 3e7897e)

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
5. `05-encaissement-dette.fr.html` (DSN-015 baseline sha256 84fea228... updated via
   DSN-021-C1, committed at `3e7897e`) — sha256
   ca78fa7a6aa50d876ca1587a563859bd2f60bbcdd6c7af08c40b1e4ae9046ff6, 9955
   bytes, 304 lines.

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
5. `05-encaissement-dette.ar.html` (DSN-020 baseline sha256 d71f6ed1... updated via
   DSN-021-C1, committed at `3e7897e`) — sha256
   4341a6977eb5a1b32d87e2dbb605b087eb26e7cbcea47c9b15d91f45e92f1dad,
   11782 bytes, 309 lines.

## 10. Errors & Dead Ends Log (carried & cumulative, do not repeat)

- CTP-persistence incident (SESSION-ARC-005 replies 15-18): the architect
  invented a wrong path/filename
  (`docs/architecture/ctp/SESSION-ARC-005-CTP-R15.md`) for a CTP
  checkpoint; the owner caught it via a direct screenshot of the real
  flat `docs/architecture/` structure. Root cause and the correct model
  are in Section 2 above.
- Boot-tool danger (TASK-AUD-015): confirmed, from actual source code
  (not memory), unconditional-overwrite behavior on any unrecognized
  flag. Golden rule is in Section 3 above.
- Circular-verification error (TASK-DOC-016 first attempt): rejected by
  the architect before owner review; corrected via TASK-DOC-016-C1 with
  an independently re-sourced reference plus raw diff. Standard is in
  Section 7 above.
- Extraction-diff reconciliation (TASK-BOOT-017): an apparent one-byte
  (LF) discrepancy per region was fully explained by marker-slicing
  mechanics (the slice between BEGIN...\n and \n...END structurally drops
  the region's trailing LF) — not a real content defect. If an identical
  pattern recurs, reconcile explicitly the same way; do not treat it as
  an automatic failure.
- Spec Target String Discrepancy (TASK-DSN-021 in ARC-006): the architect
  specified target replacement strings from conceptual memory rather than
  auditing literal disk content. The executor correctly blocked execution.
  Reconciled via TASK-DSN-021-C1 using exact disk lines audited in TASK-AUD-016.
  Lesson: never specify text replacements without a fresh read-only audit of
  verbatim lines.
- CTP Checkpoint Truncation Incident (TASK-DOC-021 in ARC-006): in reply 10,
  the architect summarized and truncated CTP-006, dropping historical hashes,
  error logs, and open items. Caught immediately by the owner. Reconciled via
  TASK-DOC-022 to restore full cumulative preservation. Lesson: CTP checkpoints
  must NEVER summarize or drop previous content; all updates are strictly additive.

## 11. Key Engine/Domain Facts (verified in ARC-006 through TASK-BRK-059 and AUD-017..019)

- 14 migrations (0001-0014); 72 IPC commands (marker file `crates/commands/src/ipc.rs`).
- `print_jobs.document_id` made nullable in SQLite via Migration 0014
  (`0014_make_print_jobs_document_id_nullable.sql`, sha256
  `8475f84f1a30693c0404ba5ae7f6c7c3525283ad33c43eac83d11af7f792ec03`, 1042 B).
  Adapted domain `PrintJob.document_id: Option<String>`, port `PrintJobRecord.document_id: Option<DocumentId>`,
  and infra repositories.
- Role/permission enforcement (AUD-018): `crates/domain/src/role.rs` — 4 roles, 34 permissions.
  `ManageCustomerCredit` (L59, Gerant/Admin only), `RecordCreditPayment` (L61, Caissier/Gerant/Admin).
  `RecordCreditPaymentUseCase` (customers.rs:315-365) requires `RecordCreditPayment`;
  rejects overpayment without `ManageCustomerCredit` (`OverpaymentUnauthorized`).
  `sqlite_customer_repository.rs` (L442-446) enforces defense-in-depth: if amount > balance
  and not allow_overpayment, aborts with `PaymentExceedsBalance`.
- Thermal printing pipeline (AUD-019): `crates/infrastructure/src/printing/rasterizer.rs` has fixed
  576 dots canvas (80mm @ 203dpi, `PAPER_WIDTH_DOTS = 576`). No 58mm path exists.
  `PrintWorkerService` (`crates/application/src/services/print_worker_service.rs`) currently
  reads only `(job_id, document_id, job_type, attempt_count)`; if `document_id == None`,
  it currently marks job failed. Non-sale print jobs require `payload_json` dispatch.
- Document sequences: `document_sequences` table handles TICKET, FACTURE, BL, DEVIS, AVOIR, BC, BR, FF.
  No sequence currently exists for debt receipts (`JUSTIFICATIF` / `RECU`).
- Test state: `cargo test --workspace -j 2` → 367 passed / 0 failed / 4 ignored (includes
  2 new automated tests for migration 0014 and nullable print jobs). Clippy clean (-D warnings exit 0).
  Fmt clean.

## 12. Open Design Points / Unknowns (carried, unresolved)

1. `QUESTIONNAIRE-COMPTABLE-FR.html` was verified in TASK-AUD-017 to be an empty draft
   template (341 lines, 77 blank answer boxes, version 2 dated 2026-10-02).
   Owner delivery of completed questionnaire or `QUESTIONNAIRE-COMPTABLE-FR-v2.pdf` remains
   pending. It MUST be reviewed the instant it arrives.
2. ADR-080 ratification — no explicit owner confirmation yet (see Section 5).
3. Accountant answers still needed: proof numbering, remaining-balance presentation,
   timbre on the proof, timbre base, A4 invoice mentions, document numbering, retention period.
4. Cashier entry point for approved debt collection (Caisse & Clôture vs. customer picker
   inside Vente) — undecided.
5. Arabic glossary terms need owner review: 112 terms in `docs/design/glossary-ar-fr.md`;
   5 prioritized terms presented to the owner in Reply 9 (Clients & Crédit vs الكريدي;
   Administrateur/Gérant; Avoir vs وصل إرجاع; À crédit vs كريدي; Preuve d’encaissement vs وصل تسديد).
6. Receipt sequence key: decision needed on sequence prefix (`JUSTIFICATIF` vs `RECU` vs `REC`).

## 13. Roadmap Status

DONE IN ARC-005:
- DSN-011..020 series (FR+AR, 10 prototypes, committed at `2c83002`).
- BRK-058 engine recon; AUD-015 boot-tool recon.
- REL-018 DSN series commit; DOC-019 CTP-005 update; DOC-020 CTP-006 seed.

DONE IN ARC-006:
- TASK-AUD-016: Handoff verification and boot check.
- TASK-DSN-021-C1: Ratify overpayment ban in prototypes 05 (FR & AR).
- TASK-REL-019: Commit ratified prototype 05 files (`3e7897e`).
- TASK-AUD-017: Reconnaissance for Migration 0014 and print schema.
- TASK-BRK-059: Migration 0014 (nullable print_jobs.document_id), adapted domain/app/infra, 2 new tests.
- TASK-REL-020: Commit Migration 0014 and print pipeline changes (`772204a`).
- TASK-AUD-018: Credit payment permission and logic reconnaissance.
- TASK-AUD-019: Thermal printing and worker dispatch reconnaissance.
- TASK-REL-021: Initial reply 10 checkpoint commit (`dfe3b85`).
- TASK-DOC-022: Restore full unabridged CTP-006 and regenerate boot package (this task).

IN PROGRESS:
- Rectification of immortal project memory and CTP synchronization.

NEXT:
- TASK-AUD-020: Detailed Thermal Receipt Model, Sequence Generation, and Use-Case Return Audit
  (unblocked once memory is secured).
- TASK-BRK-060: Implement debt receipt generation, sequence allocation, and `PrintWorkerService`
  payload dispatch for non-sale print jobs.
- TASK-REL-022: Commit BRK-060 engine deliverables.

## 14. Reply Counter / CTP Checkpoint Rule

A full CTP checkpoint — updating THIS file in place — is mandatory at reply 10, 20, 30...
of SESSION-ARC-006's own counter. The checkpoint at reply 10 is hereby fully restored
and enriched. Next checkpoint: Reply 20.

## 15. Section reserved (intentionally left for future protocol notes)

## 16. Instructions to Start/Continue Session ARC-006

This file, together with CHARTER-v2.0 (Part 1), is assembled into
`docs/architecture/BOOT-SESSION-ARC-006.md` via
`docs/architecture/tools/build-boot-package.mjs`.
SESSION-ARC-006 is active. The engine is at commit `772204a` with 14 migrations and 367 tests
passing. All 10 prototypes are committed and ratified.
Proceed to TASK-AUD-020 and TASK-BRK-060.
Execution and Boot Tool Verification:
a. Write docs/architecture/CTP-SESSION-ARC-006.md using Node.js script.
b. Clean up script immediately (NO_LEFTOVERS).
c. Run pre-check dry run:
node docs/architecture/tools/build-boot-package.mjs --charter docs/architecture/CHARTER-v2.0.md --ctp docs/architecture/CTP-SESSION-ARC-006.md --out docs/architecture/BOOT-SESSION-ARC-006.md --check
d. Run real boot package build:
node docs/architecture/tools/build-boot-package.mjs --charter docs/architecture/CHARTER-v2.0.md --ctp docs/architecture/CTP-SESSION-ARC-006.md --out docs/architecture/BOOT-SESSION-ARC-006.md
e. Run post-check confirmation:
node docs/architecture/tools/build-boot-package.mjs --charter docs/architecture/CHARTER-v2.0.md --ctp docs/architecture/CTP-SESSION-ARC-006.md --out docs/architecture/BOOT-SESSION-ARC-006.md --check
(Must exit 0 with CHECK: OK).
f. Stage and commit:
git add docs/architecture/CTP-SESSION-ARC-006.md docs/architecture/BOOT-SESSION-ARC-006.md
git commit -m "docs(architecture): restore full unabridged CTP-006 and regenerate boot package"
g. Verify clean status:
git status --porcelain (must be completely empty).
