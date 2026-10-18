# Context Transfer Package — SESSION-ARC-020 (Opening Edition — Continues SESSION-ARC-019)

## 0. Purpose, Usage, and a Correction On Record

This document is the SOLE required bootstrap artifact, alongside `BOOT-SESSION-ARC-020.md`, for SESSION-ARC-020,
a new conversation window continuing the Baraka UltraPOS architecture work previously conducted under
SESSION-ARC-019. SESSION-ARC-019 is now CLOSED. This file, together with the Baraka UltraPOS Charter v2.0 and
`BOOT-SESSION-ARC-020.md`, constitutes the complete substitute for re-reading SESSION-ARC-019's full transcript: in
this new session, "read the entire session transcript from the start" is satisfied by reading this document in
full, plus the Charter, plus `BOOT-SESSION-ARC-020.md`, before every single reply, without summarizing or
shortening, with deep thought before writing a word — exactly as the owner has mandated, unconditionally, to be
self-applied without needing restatement.

**Correction on record:** two earlier attempts at this transfer (TASK-CTP-059, TASK-CTP-060) incorrectly wrote
this same content to `docs/architecture/CTP-SESSION-ARC-019.md` — the OLD session's filename — instead of to a
new, correctly-numbered file for the new session. This was an architect error, caught by the owner, not by
self-review, and is recorded here permanently as a standing lesson (see Section 11, item 13, and the new standing
rule in Section 3). `CTP-SESSION-ARC-019.md` remains in the repository as the closed historical record of
SESSION-ARC-019 and must never again be overwritten for SESSION-ARC-020 content. This file,
`CTP-SESSION-ARC-020.md`, is the first and only transfer artifact under the correct name.

SESSION-ARC-019 ran from its first reply through its closing reply (during which it closed the dark-mode
design-token crisis sub-arc and the Suppliers-deferred-states sub-arc, detailed fully in Sections 6-9 below), and
ended at the point this transfer was issued. **SESSION-ARC-020 begins at Reply 1.** Per Charter §15, the next
mandatory periodic CTP is due at **Reply 10 of SESSION-ARC-020** (not tied to any reply count from the prior
session).

No fact in this document is inferred or assumed beyond what was directly established in SESSION-ARC-019 via the
Charter, executor reports, or explicit owner statements. Where a number, hash, or measurement is restated here, it
is restated as reported and reviewed, not re-derived.

## 1. Governing Framework and Role

- Governing document: Baraka UltraPOS Charter v2.0 (binding, supersedes the old "Sovereign Constitution v1.0").
- Role held: Master Principal Architect & Design Lead, Baraka UltraPOS — sole architect of the product.
- Session identifier: SESSION-ARC-020 (continuing from SESSION-ARC-019).

## 2. The Mission (verbatim intent, unabridged)

Deliver, as sole architect of Baraka UltraPOS, a finished, honest, visually excellent, fast, offline-first POS for
an Algerian hardware-store (quincaillerie) owner — per Charter v2.0. Definition of Done is Charter §9 (functional +
visual + realistic-data + owner + documentation gates); test/task counts never define completion. Must never
declare "100%" or "production ready" while any owner gate (real thermal-printer verification, tax/legal
validation, hardware checks) or measurement remains open.

SESSION-ARC-019's task thread closed the dark-mode design-token crisis (AUD-015 -> TASK-DSN-055/ADR-094 ->
TASK-DSN-056/ADR-095) and the Suppliers-deferred-states sub-arc (TASK-BRK-057 -> TASK-AUD-058 -> TASK-DSN-054-C2).
SESSION-ARC-020 opens with the standing queue's next item: severity-tiered batched dark-mode contrast migration
for the ~74 remaining AUD-015-flagged files (Section 10, item 1), followed by the rest of the fixed queue in
Section 10.

## 3. Standing Process Rules (apply to every future reply without exception — no rule here is optional)

- Exactly one task issued per reply; a task may only be issued after the prior task's executor report has been
  reviewed. No task may be issued "ahead" of a pending report.
- Each task is fully self-contained per Charter §2.8: all literal content to be written (ADR text, token tables,
  CTP content, test fixtures, etc.) is embedded directly in the task block — never "see above," never "copy from a
  prior section." (TASK-REL-061, a prior-session task, was voided for violating exactly this.)
- Each task has one single objective, sized to fit one executor session, with a HANDOFF clause (STATE/DONE/
  REMAINING) and continuation suffixing (-C1, -C2, ...; use the next free suffix if a base task id already has one).
- Every UI-affecting task requires: exact SCOPE-IN/SCOPE-OUT paths, VIEWPORT MATRIX, VISUAL ACCEPTANCE with named
  screenshots and MEASURABLE contrast computations (the actual foreground/background token pair and numeric ratio
  must be pasted — not a bare pass/fail claim), zero page scroll, no `position:fixed` anywhere except modal
  backdrop / toast / command palette, contrast >=4.5:1 WCAG AA (or stated large-text 3:1), all money/quantity
  digits Latin + wrapped in `bdi[dir="ltr"]` + `tabular-nums`, zero emoji anywhere in the DOM (grep-proven, empty
  match pasted).
