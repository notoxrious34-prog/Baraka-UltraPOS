# CTP-SESSION-ARC-005 — Context Transfer Package
Date: 2026-10-05 (updated at Reply 22; originally authored by session ARC-004
at session start, now updated in place per the session's own checkpoint
discipline)

## 1. Role, product, scope
Role: Master Principal Architect and Design Lead of Baraka UltraPOS, Charter
v2.0. Product: Tauri v2 desktop POS, hardware store in Algeria, frozen
Rust/SQLite engine at TASK-BRK-057, 13 migrations (0001-0013, new ones start
at 0014), 72 IPC commands — all three RE-VERIFIED with zero discrepancy by
TASK-BRK-058 (read-only recon) this session. Scope: one shell, design system,
ten constitutional modules, FTS search, printing 58/80mm thermal and A4,
import/export, release. Open v1.0.0 gates unchanged: thermal verification
58/80mm; tax/legal validation; DB encryption decision; unmeasured items
(shop-PC spec, scanner wedge, Ctrl+digit conflicts, code-splitting budget,
dark-theme accessibility). Never claim "100%" or "production ready" while a
gate or measurement is open.

## 2. Protocol
Reply header "الرد رقم N — SESSION-ARC-005"; numbering restarted at 1 for
this session, currently at Reply 22. Two sections per reply (Arabic owner
report / English executor task), exactly one task per reply. CTP due every
10th reply (10, 20, 30...); produced at Reply 10 (embedded inline only — a
gap, see Section 12) and now properly updated in place at Reply 20-22.
LESSON, now binding: a CTP checkpoint means updating THIS SAME FILE in
place, never inventing a new filename/subfolder.

## 3. Owner and communication rules
Unchanged: owner is non-technical, the sacred bridge; never ask him to edit
or merge technical text; owner-facing text is Modern Standard Arabic only;
executor-facing text is precise English; warn before any task moving the
mouse or opening windows; prefer fresh from-disk recomputation over any
manual tally; executor reports must paste raw/literal evidence, never
paraphrase — extended "doubly" into the BRK/AUD/DOC/BOOT/REL phases this
session, zero exceptions tolerated.

## 4. Owner business decisions (carried over, plus NEW this session)
Carried: ADR-068 IFU flat-rate tax; ADR-069 Latin digits always; light theme
default; app opens maximized (BRK-060); thermal 58/80mm plus A4; no Dolisoft
transfer; ADR-077 generic import/export; product name FR-typed with optional
AR alias, duplicate-name rejection; logo location known, never touched; real
DB never touched.
NEW (Reply 15, session ARC-005): the overpayment question is RESOLVED. A
Caissier may NEVER collect a debt payment exceeding the customer's remaining
balance; any amount above the debt strictly requires ManageCustomerCredit
(Gerant/Admin only). Prototypes docs/design/prototypes/05-encaissement-dette.fr.html
(DSN-015) and .ar.html (DSN-020) STILL CONTAIN the superseded "v0.1 PROPOSÉ,
en attente de confirmation du propriétaire" wording as of Reply 22 — a short
wording-only follow-up task ("proposé" -> "confirmé") is QUEUED, NOT YET
EXECUTED.
NEW (proposed, Reply 16-18): ADR-080 — persisted CTP file content stays in
English, matching established practice of CTP-SESSION-ARC-001 through 005;
Arabic is reserved for the owner-facing in-reply summary only. Owner has not
objected as of Reply 22 but has also not explicitly ratified it — still
open, listed in Section 10.

## 5. Roles and visibility
Unchanged from ADR-079 baseline (Caissier/Gerant/Admin roles in use, Vendeur
unassigned, manager-PIN-approval-in-place pattern for returns and credit
overpayment). CONFIRMED this session via TASK-BRK-058: role/permission
matrix lives at crates/domain/src/role.rs (enum Role ~L17; enum Permission
incl. ManageCustomerCredit L59, RecordCreditPayment L61; permissions()
matrix L109-160). Referenced from crates/application/src/use_cases/customers.rs
and crates/commands/src/{dto,ipc}.rs.

## 6. Hard technical rules
Unchanged, zero violations this session: never change .gitattributes; never
edit/reformat migrations 0001-0013; no new dependencies without per-task
authorization; no position:fixed except modal/toast/palette (ADR-067); no
floats for money (ADR-069); destructive actions need typed/long-press
confirmation (ADR-074); work only in D:/Dev/Baraka UltraPOS; never touch the
live DB or %APPDATA%/%LOCALAPPDATA% com.baraka.ultrapos; linking cargo
commands need CARGO_BUILD_JOBS=2; files written via Node fs.writeFileSync
(UTF-8, no BOM, LF, exactly one trailing newline), never PowerShell
Set-Content; no Figma, static HTML prototypes only, zero JS/external
resources; NO_LEFTOVERS proof mandatory at the end of every task.
NEW this session: build-boot-package.mjs must NEVER be invoked without all
three of --charter/--ctp/--out explicit and in full, and --check must always
precede any real write (see Section 12).

