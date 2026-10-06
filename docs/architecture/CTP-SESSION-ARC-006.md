# CONTEXT TRANSFER PACKAGE — SESSION ARC-006

Status: ACTIVE (Checkpoint at Reply 10)
Date updated: 2026-10-06
Authoring session: SESSION-ARC-006 (reply 10, TASK-DOC-021)
Supersedes: SEED state of CTP-SESSION-ARC-006.md

## 1. Role & Scope

You are the Master Principal Architect & Design Lead of Baraka UltraPOS,
acting under CHARTER-v2.0. Baraka UltraPOS is a Tauri v2 desktop POS for an
Algerian hardware store. The Rust/SQLite engine now contains 14 migrations
(0001-0014, with 0014 added in TASK-BRK-059 to allow nullable print_jobs.document_id).
There are 72 IPC commands.
The Triad Protocol governs every reply: an Arabic-only owner-facing section,
an English-only executor task block, exactly one task per reply (continuations
suffixed -C1, -C2...; CTPs are the sole exception), a mandatory HANDOFF clause
on every executor task, mandatory raw/literal evidence in every executor report,
and a full Context Transfer Package checkpoint every 10th reply of this session's
counter.

## 2. CTP Lifecycle Model (Binding)

Each session owns exactly one CTP file at the flat path
`docs/architecture/CTP-SESSION-ARC-00N.md` — no subfolder, no per-reply suffix.
The CURRENT session updates its OWN file in place at each checkpoint
(reply 10, 20, 30...) and once more at session end.
SESSION-ARC-006 owns and updates `CTP-SESSION-ARC-006.md` in place.

## 3. Boot-Tool Golden Rule (Binding)

`docs/architecture/tools/build-boot-package.mjs` has NO unrecognized-flag
validation. Any flag besides `--charter`, `--ctp`, `--out`, `--check`
(including `--help`) is silently ignored, and execution falls through to an
unconditional overwrite of `--out`. BINDING RULE: never invoke this tool without
all three of `--charter`, `--ctp`, `--out` explicit and in full, and ALWAYS run
`--check` first before any real write.

## 4. Owner & Communication Rules

The owner is the non-technical "sacred bridge." Every reply: SECTION 1 Arabic
(owner-facing, Modern Standard Arabic only, no dialect, never containing technical
instructions), SECTION 2 English (executor, exactly one fenced task block).
Reply header format: "الرد رقم N — SESSION-ARC-006".

## 5. Owner Business Decisions (Status)

- Overpayment blocking: RATIFIED and COMMITTED in prototypes 05 (commit `3e7897e`).
  A Caissier is strictly forbidden from collecting any payment exceeding the
  customer's remaining debt balance; any excess collection strictly requires
  ManageCustomerCredit (Gerant/Admin only).
- ADR-080 (PROPOSED, SESSION-ARC-005): Persisted CTP file content stays in English,
  matching CTP-001..005 practice; Arabic is reserved for the owner-facing in-reply
  summary only. Provisionally binding.
- Algerian Trade Glossary: 5 prioritized terms presented to the owner in Reply 9
  (Clients & Crédit vs الكريدي; Administrateur/Gérant; Avoir vs وصل إرجاع;
  À crédit vs كريدي; Preuve d’encaissement vs وصل تسديد). Awaiting owner guidance.

## 6. Hard Technical Rules (Binding)

ADR-067 (no `position: fixed` except modal overlays/toasts/command palette);
ADR-069 (no floats for money, Latin-digit formatted strings only, wrapped in `<bdi>`);
ADR-074 (destructive/irreversible actions require typed or long-press confirmation);
ADR-077; ADR-079 (PIN values never rendered as real digits, masked bullets only);
ADR-080 (proposed, see Section 5). No JavaScript in static prototypes; no external
resources/network calls; all CSS inline; all icons inline `<svg>`. Files written via
Node.js `fs.writeFileSync` with explicit UTF-8 buffer, LF line endings only, no BOM,
lines joined with `\n`, exactly one trailing newline; never PowerShell Set-Content.
`wc -l` semantics are authoritative for line-count evidence. NO_LEFTOVERS: all
ad-hoc scratch scripts/files must be deleted before task completion. Work only in
`D:/Dev/Baraka UltraPOS`. Never edit `.gitattributes` or migrations 0001-0013.

## 7. Process Rules (Binding)

- Raw/literal evidence only in every executor report.
- CIRCULAR-VERIFICATION BAN: independent re-sourced reference plus literal raw `diff`.
- Prefer fresh from-disk recomputation over manual tally.
- Never claim "100%" or "production ready" while any gate/measurement is open.

## 8. Repo Chain (as of SESSION-ARC-006 Reply 10)

- `772204a` — feat(engine): make print_jobs.document_id nullable in migration 0014 (BRK-059)
- `3e7897e` — docs(design): ratify cashier debt overpayment ban in prototypes 05 (DSN-021-C1)
- `e02db2e` — docs(architecture): session handoff ARC-005 to ARC-006 (DOC-019, DOC-020, BOOT-021)
- `2c83002` — docs(design): DSN-012..020 static prototypes FR+AR, series complete

## 9. Accepted Design Deliverables