- Regression law (Charter §5): every bug fix requires a test proven to FAIL on the defective code before the fix,
  then PASS after the fix — BOTH outputs pasted verbatim. CLARIFICATION (recorded in SESSION-ARC-019 after
  TASK-DSN-054-C2): a test that merely GUARDS an already-correct pre-existing behavior against future accidental
  regression (not a newly-introduced fix) is NOT required to fail pre-fix. Future task text invoking the
  regression law must explicitly distinguish "tests of a newly fixed defect" (must fail pre-fix) from "tests
  guarding pre-existing correct behavior" (need not fail pre-fix), to avoid ambiguous acceptance criteria.
- Migrations 0001-0015 are immutable (read-only inspection permitted; editing forbidden, no exceptions).
  `.gitattributes` (`* -text`) must never change under any circumstance. No CRLF/LF drift is permitted — every
  report that edits an EXISTING file must paste a before-edit and after-edit line-ending check proving no drift.
- No new npm or cargo dependency may be added without explicit per-task authorization. If an executor believes one
  is genuinely needed, the correct behavior is to STOP and report why, not to add it unilaterally.
- No git tag `v1.0.0` may be applied until Charter §9.2 (Definition of Done) fully holds.
- Money is NEVER floating point; money = i64 centimes. Quantities use fixed-point arithmetic at x10^4 scale.
- Owner communicates in Modern Standard Arabic (فصحى) only; architect's Arabic-facing sections must be فصحى — no
  dialect, no padding, no flattery, no filler. Executor-facing instructions must be high-precision Staff-Engineer
  English, fully self-contained, zero ambiguity, embedded verbatim. Arabic (owner-facing) and English
  (executor-facing) content are NEVER mixed within the same block; exactly one task is issued per reply inside
  exactly one fenced code block containing ONLY the English executor instructions; owner-facing commentary is
  plain prose OUTSIDE that block.
- CTP cadence: produced every 10th reply of the CURRENT session (Charter §15), AND, separately, on ad hoc
  conversation-transfer triggers. **CTPs are authored in ENGLISH** (permanent owner correction, overriding the
  Charter's Arabic-default assumption for this specific artifact). **NEW STANDING RULE (added at this transfer,
  owner-mandated, permanent):** when a conversation transfer creates a new session, the new session receives a
  NEW session number; its CTP and BOOT documents are authored under that NEW number from the very first transfer
  artifact (e.g. `CTP-SESSION-ARC-0{N+1}.md`, `BOOT-SESSION-ARC-0{N+1}.md`); the old session's CTP/BOOT files are
  left completely untouched as historical record and are NEVER reused or overwritten for the new session's content.
- The owner's standing triple directive is to be silently and fully re-applied before EVERY single reply, without
  exception, without needing restatement by the owner:
  (1) **Throughput directive**: maximum work intensity/throughput; method delegated to architect's judgment.
      Chosen method: precision over batching — small, unambiguous, single-objective tasks over bloated
      multi-deliverable ones; unrelated concerns always split into separately sequenced tasks; tightly-coupled
      parts of one true objective (e.g. a BOOT+CTP pair for one transfer) may share a task.
  (2) **Hyper-strict re-read directive**, mandatory, not optional: before writing any reply, re-read the ENTIRE
      Charter and the ENTIRE session record (in this new session: this CTP document plus `BOOT-SESSION-ARC-020.md`
      in full) from the start, WITHOUT summarizing or shortening — every single time, before every single reply.
      Think deeply and intensively before writing a single word of the reply. This includes verifying conventions
      (such as file-naming on session transfer) against what the documents actually establish, rather than
      defaulting to a plausible-seeming shortcut — a failure of exactly this kind occurred once at the
      SESSION-ARC-019 -> 020 boundary (Section 11, item 13) and must not recur in any form.
  (3) **Excellence directive**: work at maximum engineering/architectural/creative/analytical capability at all
      times; act as a world-class architect/designer pursuing excellence, not mere task completion; raise ambition
      beyond conventional/obvious solutions; anticipate problems before they occur; actively search for non-obvious
      improvements; never settle for a mediocre solution when a better one is reachable; critically review one's
      own decisions; balance architecture quality, execution strength, design originality, UX, performance,
      security, scalability, and maintainability simultaneously; every decision must have a clear, stated
      justification; every component must be part of one coherent integrated system; treat this as professional
      work meant to compete with the best global products.
- Charter §1.4 (intellectual honesty) is a hard law: never guess facts about the codebase, session state, or
  conventions — not even ones that "seem obvious" (see Section 11, item 13); every fact used must come from
  documents pasted by the owner, executor reports, or screenshots. When a fact or convention is unknown or
  uncertain, order a read-only audit task or ask the owner — never proceed on a guess. Openly correct own mistakes
  the moment they are found — this is correct, expected, professional behavior, not a failure to hide.
- No feature creep: every feature, scope extension, or new file must trace to a genuine job-to-be-done and a
  commercial benefit for the quincaillerie owner.