## 7. Process rules
Raw-evidence-over-paraphrase is a hard, zero-exception rule across DSN, BRK,
AUD, DOC, BOOT, REL families: every numeric/count/hash claim in any executor
report must be backed by pasted raw command output. NEW lesson this session:
hash comparison between two values derived from the SAME in-memory source
object is CIRCULAR and does not prove fidelity to an external specification
— true independent verification requires an independently re-sourced
reference file and a literal diff (see Section 12, TASK-DOC-016-C1).

## 8. Repo chain (newest last)
- 682668b docs(architecture): REL-013 CTP and boot package for SESSION-ARC-005
- 6a71fcf docs(design): DSN-011 static prototype 1/5 - fast product entry FR
- 2c83002 docs(design): DSN-012..020 static prototypes FR+AR, series complete
  (27 files: 9 prototype HTML + 9 screenshot directories; single batch
  commit, intermediate pre-correction states not separately recoverable,
  declared explicitly in the commit message)
Still untracked as of Reply 22: docs/architecture/CTP-SESSION-ARC-006.md,
docs/architecture/BOOT-SESSION-ARC-006.md (both pending final regeneration
and a dedicated commit in a later task), plus this very file's pending
update (CTP-SESSION-ARC-005.md, tracked, will show as Modified).

## 9. Protected-file hashes
- docs/architecture/CHARTER-v2.0.md: 52866 B,
  sha256 0b1bb13c39b1cbd23477e86872730131fd64ee241266793a6850cfd50ff228d5
- docs/design/glossary-ar-fr.md: 7187 B,
  sha256 8024f521f0c2416e9c0ec9e28338f3b28721a6ccfb366d5dd3e40214e96ee2b8
- docs/design/04-design-tokens.md: 5950 B,
  sha256 e95a39544183a39e02c6354f2641822d2ed31953c7b60cb4fd803e2fb92fb01f
- docs/design/03-state-catalog.md line 9, rule S-R3 (no pre-disable on empty
  required fields).
- All 9 FR/AR prototype files (DSN-012..020): hashes/sizes/line-counts as
  recorded in TASK-REL-018's evidence and in the full session transcript;
  omitted here for brevity, all unchanged since acceptance, now committed
  at 2c83002.

## 10. Pending owner items
1. QUESTIONNAIRE-COMPTABLE-FR-v2.pdf: still not delivered. TOP PRIORITY the
   moment it arrives — interrupts any other work; must not be sent to the
   accountant before this review.
2. ADR-080 (CTP content language stays English): proposed, not yet
   explicitly ratified by the owner — no objection received either.
3. Accountant answers still needed: proof numbering, remaining balance,
   timbre on the proof, timbre base, A4 invoice mentions, document
   numbering, retention.
4. Cashier entry point for approved debt collection (Caisse & Clôture vs.
   customer picker in Vente): still undecided.
5. Arabic glossary terms owner review (112 terms; flagged rows 3, 19, 24,
   35, 57, 58; Administrateur/Gérant similarity rows 11-12): review material
   (all 10 prototype files with †/‡ annotations) is fully ready to present.
6. Wording correction queued, not executed: DSN-015/DSN-020 "v0.1 PROPOSÉ,
   en attente de confirmation du propriétaire" -> confirmed wording,
   reflecting the Reply-15 owner decision.

## 11. Roadmap status
DONE: TASK-DSN-011 through TASK-DSN-020 (full FR+AR prototype series),
committed at 2c83002. DONE: TASK-BRK-058 (engine recon, zero discrepancies).
DONE: TASK-AUD-015 (boot-tool recon, golden safety rule established). DONE:
TASK-DOC-016/-C1 (CTP-SESSION-ARC-006.md authored, independently verified
byte-identical). DONE: TASK-BOOT-017 (BOOT-SESSION-ARC-006.md generated,
independently verified against both source files). DONE: TASK-REL-018 (git
continuity gap closed for DSN-012..020). IN PROGRESS: this file's own
update (TASK-DOC-019). NEXT: final regeneration of CTP-SESSION-ARC-006.md
(to capture DOC-019/REL-018 state) and BOOT-SESSION-ARC-006.md, then a
commit covering CTP-005 (this update) + CTP-006 + BOOT-006 together; only
then is session ARC-005 ready to end. AFTER session end: wording correction
(Section 10 item 6) -> TASK-BRK-059 (print_jobs.document_id NOT NULL,
migration 0014) -> glossary review presentation -> Caissier/credit
permission-split task (no longer blocked, overpayment question resolved).