FR (docs/design/prototypes/):
1. `01-fast-product-entry.fr.html` (DSN-011) — sha256 `75130706...`, 6296 B.
2. `02-caissier-credit-chip.fr.html` (DSN-012+C1) — sha256 `c02241da...`, 5689 B, 179 lines.
3. `03-nouveau-client-rapide.fr.html` (DSN-013) — sha256 `06dcd0ae...`, 9622 B, 304 lines.
4. `04-autorisation-gerant.fr.html` (DSN-014) — sha256 `c985b143...`, 9625 B, 302 lines.
5. `05-encaissement-dette.fr.html` (DSN-021-C1, committed `3e7897e`) — sha256
   `ca78fa7a6aa50d876ca1587a563859bd2f60bbcdd6c7af08c40b1e4ae9046ff6`, 9955 B, 304 lines.

AR/RTL (docs/design/prototypes/):
1. `01-fast-product-entry.ar.html` (DSN-016+C1) — sha256 `aaf4cb0b...`, 7477 B, 245 lines.
2. `02-caissier-credit-chip.ar.html` (DSN-017) — sha256 `282450ef...`, 6901 B, 187 lines.
3. `03-nouveau-client-rapide.ar.html` (DSN-018+C1) — sha256 `2f1798f7...`, 11625 B, 315 lines.
4. `04-autorisation-gerant.ar.html` (DSN-019+C1) — sha256 `d92676eb...`, 11122 B, 307 lines.
5. `05-encaissement-dette.ar.html` (DSN-021-C1, committed `3e7897e`) — sha256
   `4341a6977eb5a1b32d87e2dbb605b087eb26e7cbcea47c9b15d91f45e92f1dad`, 11782 B, 309 lines.

## 10. Errors & Lessons Learned (Carried & New)

- Blocker on non-verbatim targets (TASK-DSN-021): Architect supplied target strings
  from memory rather than disk, correctly blocked by executor. Rectified via DSN-021-C1
  using disk-exact targets. Lesson: always audit literal disk lines before specifying replacements.
- CTP Lifecycle discipline: Updates must occur in-place at `docs/architecture/CTP-SESSION-ARC-00N.md`.
- Boot-tool safety: Confirmed that `--check` is the only safe dry-run mechanism.

## 11. Key Engine/Domain Facts (Verified in ARC-006)

- 14 migrations (0001-0014). Migration 0014 made `print_jobs.document_id` nullable in SQLite
  and adapted the Rust domain (`PrintJob.document_id: Option<String>`), application port
  (`PrintJobRecord.document_id: Option<DocumentId>`), and infra repositories.
- Role/permission enforcement (AUD-018): Caissier has `RecordCreditPayment`, lacks
  `ManageCustomerCredit`. Overpayment check already exists in `sqlite_customer_repository.rs`
  (`PaymentExceedsBalance{balance, attempted}` and `OverpaymentUnauthorized`).
- Thermal printing pipeline (AUD-019): Fixed 576 dots width (80mm @ 203dpi) in `rasterizer.rs`.
  `PrintWorkerService` currently fails on `document_id == None` because it loads `Document`.
  Non-sale receipts require worker dispatch to hydrate directly from `payload_json`.
- Document sequences: TICKET, FACTURE, BL, DEVIS, AVOIR, BC, BR, FF. No sequence currently
  exists for debt receipts (`JUSTIFICATIF` / `RECU`).
- Test suite status: `cargo test --workspace -j 2` → 367 passed / 0 failed / 4 ignored.
  Clippy clean (-D warnings exit 0). Fmt clean.

## 12. Open Design Points / Unknowns

1. `QUESTIONNAIRE-COMPTABLE-FR.html` was verified in AUD-017 to be an empty draft template
   (341 lines, 77 blank answer boxes). Owner delivery of completed questionnaire or
   `QUESTIONNAIRE-COMPTABLE-FR-v2.pdf` remains pending.
2. ADR-080 formal ratification pending.
3. Glossary feedback from owner pending (5 terms presented in Reply 9).
4. Thermal receipt sequence key for debt receipts (`JUSTIFICATIF` vs `RECU` vs `CUSTOMER_PAYMENT`).

## 13. Roadmap Status

COMPLETED IN ARC-006:
- AUD-016: Handoff verification and boot check.
- DSN-021-C1: Ratify overpayment ban in prototypes 05 (FR & AR).
- REL-019: Commit ratified prototype 05 files (`3e7897e`).
- AUD-017: Reconnaissance for Migration 0014 and print schema.
- BRK-059: Migration 0014 (nullable print_jobs.document_id), adapted domain/app/infra, 2 new tests.
- REL-020: Commit Migration 0014 and print pipeline changes (`772204a`).
- AUD-018: Credit payment permission and logic reconnaissance.
- AUD-019: Thermal printing and worker dispatch reconnaissance.

IN PROGRESS:
- DOC-021: Reply 10 CTP Checkpoint (this task).

NEXT:
- TASK-BRK-060: Implement debt receipt generation, sequence allocation, and `PrintWorkerService`
  payload dispatch for non-sale print jobs.
- TASK-REL-021: Commit BRK-060 engine deliverables.

## 14. Reply Counter / CTP Checkpoint Rule

A full CTP checkpoint — updating THIS file in place — is mandatory at reply 10, 20, 30...
of SESSION-ARC-006's own counter. Next checkpoint: Reply 20.

## 15. Section Reserved

(Reserved for future protocol notes).

## 16. Instructions to Continue Session ARC-006

SESSION-ARC-006 is active. The engine is at commit `772204a` with 14 migrations and 367 tests
passing. Proceed immediately to TASK-BRK-060 to wire debt collection receipt printing.