- The architect must NEVER ask the owner to perform technical merging, editing, or assembly of task text or code.
- `position: fixed` is forbidden everywhere EXCEPT modal backdrops, toasts, and the command palette.
- Never touch the owner's live production database or `%APPDATA%/com.baraka.ultrapos`, under any circumstance —
  all capture/testing uses the dev server plus synthetic/mock data only.
- No claims of legal/tax compliance or hardware verification — owner gates only, never closed internally.
- No git tag `v1.0.0` before Charter §9.2's Definition of Done fully and verifiably holds.

## 4. User Constraints and Corrections Carried Forward From SESSION-ARC-019 (full list, none omitted)

1. Owner communicates in Modern Standard Arabic (فصحى) only; architect's Arabic section must be فصحى — no dialect,
   no padding, no flattery.
2. Executor instructions must be high-precision Staff-Engineer English, fully self-contained, zero ambiguity,
   embedded verbatim — never "see above."
3. Exactly one task per reply, inside exactly one fenced code block; Arabic and English sections never mixed.
4. Owner's standing triple directive (throughput/method-delegated, hyper-strict full re-read, maximum excellence)
   — full detail in Section 3 above, unconditional, self-applied without needing restatement.
5. Charter §1.4 intellectual honesty: never guess facts or conventions; source everything from pasted documents,
   executor reports, or screenshots; order a read-only audit or ask the owner when uncertain; correct own mistakes
   openly and immediately, without minimizing them.
6. No feature creep — every feature must trace to a job-to-be-done and commercial benefit.
7. Never ask the owner to perform technical merging/editing of task text.
8. No `position: fixed` anywhere except modal backdrops, toasts, command palette.
9. No floats for money; money = i64 centimes; quantities are fixed-point at x10^4 scale.
10. No editing of existing migrations (0001-0015, frozen); never touch `.gitattributes`.
11. Never touch the owner's live database or `%APPDATA%/com.baraka.ultrapos` — dev server plus synthetic data only.
12. No tag `v1.0.0` before Definition of Done (Charter §9.2) fully holds.
13. No claims of legal/tax compliance or hardware verification — owner gates only.
14. CTPs are authored in ENGLISH (permanent correction); saved by the executor verbatim to
    `docs/architecture/CTP-SESSION-ARC-0NN.md` for the CURRENT session's number.
15. The Reply-11 (SESSION-ARC-019) instruction governing full-fidelity context transfer: review every word of the
    conversation from the start without abbreviation or summarization; formulate the context package only after
    fully internalizing every word, with maximum intelligence, with no abbreviation or summarization;
    professionalism and technical precision are mandatory; ensure NOT ONE WORD is lost; the new conversation must
    begin as though the current one had never been transferred away (zero information loss); the hyper-strict
    re-read mandate applies to the FORMULATION of the transfer package itself, not only to future replies; the
    instruction to the executor must be phrased as a proper, fully self-contained, Charter-§2.8-compliant task.
16. **NEW (issued at this transfer):** on any conversation transfer, the new session's CTP and BOOT documents must
    be authored under the NEW session's number from the very first transfer artifact; the prior session's files
    are never reused or overwritten for the new session's content (see Section 3's new standing rule, and
    Section 11, item 13, for the error this corrects).

## 5. External Sources

None. No external web search, no Figma, no external URL, and no tool outside this project's own repository,
Charter document, and executor reports has been consulted at any point in SESSION-ARC-019 or at this transfer.

## 6. ADR Index State

- Pre-existing ADRs (confirmed via AUD-015): 066-079 and 082-093 — 26 ADRs total. Numbering gap at 080/081 is
  unexplained and NOT investigated — noted only, not actioned.
- ADR-094: "Dark-mode semantic design tokens" (`docs/adr/ADR-094-dark-mode-semantic-design-tokens.md`) — CLOSED.
- ADR-095: "ink-muted token tier refinement" (`docs/adr/ADR-095-ink-muted-token-tier-refinement.md`) — CLOSED.
- Filename convention: `NNN-kebab-case-title.md`.
- No ADR was created by TASK-BRK-057, TASK-AUD-058, or TASK-DSN-054-C2 (all explicitly scoped to introduce zero
  new tokens/architectural decisions).
- Next available ADR number: **096**.

## 7. Design Token System — Current Source of Truth (ADR-094 baseline + ADR-095 addition)

Location: `src/index.css`, `:root` (light) / `:root.dark` (dark) CSS custom properties; mapped into Tailwind via
`tailwind.config.js` into `surface`, `ink`, `line`, `success`, `warning`, `danger`, `info` color groups.

Root cause fixed: `src/index.css` had hardcoded body background/color (`#f8fafc` / `#0f172a`) with ZERO dark-mode
variant anywhere in `src/`, despite `tailwind.config.js` setting `darkMode: "class"`. The `.dark` class is toggled
ONLY by `src/components/pos/Topbar.tsx:33`, view-local state, desyncing after navigation (still-open bug, Section
10 item 2). This produced 685 ad hoc `dark:` Tailwind occurrences across 76 files with no shared contract. 6 of 7
AUD-015-sampled shipped screens failed WCAG AA in dark mode (ratios as low as 1.0:1). Diagnosed as architectural,
not a two-component bug; fixed via a token foundation first, then incremental measured migration.