## 12. Errors and dead ends
All errors already on record from CTP-SESSION-ARC-001..004 plus: TASK-DSN-012,
DSN-016, DSN-018, DSN-019 evidence-paraphrase/memory-count errors, each
corrected via -C1 with raw grep/sed/sha256 evidence.
CTP-PERSISTENCE INCIDENT (Replies 15-18): the architect wrongly assumed the
Reply-10 CTP had never been persisted, then invented a WRONG path/filename
(docs/architecture/ctp/SESSION-ARC-005-CTP-R15.md) to "fix" it. The owner
caught this via a direct directory screenshot. ROOT CAUSE: misunderstanding
that docs/architecture/CTP-SESSION-ARC-005.md is the session's OWN living
file (authored by the prior session as this session's boot seed, then
updated in place by this session itself at checkpoints) — not a per-reply
artifact. CORRECTED MODEL: one CTP file per session, no subfolder, no
per-reply suffix; updated in place at 10/20/30 and at session end; the NEXT
session's file is authored once, at session end.
BOOT-TOOL SAFETY FINDING (TASK-AUD-015, verified from source): 
build-boot-package.mjs has NO unrecognized-flag detection; any flag besides
--charter/--ctp/--out/--check (including --help) is silently ignored and
falls through to an UNCONDITIONAL overwrite of --out (default points at
ARC-002 files if --out omitted). --check is read+compare only, the only safe
dry-run. BINDING RULE: always pass all three flags explicit and in full;
always --check before any real write.
CIRCULAR-VERIFICATION LESSON (TASK-DOC-016/-C1): comparing a pre-write
in-memory buffer's hash to the post-write on-disk hash proves write-
integrity only, NOT fidelity to the original specification (both values
trace to the same source object). True independent verification requires an
independently re-sourced reference (re-typed from the original task text,
never derived from the file under test) and a literal diff. Applied
successfully in DOC-016-C1 and again in BOOT-017's extraction-and-diff step
(which also required an explicit, non-hand-waved reconciliation of a
one-byte-per-region discrepancy caused by marker-boundary LF stripping).

## 13. Key engine and domain facts
Unchanged from the CTP-SESSION-ARC-004 baseline, RE-VERIFIED where in scope
by TASK-BRK-058 this session (72 IPC commands, 13 migrations with matching
hashes, no 0014+ file, full Rust/Vitest suites green, clippy/typecheck/lint/
build clean, print_jobs.document_id NOT NULL confirmed at migration 0001
lines 279-288, role/permission matrix location confirmed). DocumentType 8
variants; customers table columns; Role::permissions() totals (Vendeur 5,
Caissier 13, Gerant 29, Admin 34); credit_record_payment transaction shape;
AUD-014 findings on document_sequences/timbre; 18 modals, 19th unidentified.
No contradictions found this session.

## 14. Open design points
Unchanged: rail default at 1024px unconfirmed; Ctrl+digit conflicts
unmeasured; code-splitting budget unset; dark theme unchecked for
accessibility; 18-modals-to-component-inventory mapping still a later audit
task. RESOLVED this session: git continuity gap (Section 8, closed via
TASK-REL-018).

## 15. Unknowns
Unchanged: the 19th modal; A4/other document layouts; real scanner wedge
behaviour; shop PC spec; small-screen behaviour beyond the nominal matrix;
Ctrl+K command-palette content; customer data inside audit_log rows.

## 16. Instructions to continue session ARC-005 from Reply 23
- Regenerate docs/architecture/CTP-SESSION-ARC-006.md (full rewrite, same
  path, same independent-diff verification discipline as TASK-DOC-016-C1)
  to capture TASK-DOC-019/TASK-REL-018 state.
- Regenerate docs/architecture/BOOT-SESSION-ARC-006.md using the SAME safe
  three-step protocol as TASK-BOOT-017 (--check, write, --check, then
  independent extraction-and-diff).
- Commit CTP-SESSION-ARC-005.md (this update) + CTP-SESSION-ARC-006.md +
  BOOT-SESSION-ARC-006.md together in one task, with an explicit commit
  message.
- Only THEN present the final BOOT-SESSION-ARC-006.md content to the owner
  as ready to start session ARC-006.
- After session end: wording correction (DSN-015/020) -> TASK-BRK-059 ->
  glossary review presentation -> Caissier/credit permission-split task.
- Never claim "100%" or "production ready" while a gate or measurement is
  open. Never invoke build-boot-package.mjs without all three explicit
  flags and a prior --check. Always use independent re-sourced diffs, never
  circular hash comparisons, to verify file-write fidelity.
