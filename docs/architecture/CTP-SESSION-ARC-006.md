# CONTEXT TRANSFER PACKAGE — SESSION ARC-006

Status: SEED (authored by outgoing session ARC-005 at handoff)
Date authored: 2026-10-05
Authoring session: SESSION-ARC-005 (reply 23, TASK-DOC-020)
Supersedes: the CTP-SESSION-ARC-006.md draft produced via TASK-DOC-016/-C1
(superseded in full by this rewrite, which reflects the final state of
CTP-SESSION-ARC-005.md after TASK-DOC-019 and the committed state after
TASK-REL-018).

## 1. Role & Scope

You are the Master Principal Architect & Design Lead of Baraka UltraPOS,
acting under CHARTER-v2.0 (binding, full text accompanies this CTP as
Part 1 of BOOT-SESSION-ARC-006.md). Baraka UltraPOS is a Tauri v2 desktop
POS for an Algerian hardware store. The Rust/SQLite engine is FROZEN as of
TASK-BRK-057. There are 13 migrations (0001-0013); any new migration
starts at 0014. There are 72 IPC commands, re-verified via TASK-BRK-058.
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

- Overpayment blocking: RESOLVED (SESSION-ARC-005, reply 15). A Caissier
  may NEVER collect a debt payment exceeding the customer's remaining
  balance; any amount above the debt strictly requires
  ManageCustomerCredit (Gerant/Admin only). DSN-015 (FR) and DSN-020 (AR)
  still contain superseded "v0.1 PROPOSÉ, en attente de confirmation du
  propriétaire" wording — a wording-only correction task is QUEUED, NOT
  YET EXECUTED. This is the FIRST roadmap item once the handoff thread
  closes.
- ADR-080 (PROPOSED, SESSION-ARC-005): persisted CTP file content stays
  in English, matching CTP-001..005 practice; Arabic is reserved for the
  owner-facing in-reply summary only. NOT YET explicitly ratified by the
  owner as of SESSION-ARC-005 end; no objection received either. Treat as
  provisionally binding; seek explicit ratification opportunistically
  without derailing priority work.

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

## 8. Repo Chain (as of SESSION-ARC-005 end)

- `2c83002` — docs(design): DSN-012..020 static prototypes FR+AR, series
  complete (one-time declared batch-commit deviation; message lists all
  task IDs including -C1 rounds)
- `682668b` — docs(architecture): REL-013 CTP and boot package for
  SESSION-ARC-005
- `6a71fcf` — docs(design): DSN-011 static prototype 1/5 - fast product
  entry FR

NOTE: as of SESSION-ARC-005 reply 23, `CTP-SESSION-ARC-005.md` (updated
via TASK-DOC-019), `CTP-SESSION-ARC-006.md` (this file, rewritten via
TASK-DOC-020), and `BOOT-SESSION-ARC-006.md` (to be regenerated one more
time) are UNCOMMITTED. A single commit covering all three, with an
explicit commit message, is mandatory before this handoff thread closes
(see Section 16).

## 9. Accepted Design Deliverables (all FINAL, git-committed at 2c83002)

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
5. `05-encaissement-dette.fr.html` (DSN-015) — sha256
   84fea228b5a47fce41041f0282162c287d8b8c09ef93ce042ca437c3535c2f73, 9802
   bytes, 304 lines. [Contains superseded "PROPOSÉ" wording — correction
   queued.]

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
5. `05-encaissement-dette.ar.html` (DSN-020) — sha256
   d71f6ed11b2522a1e7eef75f35120da66eed034c313d9040794fbe1875a51003,
   11663 bytes, 309 lines. [Also contains superseded "PROPOSÉ" wording —
   correction queued.]

## 10. Errors & Dead Ends Log (carried, do not repeat)

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

## 11. Key Engine/Domain Facts (verified TASK-BRK-058; re-verify before
long-term reliance)

- 13 migrations (0001-0013); 72 IPC commands (sole marker file
  `crates/commands/src/ipc.rs`).
- `print_jobs.document_id NOT NULL` confirmed at migration 0001, lines
  279-288 (needs a new migration 0014 per TASK-BRK-059, queued).
- Role/permission matrix: `crates/domain/src/role.rs` — `enum Role`
  ~L17; `enum Permission` including `ManageCustomerCredit` L59,
  `RecordCreditPayment` L61; `permissions()` matrix L109-160.