### Light theme values (final):
- `--color-surface-canvas`: `#f8fafc` · `--color-surface-raised`: `#ffffff` · `--color-surface-sunken`: `#eef2f6`
- `--color-ink-primary`: `#0f172a` · `--color-ink-secondary`: `#334155` (adjusted from draft `#475569` for
  light-identity match with existing slate-700 copy)
- `--color-ink-muted`: `#64748b` (ADR-095; pinned EXACTLY to the pre-existing resolved `text-slate-500`,
  `rgb(100,116,139)`)
- `--color-ink-inverse`: `#f8fafc` · `--color-line-default`: `#e2e8f0` · `--color-line-strong`: `#94a3b8`
- `--color-success-bg/-fg`: `#dcfce7` / `#16803c` · `--color-warning-bg/-fg`: `#fef3c7` / `#b45309`
- `--color-danger-bg/-fg`: `#fee2e2` / `#c62828`
- `--color-info-bg/-fg`: `#dbeafe` / `#1d4ed8` (adjusted from Charter's literal `#2563EB`, which measured only
  4.24:1 — a FAIL vs 4.5:1 AA; adjustment authorized by Charter §8's own refinement clause)

### Dark theme values (final):
- `--color-surface-canvas`: `#0b1120` · `--color-surface-raised`: `#111827` · `--color-surface-sunken`: `#0a0f1a`
- `--color-ink-primary`: `#f1f5f9` · `--color-ink-secondary`: `#94a3b8`
- `--color-ink-muted`: `#94a3b8` (SAME hex as ink-secondary dark, by deliberate design — the two tokens share one
  dark value because their LIGHT identities differ and were pinned independently)
- `--color-ink-inverse`: `#0f172a` · `--color-line-default`: `#1e293b` · `--color-line-strong`: `#334155`
- `--color-success-bg/-fg`: `#14321f` / `#4ade80` · `--color-warning-bg/-fg`: `#3a2a0a` / `#fbbf24`
- `--color-danger-bg/-fg`: `#3b1414` / `#f87171` · `--color-info-bg/-fg`: `#0f2744` / `#60a5fa`

### Measured contrast ratios (light / dark), all >= required threshold:
ink-primary/canvas 17.06/17.19 · ink-primary/raised 17.85/16.19 · ink-secondary/canvas 9.90/7.34 ·
ink-secondary/raised 10.40/6.92 · ink-muted/canvas 4.55/7.34 · success 4.57/8.00 · warning 4.51/8.30 ·
danger 4.60/5.86 · info 5.49/5.93.

### Components migrated to tokens so far:
- `CustomerDossierPane.tsx` — teaching/empty panel (DSN-055). Dark fixed 1.05 -> 17.19.
- `SupplierListScreen.tsx` — no-match panel (DSN-055, fixed 1.18 -> 7.34 dark); EXTENDED in TASK-DSN-054-C2 with a
  zero-database teaching state and a default-list-on-empty-query state, tokens only, zero new raw Tailwind color
  utilities, zero new tokens.
- `ReportsHomeScreen.tsx` — card title auto-fixed by the root-canvas change (16.3 dark, no direct migration
  needed); subtitle (line 76, sole `text-slate-500` usage) migrated to `text-ink-muted` in DSN-056 (3.96 FAIL ->
  7.34 PASS dark; light 4.55, identical to pre-regression original).

The remaining ~74-file backlog is Section 10, item 1 — the next task to be issued in this session.

### OPEN VERIFICATION ITEM (non-blocking, STILL OPEN, carried forward unresolved from SESSION-ARC-019): TASK-
DSN-054-C2's report cited a contrast ratio of "14.48" for an element in the new Suppliers teaching state, not
matching any value in the ratio table above. Must be clarified (exact foreground/background token pair) the next
time any task touches `SupplierListScreen.tsx` — confirm it is an existing measured token pair (most plausibly an
icon/illustration color against `surface-raised`), not an undocumented new color value.

## 8. Confirmed Baseline — Gate Status Carried Into SESSION-ARC-020

- `cargo test --workspace -j 2`: **463 passed / 0 failed**.
- `npx vitest run`: **600 passed / 0 failed**, 59 test files.
- `cargo fmt --check`: clean. `cargo clippy --all-targets --all-features -D warnings`: clean. `npm run lint`:
  clean. `npm run typecheck`: clean. `npm run build`: clean (observed ~5.4s-14.1s, normal variance).
- `hermes verify` remains PERMANENTLY ENVIRONMENTALLY BLOCKED on this repository (confirmed across >=4 attempts):
  its Vite recipe bootstraps via `pnpm install`, aborting with `ERR_PNPM_IGNORED_BUILDS` (esbuild@0.21.5 needs
  `pnpm approve-builds`). This repo is npm-based. Approving build scripts would write trust policy into
  `package.json` — a decision outside any task's authority, NOT authorized. This is a TOOLING/ENVIRONMENT
  MISMATCH, not a code defect; do not re-attempt without an explicit owner/architect ruling.
- Repository commit history (last independently confirmed): HEAD commit `0812169` ("settings command center
  consolidation and catalog purification [TASK-DSN-053]"), followed by docs-only `ecc1b4d` (CTP/BOOT-ARC-019 +
  ADR-093). On top of this, SESSION-ARC-019 produced MULTIPLE accepted-but-not-yet-committed deliverables:
  TASK-DSN-055, TASK-DSN-056, TASK-BRK-057, TASK-AUD-058, TASK-DSN-054-C2, TASK-CTP-059 (now superseded, left as
  historical record), TASK-CTP-060 (superseded by this document), TASK-CTP-061 (this document's own task).
  **`git status --porcelain` is NOT expected to be empty at any point** — each task's "scope proof" must show only
  ITS OWN declared files changed relative to the prior accepted state, not an empty tree. This is permanent,
  standing understanding, confirmed correct in SESSION-ARC-019 (TASK-AUD-058's report).

## 9. Completed Task Log — SESSION-ARC-019 (chronological, exhaustive, carried forward in full; nothing summarized)

1. **TASK-DSN-053** (pre-session) — settings command center consolidation and catalog purification; HEAD commit
   `0812169`.
2. **AUD-015** (pre-session-019-visible-work) — systemic dark-mode contrast audit; found root cause (Section 7);
   found 6 of 7 sampled screens failing WCAG AA in dark mode (ratios as low as 1.0:1); produced
   `audit/AUD-015/REPORT.md`, the severity-data source for Section 10 item 1.
3. **TASK-DSN-054 / TASK-DSN-054-C1** — disproved the original pre-session CTP's false claim of a shared "instant
   search primitive" (Section 11, item 1); found two components with unreadable dark-mode text (escalated to
   DSN-055/ADR-094); discovered and correctly deferred the `suppliers_list` engine gap (resolved later via
   TASK-BRK-057).
4. **TASK-DSN-055 (ADR-094)** — built the full semantic design-token system (Section 7); migrated
   `CustomerDossierPane.tsx` and `SupplierListScreen.tsx`'s no-match panel; fixed 6 of 7 AUD-015 screens via the
   root-canvas change; introduced one measured regression in `ReportsHomeScreen.tsx`'s subtitle (4.55 -> 3.96
   FAIL), correctly reported, not silently patched. Deliverables: `docs/adr/094-...md`, `docs/design/design-
   tokens.md`, `audit/DSN-055/` (script, 11 PNGs, REPORT.md).
5. **TASK-DSN-056 (ADR-095)** — added the `ink-muted` token tier specifically to close the DSN-055 regression
   WITHOUT reusing `ink-secondary` (would have caused a different unauthorized light-mode drift); migrated ONLY
   the regressed subtitle (line 76) to `text-ink-muted`; confirmed card title untouched (16.3 dark); confirmed
   zero collateral across all 7 AUD-015/DSN-055 screens. Self-corrected a stray `pnpm-lock.yaml` change (side
   effect of the executor's own prior `hermes verify` probe) BEFORE starting real work, reported transparently.
   Deliverables: `docs/adr/095-...md` (light `#64748b`, dark `#94a3b8`, ratio 7.34:1), one new design-tokens.md
   row, `audit/DSN-056/` (script, 10 PNGs, JSON, REPORT.md), including a compiled-CSS bundle proof. This CLOSED
   the entire dark-mode design-token crisis sub-arc.
6. **TASK-BRK-057** — added `suppliers_list` IPC capability as a full-stack exact mirror of `customers_list`:
   trait `fn list_suppliers(&self, active_only: bool) -> Result<Vec<SupplierView>, ApplicationError>;` (`ports.rs`,
   mirroring `fn list_customers` at `ports.rs:1092`/trait `:1048`); SQLite impl `"SELECT {SUPPLIER_COLUMNS} FROM
   suppliers WHERE (?1 = 0 OR is_active = 1) ORDER BY name"` (format!, reusing `map_supplier`, no pagination,
   mirroring `sqlite_customer_repository.rs:387`); use case `ListSuppliersUseCase::execute(&self, session:
   &SessionContext, active_only: bool)` in `crates/application/src/use_cases/suppliers.rs`, gated by
   `Permission::SupplierManage` (later independently audited safe by TASK-AUD-058); Tauri command
   `suppliers_list(state, session_token: String) -> Result<SearchSuppliersResponseDto, String>`, registered at
   `src-tauri/src/lib.rs:279` (mirroring `customers_list` at `:255`); frontend binding `ipcListSuppliers
   (sessionToken: string): Promise<SupplierView[]>` in `src/lib/ipc.ts` (CRLF preserved, +11/-0, verified), ZERO
   call sites at delivery (additive-only, UI wiring deferred). FAIL-then-PASS proven (compiler errors ->
   `1 passed; 0 failed`). Rust tests 462 -> **463**. Vitest unchanged at 597 (pure typed passthrough, no trivial
   test fabricated, per that task's own Acceptance Criterion 9). One declared deviation: a pre-existing
   `supplier-empty-states.test.ts:92-101` assertion (pinning the capability's ABSENCE) was minimally updated to
   assert PRESENCE once the command landed — disclosed, no UI/store touched. `customers_list`-related files
   proven byte-identical throughout. Deliverable: `audit/BRK-057/REPORT.md`.
7. **TASK-AUD-058** (read-only, ZERO files modified by design) — verified `suppliers_list`'s `Permission::
   SupplierManage` gate against the existing supplier search path's gate (`SearchSuppliersUseCase::execute`,
   `suppliers.rs:168`) — EXACT SAME variant. `SupplierManage` is not the only supplier permission (a second,
   `SupplierPaymentRecord` at `role.rs:79`, gates an unrelated concern — payment recording, `purchases.rs:376`).
   Role mapping (`Role::permissions()`, `role.rs:109-205`): Gerant+Admin hold both; Vendeur/Caissier hold neither;
   no role can search while being denied listing, or vice versa. **CONCLUSION: NO ASYMMETRY — safe to wire without
   further decision.** One honesty note accepted: the task's Acceptance Criterion 3 literally (and imprecisely)
   demanded an "empty" `git status --porcelain`, technically impossible given prior accepted-but-uncommitted
   deliverables; the executor correctly pasted the full output and clarified only its own single deliverable was
   new — accepted as correct handling of an imprecisely-worded criterion, not a failure. Deliverable:
   `audit/AUD-058/REPORT.md`.
8. **TASK-DSN-054-C2** — wired `ipcListSuppliers` into `SupplierListScreen.tsx`, closing the two Suppliers states
   deferred since the original TASK-DSN-054 (default list on empty query; zero-database teaching state). No
   BLOCKED trigger. Store-level ternary (`supplier-store.ts`, mirroring `CustomersModule.tsx:79`'s pattern found
   in BRK-057's Step 0). Teaching state mirrors the existing Customers teaching-panel precedent, adapted only with
   supplier-specific nouns/CTA, tokens exclusively, grep-confirmed zero new raw Tailwind color utilities, zero new
   tokens, zero new ADR. Balance figures render via `<bdi dir="ltr">15 250,00 DZD</bdi>` with `tabular-nums`.
   Pre-existing permission guard confirmed sufficient (consistent with AUD-058). No-match panel re-verified
   BYTE-IDENTICAL to DSN-055 baseline (7.34:1 dark) — zero collateral. "BEFORE" screenshot captured (dead-end
   "lancez une recherche" static paragraph). Three new regression tests: 2 correctly failed-then-passed (teaching
   state render condition, store wiring); 1 correctly passed in both pre-/post-fix states (guarding the
   PRE-EXISTING no-match-panel behavior, never a defect itself) — revealing an imprecision in the architect's own
   Acceptance Criterion 10 wording (see Section 3's regression-law clarification and Section 11, item 10). Vitest
   597 -> **600** (+3). One declared, accepted, flagged scope extension: 3 new i18n keys (FR+AR) for the teaching
   copy, not originally in SCOPE-IN — accepted WITH a standing process note: future scope extensions must be
   flagged at the moment of discovery, not only in the final report. `src/lib/ipc.ts` and all Rust files confirmed
   untouched (pure frontend task). One open item flagged and carried forward: the "14.48" ratio (Section 7's OPEN
   VERIFICATION ITEM). Deliverables: modified `SupplierListScreen.tsx` + store file, extended/new tests,
   `audit/DSN-054-C2/` (script, 7 PNGs: 1 BEFORE + 6 AFTER, REPORT.md). This CLOSED the entire Suppliers-deferred-
   states sub-arc.
9. **TASK-CTP-059** — persisted a periodic checkpoint CTP (later found to be an incomplete/preliminary edition) to
   `docs/architecture/CTP-SESSION-ARC-019.md`. Tooling quirk: `write_file`'s stale-overwrite guard refused 3x
   despite a complete prior read; shell-heredoc workaround failed on apostrophe-quoting; resolved via
   delete-then-recreate, accepted as a tooling quirk not a content defect. SHA-256 checksum pasted and
   independently re-verified by the architect (64 hex chars, valid length) before being accepted as fact —
   a positive example of Charter §1.4 verify-before-asserting discipline (Section 11, item 12).
10. **TASK-CTP-060** — persisted the FULL conversation-transfer edition of the CTP (superseding TASK-CTP-059's
    output), again written to `docs/architecture/CTP-SESSION-ARC-019.md` — **this filename choice was the error
    corrected by this very document and by `TASK-CTP-061`** (Section 11, item 13). Its substantive content
    (Sections 1-10, 12 as structured there) is fully carried forward into THIS document with only session-number
    and transfer-framing corrections applied — no substantive fact was lost or altered in that correction.
11. **TASK-CTP-061** (the task delivering this very document, plus `BOOT-SESSION-ARC-020.md`) — corrects the
    session-transfer artifact naming error: authors the two correctly-named session-020 artifacts, leaves all
    session-019 artifacts (including `CTP-SESSION-ARC-019.md`, now closed/historical) completely untouched.

## 10. Standing Queue (fixed order, re-affirmed at every reply without exception; item 1 is NEXT, first task of
    SESSION-ARC-020)

1. **Batched migration tasks** for the remaining AUD-015-flagged files with SUSPECT/UNKNOWN dark-mode contrast
   risk (~74 files remain beyond those already migrated per Section 7), prioritized by severity data in
   `audit/AUD-015/REPORT.md`. **NOT YET SCOPED.** Per the precision-over-batching method, split into MULTIPLE
   small, severity-tiered tasks (Tier 1 = highest severity first), each sized to one executor session, each with
   its OWN mandatory Step 0 that RE-MEASURES current contrast ratios of its specific target files BEFORE any edit
   — AUD-015's original numbers must NOT be assumed current, since TASK-DSN-055's root-canvas change already
   silently fixed some files and may have silently altered others not yet individually re-checked. **This is the
   task to be issued at Reply 1 of SESSION-ARC-020 (the first reply inside this new conversation window).**
2. Fix the dark-mode toggle desync bug in `src/components/pos/Topbar.tsx:33` (view-local React state, desyncs
   from the applied theme after navigation) — logged technical debt, confirmed via AUD-015, not yet scheduled.
3. The ORIGINAL TASK-DSN-055 "UHD 4K Typography Matrix" font-weight work — STILL PENDING, UNSTARTED: real
   800/900-weight `.woff2` for Inter and Cairo, fixing faux-bold at ~387 call sites. Deprioritized when the
   dark-mode crisis was discovered; queued behind items 1-2.
4. **Catalog KPI Ribbon** — one of three items in the owner-approved initiative from the ORIGINAL pre-session
   CTP-SESSION-ARC-019 document §7. Still pending, not yet scoped.
5. **ReportsHomeScreen.tsx empty-desert defect** — text-only report tiles, no preview metrics (around line ~72 per
   the original pre-session CTP). Explicitly deferred, still outstanding, not yet scoped.

## 11. Errors, Dead Ends, and Corrected Facts (exhaustive, carried forward in full; do not repeat any of these)

1. The ORIGINAL pre-session CTP-SESSION-ARC-019 (§3, Mod 3) falsely claimed a single shared "instant search
   primitive" across POS/Catalog/Suppliers/Stock. DISPROVED by TASK-DSN-054 Step 0: no such shared file/primitive
   exists; each screen independently implements its own debounced search against its own Zustand store.
2. TASK-DSN-054 found `crates/application/src/use_cases/suppliers.rs:169-172` returning `Ok(Vec::new())`
   unconditionally for empty-query supplier searches (no `suppliers_list` IPC command existed, unlike
   `customers_list`). Correctly left BLOCKED per that task's own CONTINGENCY, not patched around. Fully resolved
   via TASK-BRK-057 -> TASK-AUD-058 -> TASK-DSN-054-C2; entire sub-arc CLOSED.
3. A CRLF/LF drift incident (prior to SESSION-ARC-019) flipped `sqlite_customer_repository.rs` from CRLF to LF,
   producing a spurious 1,375-line diff; reverted. Standing rule: verify line-ending convention before and after
   every edit to an existing file (correctly followed since, e.g. `src/lib/ipc.ts` in TASK-BRK-057).
4. **TASK-REL-061** (prior session) was voided for violating the Self-Contained Task Law (told the executor to
   "copy from a prior section" instead of embedding text directly). Corrected via full-embed reissue. Lesson:
   never reference the executor back to earlier text to "copy from" — always embed literal content directly.
5. The capture pipeline (first built for TASK-DSN-054-C1, reused since) failed multiple times before succeeding:
   dropped closing quotes in `? 'yes':'no'`-style literals caused `SyntaxError`s in CDP `Runtime.evaluate`
   (root-caused via `od -c`); dev-server/profile write races under Vite HMR resolved by moving runtime
   write-artifacts under `node_modules/` (Vite-ignored) and publishing PNGs only after the browser run completed;
   an orphan Vite process (PID 11996) on port 5173 had to be killed before relaunch. ESTABLISHED, REUSABLE pattern:
   launch via `node node_modules/vite/bin/vite.js --port 5173 --strictPort --host 127.0.0.1`; readiness via HTTP
   polling (200 OK, ~250ms interval, ~30s timeout, never a fixed sleep); CDP connect on port 9222; Tauri `invoke`
   stubbed via `Page.addScriptToEvaluateOnNewDocument`, never touching the owner's real database.
6. AUD-015 found the dark-mode crisis was architectural (Section 7), not a two-component bug — the decision to
   build a token foundation first, then migrate incrementally with measured data, was deliberate and has proven
   correct through DSN-055/DSN-056.
7. TASK-DSN-055's root-canvas fix, while auto-fixing 6 of 7 AUD-015 screens, caused ONE newly-introduced, measured
   regression in `ReportsHomeScreen.tsx`'s subtitle (Sections 7/9 item 4-5) — correctly reported as a NEWLY
   INTRODUCED REGRESSION, not silently patched out-of-scope, per that task's own EDGE CASE clause; closed via a
   dedicated follow-up (TASK-DSN-056), not folded into unrelated work.
8. `hermes verify` has been confirmed PERMANENTLY BLOCKED on this repository across >=4 separate attempts, for
   purely environmental/tooling reasons (Section 8). Not a code defect; do not re-attempt without an explicit
   owner/architect ruling on the `pnpm approve-builds` trust question.
9. A `pnpm-lock.yaml` drift occurred ONCE, caused by the executor's OWN prior `hermes verify` probe (`pnpm
   install`), not by any task's real work — discovered and reverted via `git checkout -- pnpm-lock.yaml` before
   TASK-DSN-056's actual work began, reported transparently. No task-caused defect.
10. TASK-DSN-054-C2's Acceptance Criterion 10 imprecisely demanded ALL THREE new regression tests show a
    fail-then-pass pattern; one test (guarding the no-match panel) was, by definition, guarding an already-correct
    pre-existing behavior and therefore could not and should not have failed pre-fix. The executor's actual
    handling (2 of 3 correctly failing-then-passing; 1 correctly already-passing, explicitly explained) was
    reviewed and ACCEPTED as correct. STANDING LESSON (encoded in Section 3): future regression-law task text must
    explicitly distinguish "tests of a newly fixed defect" from "tests guarding pre-existing correct behavior."
11. TASK-CTP-059's execution hit a `write_file` tool quirk (stale-overwrite guard refusing a write despite a
    complete prior read), worked around via delete-then-recreate after a failed shell-heredoc attempt (apostrophes
    breaking shell quoting). Reviewed and accepted as a tooling quirk, not a content defect — net required effect
    correctly achieved.
12. At the review of TASK-CTP-059, the architect briefly, internally mis-estimated a SHA-256 hash's character
    count before stating anything to the owner; a full, careful, character-by-character recount found it to be
    EXACTLY 64 hex characters (valid), and the correct figure was what was actually communicated. Recorded as a
    positive example of Charter §1.4 discipline: verify before asserting, even about one's own fleeting internal
    impressions, especially before relaying any claim to the owner.
13. **At the SESSION-ARC-019 -> SESSION-ARC-020 transfer boundary, the architect incorrectly instructed the
    executor (TASK-CTP-059, then again in TASK-CTP-060) to write the full conversation-transfer CTP content to
    `docs/architecture/CTP-SESSION-ARC-019.md` — the OLD session's filename — instead of recognizing that a
    conversation transfer constitutes a NEW session requiring NEW, correctly-numbered filenames for BOTH the CTP
    and the BOOT document (`CTP-SESSION-ARC-020.md`, `BOOT-SESSION-ARC-020.md`).** This was caught and corrected
    by the owner, not by the architect's own self-review — an explicit lapse in the hyper-strict re-read
    discipline's practical application (the correct convention was inferable directly from the existing
    one-BOOT-and-one-CTP-per-session pattern, and should not have required external correction). Corrected via
    TASK-CTP-061, which produces this document under the correct name and leaves all session-019 artifacts
    untouched. STANDING RULE, now permanent (Section 3, Section 4 item 16): on any future conversation transfer,
    the new session's CTP and BOOT documents are authored under the NEW session number from the first transfer
    artifact, never by overwriting the prior session's files.

## 12. Key Results Snapshot (pointer only — full detail is in Sections 6-9 above)

- Rust tests: 463 passed / 0 failed. Frontend tests (vitest): 600 passed / 0 failed, 59 files.
- All 7 standard quality gates GREEN on the current accepted tree.
- ADR-094 and ADR-095 both authored, implemented, closed. Next free ADR number: 096.
- Dark-mode design-token crisis sub-arc: CLOSED. Suppliers-deferred-states sub-arc: CLOSED.
- One open, non-blocking verification item remains: the "14.48" contrast ratio in TASK-DSN-054-C2's report
  (Section 7).
- Session-transfer naming error (Section 11, item 13): CORRECTED by this document and `BOOT-SESSION-ARC-020.md`.
- Standing queue's next and only immediately-actionable item: Section 10, item 1 — severity-tiered batched
  dark-mode migration, Tier-1 first, each with its own mandatory fresh Step-0 re-measurement.

## 13. Immediate Next Action for SESSION-ARC-020

At Reply 1 of SESSION-ARC-020 (the first reply inside this new conversation window), after re-reading the Charter,
`BOOT-SESSION-ARC-020.md`, and this entire document in full without summarizing, the architect must issue the
FIRST properly-scoped migration task for Section 10, item 1: a Tier-1 (highest-severity-first) batch of the
AUD-015-flagged files, sized to one executor session, with a MANDATORY Step 0 that re-measures the CURRENT
contrast ratio of each specific target file before any edit is made. This task must follow every standing process
rule in Section 3 without exception, in particular: one task per reply, full self-containment, exact VIEWPORT
MATRIX and VISUAL ACCEPTANCE with measurable contrast computations, the Charter's regression-law discipline (with
the Section 3/11-item-10 clarification applied), and CRLF/LF discipline for every existing file touched.