- Test state at BRK-058: `cargo test --workspace -j 2` → 46 suites ok,
  365 passed / 0 failed / 4 ignored; `npx vitest run` → 22 files / 183
  tests passed; clippy clean, 0 warnings; typecheck/lint/build all clean.

## 12. Open Design Points / Unknowns (carried, unresolved)

1. `QUESTIONNAIRE-COMPTABLE-FR-v2.pdf` has still NOT been delivered by
   the owner despite repeated requests. It MUST be reviewed before being
   sent to the accountant. Resume this review as the ABSOLUTE FIRST
   PRIORITY the instant it is delivered, interrupting any task in
   progress, even across the ARC-005→ARC-006 session boundary.
2. ADR-080 ratification — no explicit owner confirmation yet (see
   Section 5).
3. Accountant answers still needed: proof numbering, remaining-balance
   presentation, timbre on the proof, timbre base, A4 invoice mentions,
   document numbering, retention period.
4. Cashier entry point for approved debt collection (Caisse & Clôture vs.
   customer picker inside Vente) — undecided.
5. Arabic glossary terms need owner review (112 terms; flagged rows 3,
   19, 24, 35, 57, 58; Administrateur/Gérant similarity note rows 11-12)
   — review material fully prepared, not yet presented to the owner.

## 13. Roadmap Status

DONE: DSN-011..020 series (FR+AR, 10 prototypes, committed at 2c83002);
BRK-058 engine recon; AUD-015 boot-tool recon; DOC-016/-C1 first CTP-006
draft (now superseded by this file); BOOT-017 first BOOT-006 draft (to be
regenerated); REL-018 DSN series commit; DOC-019 CTP-005 in-place update;
DOC-020 (this file) CTP-006 rewrite.

IN PROGRESS: session-handoff thread — this CTP-006 rewrite is step 1 of 3
remaining steps (see Section 16).

NEXT (once the handoff thread closes): (a) DSN-015/DSN-020 wording
correction "proposé" → "confirmé"; (b) TASK-BRK-059 — fix
`print_jobs.document_id NOT NULL`, new migration 0014; (c) present the
glossary review package to the owner (112 terms, all 10 prototype files
with †/‡ annotations fully ready); (d) Caissier/credit permission-split
task (ManageCustomerCredit / RecordCreditPayment split — no longer
blocked, overpayment question resolved).

AFTER: resume substantive BRK/DSN work per owner priority once the above
clears, subject to immediate interruption if the accountant questionnaire
PDF arrives (see Section 12, item 1).

## 14. Reply Counter / CTP Checkpoint Rule

A full CTP checkpoint — meaning: update THIS file in place — is mandatory
at reply 10, 20, 30... of SESSION-ARC-006's own restarted-at-1 counter.
Never postpone it. Lesson carried from ARC-005: persist to the correct
flat path immediately when due; do not let several replies pass before
reconciling a missed checkpoint.

## 15. Section reserved (intentionally left for future protocol notes)

## 16. Instructions to Start/Continue Session ARC-006

This file, together with CHARTER-v2.0 (Part 1), is assembled into
`docs/architecture/BOOT-SESSION-ARC-006.md` via
`docs/architecture/tools/build-boot-package.mjs`. Before SESSION-ARC-006
is told to begin substantive work, SESSION-ARC-005 must complete, in
order:
  (a) [TASK-DOC-020, this task] rewrite this file to its final state —
      complete once this task's report is accepted;
  (b) regenerate `BOOT-SESSION-ARC-006.md` one more time, using the same
      three-step safe protocol as TASK-BOOT-017 (`--check` dry run before
      write, real write, `--check` confirm after write, then an
      independent extraction-and-diff against both the CHARTER source and
      this CTP file);
  (c) commit `CTP-SESSION-ARC-005.md` + `CTP-SESSION-ARC-006.md` +
      `BOOT-SESSION-ARC-006.md` together in one task with an explicit
      commit message;
  (d) only then present the final `BOOT-SESSION-ARC-006.md` content to
      the owner as ready to start SESSION-ARC-006.
If you are reading this as the newly started ARC-006 architect, all of
the above should already be complete and git-committed — verify via
`git log` before proceeding, and immediately check whether the owner has
delivered `QUESTIONNAIRE-COMPTABLE-FR-v2.pdf` (Section 12, item 1) before
anything else.
