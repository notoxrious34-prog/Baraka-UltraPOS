# Context Transfer Package — SESSION-ARC-021 (Opening Edition — Continues SESSION-ARC-020)

## 0. Chain of Custody

SESSION-ARC-021 continues SESSION-ARC-020, which continued SESSION-ARC-019,
which continued SESSION-ARC-018. SESSION-ARC-020 is now CLOSED, transferred
at its Reply 10. This document simultaneously satisfies two triggers: (a) an
explicit owner-directed ad hoc conversation transfer, and (b) Charter §15's
mandatory periodic CTP, due every 10th reply of the current session.
SESSION-ARC-021 begins at Reply 1; its own next mandatory periodic CTP is due
at Reply 10 of SESSION-ARC-021, counted independently from this session's own
reply counter, not from SESSION-ARC-020's.

Per the owner-mandated permanent standing rule (established at the
SESSION-ARC-019→020 transfer, after an architect filename error caught by the
owner, not by self-review): a conversation transfer creates a NEW session
number; its CTP and BOOT documents are authored under that NEW number from
the very first transfer artifact; the OLD session's CTP/BOOT files
(`CTP-SESSION-ARC-020.md`, `BOOT-SESSION-ARC-020.md`) are left completely
untouched as historical record and are NEVER reused or overwritten.

## 1. Role & Binding Authority

Role: Master Principal Architect & Design Lead, Baraka UltraPOS. Owns both
the Rust engine and the entire presentation layer (UI/UX, visual design,
documents, print design) of Baraka UltraPOS, an offline-first POS for an
Algerian hardware/quincaillerie retailer. Binding authority: Charter v2.0.
The Charter itself is a separate document and must be re-supplied and
re-read in full at the opening of SESSION-ARC-021 — this CTP supplements,
and does not replace, the Charter. Definition of Done is Charter §9
(functional + visual + realistic-data + owner + documentation gates); task
or test counts never define completion. Never declare "100%" or "production
ready" while any owner gate (real thermal-printer verification, tax/legal
validation, hardware checks) or measurement remains open.

## 2. Standing Process Rules (binding every reply; sourced from Charter v2.0
plus CTP-SESSION-ARC-020 §3/§4; carried forward unchanged into SESSION-ARC-021)

- Exactly ONE task per reply, inside exactly ONE fenced code block; Arabic
  (owner-facing) and English (executor-facing) sections NEVER mixed; the
  Arabic section is Modern Standard Arabic (فصحى) only — no dialect, no
  padding, no flattery, no filler.
- Executor instructions: high-precision Staff-Engineer English, fully
  self-contained (Charter §2.8) — ALL literal content (ADR text, token
  tables, CTP content, test fixtures) embedded directly in the task block;
  NEVER "see above" or "copy from a prior section" (TASK-REL-061 was voided
  for exactly this violation; corrected in TASK-REL-061-C1 / TASK-CTP-061).
- Owner's standing triple directive (self-applied every single reply, no
  restatement needed): (1) Throughput — maximum work intensity, method
  delegated to architect: chosen method is precision over batching (small,
  unambiguous, single-objective tasks; unrelated concerns always split into
  separate sequenced tasks; tightly-coupled parts of one true objective may
  share a task). (2) Hyper-strict re-read — re-read the ENTIRE Charter and
  ENTIRE session record from the start, without summarizing or shortening,
  every single time, before every single reply; think deeply before writing
  a word; verify conventions against what documents actually establish
  rather than defaulting to plausible-seeming shortcuts. (3) Excellence —
  work at maximum engineering/architectural/creative/analytical capability
  at all times; act as world-class architect/designer pursuing excellence,
  not mere completion; raise ambition beyond conventional solutions;
  anticipate problems; search for non-obvious improvements; never settle for
  mediocre when better is reachable; balance architecture quality, execution
  strength, design originality, UX, performance, security, scalability,
  maintainability; every decision must have a stated justification; treat
  work as competing with the best global products.
- Charter §1.4 intellectual honesty (hard law): NEVER guess facts about
  codebase/session-state/conventions, even ones that "seem obvious"; every
  fact must come from pasted documents, executor reports, or screenshots;
  when uncertain, order a read-only audit or ask the owner; correct own
  mistakes openly and immediately, without minimizing.
- No feature creep — every feature/scope extension must trace to a genuine
  job-to-be-done and commercial benefit.
- Architect must NEVER ask the owner to perform technical merging, editing,
  or assembly of task text or code.
- `position: fixed` forbidden everywhere EXCEPT modal backdrops, toasts,
  command palette.
- Never touch the owner's live production database or
  `%APPDATA%/com.baraka.ultrapos` under any circumstance — capture/testing
  uses dev server plus synthetic/mock data only.
- No claims of legal/tax compliance or hardware verification — owner gates
  only, never closed internally.
- No git tag `v1.0.0` before Charter §9.2 Definition of Done fully and
  verifiably holds; no tag `v1.0.0-rc.10` exists in the repo despite being
  referenced in a now-corrected prior CTP.
- Money NEVER floating point (i64 centimes); quantities fixed-point at
  x10^4 scale.
- Migrations 0001–0015 immutable (read-only inspection permitted, editing
  forbidden, no exceptions); `.gitattributes` (`* -text`) must never change;
  no CRLF/LF drift permitted — every edit to an existing file must prove
  before/after line-ending identity.
- No new npm or cargo dependency without explicit per-task authorization; if
  genuinely needed, executor must STOP and report why, never add unilaterally.
- Every UI-affecting task requires: exact SCOPE-IN/SCOPE-OUT paths, VIEWPORT
  MATRIX, VISUAL ACCEPTANCE with named screenshots and MEASURABLE contrast
  computations (actual foreground/background token pair + numeric ratio
  pasted, never a bare pass/fail claim), zero page scroll, no
  `position:fixed` outside the three exceptions, contrast ≥4.5:1 WCAG AA (or
  stated large-text 3:1), all money/quantity digits Latin + wrapped in
  `bdi[dir="ltr"]` + `tabular-nums`, zero emoji anywhere in DOM (grep-proven,
  empty match pasted).
- Regression law (Charter §5), CLARIFIED in CTP-020 §3 after a
  SESSION-ARC-019 incident: a bug fix requires a test proven to FAIL on
  defective code before the fix and PASS after (both outputs pasted).
  CLARIFICATION: a test that merely GUARDS an already-correct pre-existing
  behavior (not a newly-introduced fix) is NOT required to fail pre-fix.
  Future task text invoking the regression law must explicitly distinguish
  "tests of a newly fixed defect" (must fail pre-fix) from "tests guarding
  pre-existing correct behavior" (need not fail pre-fix).
- CTP cadence: produced every 10th reply of the CURRENT session (Charter
  §15), plus on ad hoc conversation-transfer triggers. CTPs are authored in
  ENGLISH (permanent owner correction overriding Charter's Arabic-default
  assumption for this specific artifact).
- Standing rule on session transfer (owner-mandated, permanent): when a
  conversation transfer creates a new session, the new session receives a
  NEW session number; its CTP and BOOT documents are authored under that NEW
  number from the very first transfer artifact; the OLD session's CTP/BOOT
  files are left completely untouched as historical record and are NEVER
  reused or overwritten.
- No external web search, no Figma, no external URL, no tool outside this
  project's own repository/Charter/executor reports may be consulted.
- Owner directive: work intensity/density should increase so delivery
  happens faster; method delegated to architect — chosen approach is
  precision/small-task discipline, not large bundled tasks.
- Owner directive (repeated emphasis): before every single reply, read the
  Charter and the entire conversation from the start, carefully, without
  summarizing or shortening; think deeply before writing a word; apply
  professional design creativity and the highest possible quality/perfection
  standard.
- A task is only considered TRANSMITTED/ISSUED once it is actually
  dispatched to and received by the executor — drafting text inside an
  architect reply is NOT equivalent to issuance; see §10 below for the
  concrete incident that produced this clarification.

## 3. External Sources

None. Confirmed as of CTP-SESSION-ARC-020 authoring and reconfirmed through
the entirety of SESSION-ARC-020: no external web search, no Figma, no
external URL, no tool outside this project's own repository, Charter
document, and executor reports has been consulted at any point.

## 4. Git Ground Truth (independently verified: TASK-AUD-020/021/022/022-C1/
022-C2, TASK-REL-062; UNCHANGED since TASK-REL-062 because no code task
executed during SESSION-ARC-020 — see §10)

- Canonical repo: `D:/Dev/Baraka UltraPOS`. Stale copy
  `D:/Baraka Project/Baraka UltraPOS` must never be touched.
- Current HEAD: `16c3084` — `docs(architecture): persist CTP-SESSION-ARC-020
  and boot package [TASK-REL-062]`.
- Full commit chain at HEAD (newest first): `16c3084` → `6b06f93`
  (`feat(ui): dark-mode semantic design tokens, ink-muted refinement, and
  empty-state remediation for suppliers and customers [TASK-DSN-055/ADR-094,
  TASK-DSN-056/ADR-095, TASK-DSN-054-C2, TASK-AUD-058]`, 92 files,
  +5419/-18) → `37b4191` (`feat(engine): suppliers_list read capability
  mirroring customers_list [TASK-BRK-057]`, 7 files, +206/-6) → `ecc1b4d`
  (`docs(architecture): persist CTP-SESSION-ARC-019, generate boot package,
  record ADR-093 [TASK-REL-061-C1]`) → `0812169` (`feat(ui): settings
  command center consolidation and catalog purification [TASK-DSN-053]`) →
  `a7f538c` (tag reference error — known uncorrected mistake in a closed
  historical CTP, do not trust) → `591dc0d` → `e81d80c` → `a9e66bc` →
  `3291bbd` → `1b3ec23` → ... (full older history via
  `git log --oneline -n 40`, not reproduced here; 126+ total commits).
- `git diff ecc1b4d HEAD --stat` (as of end of SESSION-ARC-019/start of
  SESSION-ARC-020): 101 files changed, 6846 insertions(+), 24 deletions(-).
  `.gitattributes` unchanged (verified empty diff). No migration files
  touched.
- Tags present: `baseline-engine-pre-redesign`, `v1.0.0-rc.1` through
  `v1.0.0-rc.9` only. NO `v1.0.0-rc.10` tag exists (contradicts a claim
  embedded in the closed `CTP-SESSION-ARC-019.md`). NO `v1.0.0` tag
  (correctly withheld per Charter §9.2).
- Working tree is CLEAN (`git status --porcelain` empty), confirmed through
  the end of SESSION-ARC-020 (re-confirmed in TASK-AUD-022-C2's report,
  before and after all reads).
- All 7 quality gates independently re-verified GREEN on the full pending
  tree before the TASK-REL-062 commit: `cargo fmt --check` clean; `cargo
  clippy --all-targets --all-features -- -D warnings` clean
  (CARGO_BUILD_JOBS=2); `cargo test --workspace -j 2` = 463 passed / 0
  failed; `npm run lint` clean; `npm run typecheck` clean; `npm run build`
  clean (6.16s); `npx vitest run` (full suite) = 600 passed / 0 failed, 59
  files. These numbers are UNCHANGED as of this CTP's authoring — no code
  task executed during SESSION-ARC-020.
- AGENTS.md exists at repo root (5538 bytes), states "Executor Constitution
  v2.1," supersedes `CONSTITUTION_OF_EXECUTOR.md` v1.0.0 (archived, no
  longer at root) and previous AGENTS.md; full text at
  `docs/architecture/EXECUTOR-CONSTITUTION-v2.md` (12153 bytes).
- ADR index (`docs/adr/INDEX.md`, 102 lines, read in full and re-verified
  verbatim during SESSION-ARC-020 via TASK-AUD-022-C2): confirmed to contain
  the four rows appended by TASK-REL-062 for ADR-092, ADR-093, ADR-094,
  ADR-095 (all ACCEPTED); "Next free id: ADR-096" confirmed present at line
  102. Full content reproduced in §5 below.
- `fiscal.timbre_enabled` setting (ADR-092), starter-pack system (7 tracked
  files including `src/assets/fixtures/quincaillerie_starter_300.csv`), and
  the dual-track safe-deletion guards are ALL committed and verified present
  at HEAD — features from SESSION-ARC-018.
- `hermes verify` tooling PERMANENTLY ENVIRONMENTALLY BLOCKED (confirmed ≥4
  attempts): its Vite recipe bootstraps via `pnpm install`, aborting with
  `ERR_PNPM_IGNORED_BUILDS` (esbuild@0.21.5 needs `pnpm approve-builds`).
  Repo is npm-based; approving build scripts would alter `package.json`
  trust policy — outside any task's authority, NOT authorized. Tooling/
  environment mismatch, not a code defect. Do not re-attempt without
  explicit owner/architect ruling.

## 5. ADR Index State (verbatim-verified, `docs/adr/INDEX.md`, 102/102 lines
read in full via `git show HEAD`, zero gaps, TASK-AUD-022-C2)

Structure: §1 ADR-001..015 (MASTER-ARCHITECTURE-v1.0.0.md baseline, all
ACCEPTED as architectural baseline, realization status varies, see
MASTER-ARCHITECTURE-ERRATA.md). §2 ADR-016..061 (referenced-only, original
texts lost to a previous architect's context; 17 of 46 ids actually
referenced in the repo, 29 unreferenced and reserved, never to be reused).
§3 ADR-062..065 (from a previous architect's package SESSION-BRK-011, not
independently re-verified: safe-restore protocol, crash/power matrix,
in-transaction idempotency, printing diagnostics/capture).

§4 ADR-066..095 — Architect (Charter v2.0), all ACCEPTED (except ADR-084
SUPERSEDED by ADR-085):

ADR-066 Viewport baseline and window policy · ADR-067 Single top bar; no
floating action buttons · ADR-068 Tax regime IFU: no VAT in v1 (validation
gate OPEN) · ADR-069 Locales fr + ar, Latin digits always · ADR-070 Output
formats: thermal 58, thermal 80, A4 · ADR-071 Deployment topology: single
device, single instance · ADR-072 Catalog migration from Dolisoft export
(amended by ADR-075/077) · ADR-073 Canonical repository location · ADR-074
Destructive-action confirmation pattern · ADR-075 Catalog import scope:
products only · ADR-076 Isolated data directory override (BARAKA_DATA_DIR)
· ADR-077 Product entry priority, French name with optional Arabic alias,
generic import/export · ADR-078 Information architecture and navigation ·
ADR-079 Roles in use and manager approval in place · (ADR-080, ADR-081
absent — numbering gap, no files, unexplained, not actioned) · ADR-082
Debt-receipt numbering reuses customer_ledger sequence · ADR-083 Debt-
receipt printing uses a fully separate rendering path · ADR-084 Persisted
CTP files authored in Arabic, superseding ADR-080 — SUPERSEDED by ADR-085 ·
ADR-085 English-language CTP persistence re-ratified by owner, superseding
ADR-084 — ACCEPTED · ADR-086 Release Candidate v1.0.0 Packaging and
Production Approval · ADR-087 Artisan & wholesale pricing tiers, Margin
Guard, official brand identity · ADR-088 Single-instance application mutex
and quincaillerie starter fixture · ADR-089 Data-Retention Invariants,
In-Place Upgrade Safety, and Downgrade Policy · ADR-090 Data Exchange Hub,
POS Fast-Sale Optimization, and Responsive Catalogue Layout · ADR-091
Dual-Width (58mm/80mm) ESC/POS Thermal Pipeline and Font-Shaping Calibration
· ADR-092 Dual-Track Safe Deletion and Configurable Fiscal Stamp · ADR-093
Settings Command Center and Catalog Purification · ADR-094 Dark Mode via
Semantic Design Tokens · ADR-095 Ink-Muted Token Tier Refinement.

**Next free id: ADR-096.**

## 6. Design Token System (ADR-094/095; source `docs/design/design-tokens.md`,
33/33 lines verbatim-verified via `git show HEAD`, TASK-AUD-022-C2; root
cause per `audit/AUD-015/REPORT.md`, see §7)

Consumed via Tailwind utilities (`bg-surface-canvas`, `text-ink-primary`,
`border-line`, `bg-success-bg`, `text-success-fg`, …). Ratios computed by
relative luminance (same method AUD-015/DSN-055/DSN-056 used).

| Token | Light | Dark | Pair (ratio → threshold) | Light | Dark |
|---|---|---|---|---|---|
| surface-canvas | #f8fafc | #0b1120 | ink-primary on canvas (≥7) | 17.06 ✓ | 17.19 ✓ |
| surface-raised | #ffffff | #111827 | ink-primary on raised (≥7) | 17.85 ✓ | 16.19 ✓ |
| surface-sunken | #eef2f6 | #0a0f1a | — (structural only) | — | — |
| ink-primary | #0f172a | #f1f5f9 | see above | — | — |
| ink-secondary | #334155¹ | #94a3b8 | on canvas (≥4.5) | 9.90 ✓ | 7.34 ✓ |
| ink-secondary | #334155¹ | #94a3b8 | on raised (≥4.5) | 10.40 ✓ | 6.92 ✓ |
| ink-inverse | #f8fafc | #0f172a | — (on accent/primary fills) | — | — |
| line-default | #e2e8f0 | #1e293b | — (decorative) | — | — |
| line-strong | #94a3b8 | #334155 | — (decorative/icons) | — | — |
| success-bg / fg | #dcfce7 / #16803c | #14321f / #4ade80 | fg on bg (≥4.5) | 4.57 ✓ | 8.00 ✓ |
| warning-bg / fg | #fef3c7 / #b45309 | #3a2a0a / #fbbf24 | fg on bg (≥4.5) | 4.51 ✓ | 8.30 ✓ |
| danger-bg / fg | #fee2e2 / #c62828 | #3b1414 / #f87171 | fg on bg (≥4.5) | 4.60 ✓ | 5.86 ✓ |
| info-bg / fg | #dbeafe / #1d4ed8² | #0f2744 / #60a5fa | fg on bg (≥4.5) | 5.49 ✓ | 5.93 ✓ |
| ink-muted | #64748b³ | #94a3b8 | on canvas (≥4.5) | 4.76 (file's own claim — SEE OPEN ITEM 2 below) | 7.34 ✓ |

¹ Draft was #475569; adjusted to #334155 (existing slate-700 body copy) to
keep light rendering pixel-identical. ² Draft #2563eb measured 4.24:1 (below
AA); darkened to #1d4ed8 (same hue, 5.49:1). ³ TASK-DSN-056: light pinned
exactly to resolved `text-slate-500` (zero visual change); dark computed
independently (ADR-095).

Rule (verbatim, design-tokens.md): "a text token may only be used on its
paired surface token (or a surface it was measured against); decorative
tokens (`line-*`) are never body copy."

Components migrated to date (3 of ~76 flagged files): `CustomerDossierPane.tsx`
(teaching/empty panel, DSN-055, dark 1.05→17.19); `SupplierListScreen.tsx`
(no-match panel, DSN-055, dark 1.18→7.34; extended in DSN-054-C2 with
zero-database teaching state + default-list-on-empty-query state, tokens
only); `ReportsHomeScreen.tsx` (card title auto-fixed by root-canvas change
to 16.3 dark; subtitle line 76 migrated in DSN-056: 3.96 FAIL → 7.34 PASS
dark, light 4.55 unchanged).

OPEN ITEM 1 (non-blocking, carried forward unresolved since SESSION-ARC-019):
TASK-DSN-054-C2 cited a contrast ratio of "14.48" for an element in the new
Suppliers teaching state, not matching any value in this table. Must be
clarified (exact fg/bg token pair) the next time `SupplierListScreen.tsx` is
touched — most plausibly an icon/illustration color against
`surface-raised`.

OPEN ITEM 2 (NEW, discovered SESSION-ARC-020 Reply 8, non-blocking, flagged
not corrected): `design-tokens.md:23` states ink-muted-on-canvas (light) =
**4.76**. Independent relative-luminance computation using the file's own
declared hexes (#64748b on #f8fafc) yields **4.549**, matching BOTH this
CTP-chain's own figure (4.55, carried since CTP-SESSION-ARC-020 §7) AND
AUD-015's own empirical measurement (`audit/AUD-015/REPORT.md:130`: "reports
| P 'Analyses d'activité…' | #64748b on #f8fafc | 4.55 / 4.5 | PASS
(borderline)"). Two independent sources agree at 4.55; the committed
design-tokens.md "4.76" appears to be a transcription error. Both values
clear the 4.5 AA threshold, so this is non-blocking — but 4.55 sits only
0.05 above the minimum AA threshold, too tight to leave permanently
unverified. QUEUED as its own small, independent, read-only-then-single-line
-fix task (§8 item 6 below); not to be bundled into any other task.

## 7. Full AUD-015 Severity Data (verbatim-verified, `audit/AUD-015/REPORT.md`,
172/172 lines read in full via `git show HEAD`, zero gaps, TASK-AUD-022-C2 —
this is the authoritative severity-tier source for all future dark-mode
migration scoping; cite directly, do not re-request, do not re-derive)

Root cause (verbatim): `tailwind.config.js:4`: `darkMode: "class",`. Toggle
site `src/components/pos/Topbar.tsx:33`:
`document.documentElement.classList.toggle("dark", dark);` — local
`useState` default `false`; the Topbar (and therefore the toggle) renders
ONLY inside the POS view. `src/index.css` global base (lines ~94-97):
`body { background-color: #f8fafc; color: #0f172a; }` — NO `.dark` variant,
no `color-scheme`, no CSS custom properties for surfaces. No `.dark`
selector, `[data-theme]`, or `prefers-color-scheme` exists anywhere under
`src/` (grep, excl. tests: zero hits). DEFINITIVE CONCLUSION: the codebase
had NO mechanism by which the root/body background changed in dark mode;
`body` stayed `#f8fafc`/`#0f172a` in both themes; only per-component `dark:`
utilities changed anything, ad hoc, with no shared surface/token contract.
Static inventory: repo-wide grep for `dark:` found 76 source files, 685
occurrences (tests excluded).

SUSPECT family 1 (light text, undarkened ambient surface) — full table:

| File:lines | Evidence |
|---|---|
| `CustomerDossierPane.tsx:75,78` | PROVEN (C1): 1.05/1.18:1 — MIGRATED (DSN-055) |
| `SupplierListScreen.tsx:199,205` | PROVEN (C1): 1.18:1 — MIGRATED (DSN-055) |
| `CustomerListPane.tsx:145` | PROVEN (AUD-015): debtor balance `dark:text-amber-400` #fbbf24 on #f8fafc = 1.6:1 FAIL — NOT YET MIGRATED |
| `CustomersModule.tsx:186` | deleteNote `dark:text-emerald-300` on ambient (module root has no `dark:bg`) — NOT YET MIGRATED |
| `SupplierListScreen.tsx:371` | deleteNote `dark:text-emerald-300`, same pattern — NOT YET MIGRATED |
| `CustomerDossierPane.tsx:42` | HudCard `dark:text-amber-400`, card root line 38 has NO `dark:bg` (sibling StockOverviewPane HudCard DOES — copy-paste miss) — NOT YET MIGRATED |
| `CustomerDossierPane.tsx:128`, `ProductEditModal.tsx:488`, `StarterPackBanner.tsx:238` | delete/destructive buttons: `dark:text-red-200` with only `dark:hover:bg-*`, NO resting dark bg, resting ≈1.3:1 — NOT YET MIGRATED |
| `LowStockReport.tsx:100` | `dark:text-red-300` table cells, ReportsHomeScreen root (line 10) has no `dark:bg` — NOT YET MIGRATED |
| `CreditOutstandingReport.tsx:134` | `dark:text-red-300` cells, same ambient root — NOT YET MIGRATED |
| `StockMovementReport.tsx:64-65`, `GrossMarginReport.tsx:50-51` | `signedClass`/`marginClass` red/emerald-700→300 helpers, same ambient root — NOT YET MIGRATED |
| `CategoryPillStrip.tsx:99` | manage button `dark:text-slate-400` on transparent resting bg; CatalogModule roots (392, 856) have no `dark:bg` — NOT YET MIGRATED |
| `ProductDiscovery.tsx:184` | scan-success banner `dark:text-emerald-400` on `bg-emerald-500/10` (no dark variant) ≈2:1; plus unmarked empty text ≈3.75:1 FAIL — NOT YET MIGRATED |
| `TotalsFooter.tsx:25-33` | footer paints own `dark:bg-slate-900` (line 24) but rows stay `text-slate-500` unmarked ≈3.7:1 est. (needs measurement) — NOT YET MIGRATED |
| `DiscountModal.tsx:61,73` | labels `text-slate-500` unmarked inside ModalShell dark card ≈3.7:1 est. — NOT YET MIGRATED |

SUSPECT family 2 (inverse: unmarked dark text on darkened surface) — full
table:

| File:lines | Evidence |
|---|---|
| `CatalogModule.tsx` product cards (~450) | PROVEN: product-name SPAN #0f172a on #0f172a = 1.0:1 FAIL — NOT YET MIGRATED, TIER-1 BATCH 1 TARGET |
| `ReportsHomeScreen.tsx:86` card | PROVEN: card title "Marges brutes" #0f172a on #0f172a = 1.0:1 FAIL — AUTO-FIXED as a side effect of DSN-055's root-canvas token change (now 16.3:1 dark per §6 above); no dedicated task was ever filed for this specific site |
| `SettingsModule.tsx` h1 | PROVEN: "Paramètres & Configuration" #0f172a on #0f172a = 1.0:1 FAIL (root darkens line 192, h1 unmarked) — NOT YET MIGRATED, TIER-1 BATCH 1 TARGET |
| `StockModule.tsx` h1 | PROVEN: "Stock & Dépôts" 1.0:1 FAIL, same pattern (root darkens line 149) — NOT YET MIGRATED, TIER-1 BATCH 1 TARGET |
| `ParkedSalesModal.tsx:204,211,223,224,229` | text nodes with NO `dark:text` at all inside ModalShell dark card (only `dark:` in file is a border, line 220) — NOT YET MIGRATED |
| `DocumentTable.tsx` (5× `dark:bg`, 0× `dark:text`) | unmarked rows; parent DocumentsModule root darkens (line 96) — static finding, needs measurement, NOT YET MIGRATED |
| `CustomerDossierPane.tsx:68` | icon `dark:text-slate-500` on ambient — decorative, cosmetic only, NOT YET MIGRATED |

UNKNOWN (bg resolved to another file, pre-verified, likely fine): `AboutTab.tsx`
→ SettingsModule.tsx:192 dark root + measured 8.4:1 PASS; `BackupSettingsTab.tsx`
→ same root; `UserListTable.tsx` → UsersModule.tsx:61 dark root; `StockOverviewPane.tsx:98`
→ StockModule.tsx:149 dark root; `CartTable.tsx` → PosShell.tsx:54 /
CartPanel.tsx:55; all 12 ModalShell consumers → ModalShell.tsx:12
`dark:bg-slate-900` verified.

SAFE (representative, full list in audit/AUD-015/REPORT.md lines 94-106):
SafeDeleteModal, CategoryManageModal, ProductCreateModal, ProductEditModal
content, ProductImportModal, OperationsMenu, ProductCreateForm,
StarterPackBanner cards, UnitSelectPopover, CreditHud, CustomerTierBadge,
DataExchangeMenu, UsersModule, StockModule/DocumentsModule roots, CartPanel,
ProductDiscovery list/cards, StockOverviewPane HudCard, DocumentFilterBar,
App.tsx:155, document-helpers.ts/user-helpers.ts/stock-helpers.ts/
settings-helpers.ts (paired triples), KeyFooter (theme-independent).

Empirical contrast sample (dark, fr, 1280×720, user-toggle path) — full
18-row table from audit/AUD-015/REPORT.md §b, reproduced:

| Screen | Element | fg on bg | Ratio/threshold | Verdict |
|---|---|---|---|---|
| pos | P "Catalogue vide — ajoutez…" | #64748b on #0f172a | 3.75/4.5 | FAIL |
| pos | BUTTON "Caisse" (active nav) | #020617 on #f59e0b | 9.39/4.5 | PASS |
| settings | H1 "Paramètres & Configuration" | #0f172a on #0f172a | 1.0/4.5 | FAIL |
| settings | P "Version 1.0.0…" (About) | #e2e8f0 on #334155 | 8.4/4.5 | PASS |
| catalog | SPAN "Savon de Marseille" | #0f172a on #0f172a | 1.0/4.5 | FAIL |
| catalog | SPAN "Ciment CPJ 45" | #0f172a on #0f172a | 1.0/4.5 | FAIL |
| catalog | H2 "Catalogue des Produits" | #0f172a on #f8fafc | 17.06/4.5 | PASS |
| stock | H1 "Stock & Dépôts" | #0f172a on #0f172a | 1.0/4.5 | FAIL |
| customers | H1 "Clients & Crédit" | #0f172a on #f8fafc | 17.06/4.5 | PASS |
| customers | P "Amine Benali" (row) | #0f172a on #f8fafc | 17.06/4.5 | PASS |
| customers | BDI "1 250,50 DZD" (debtor bal.) | #fbbf24 on #f8fafc | 1.6/4.5 | FAIL |
| suppliers | H2 "Fournisseurs" | #0f172a on #f8fafc | 17.06/4.5 | PASS |
| suppliers | TD "Fournisseur Test Alpha" | #0f172a on #f8fafc | 17.06/4.5 | PASS |
| suppliers | TD "2 500,00 DZD" | #0f172a on #f8fafc | 17.06/4.5 | PASS |
| reports | H2 "Rapports" | #0f172a on #f8fafc | 17.06/4.5 | PASS |
| reports | SPAN "Marges brutes" (card) | #0f172a on #0f172a | 1.0/4.5 | FAIL (now auto-fixed, see above) |
| reports | P "Analyses d'activité…" | #64748b on #f8fafc | 4.55/4.5 | PASS (borderline) |

6 of 7 screens reproduced a defect; suppliers-populated was clean. Stock
low-stock ROWS were not reachable (mock data-path error); stock H1 finding
stands, row-level sampling deferred.

AUD-015's own "Design input for next task" (verbatim): "A single root-level
change (dark body background) would fix family 1 but BREAK every family-2
site that currently relies on inheriting dark body text on a darkened card —
and vice versa. The inventory above is the blast radius. Cheapest structural
fix consistent with the data: give the four ambient-root views
(CustomersModule, SupplierListScreen section, CatalogModule,
ReportsHomeScreen) the same bg-slate-50 dark:bg-slate-900 root the other
four views already have, then re-measure; family-2 sites need per-element
dark:text-* additions." This raw suggestion is SUPERSEDED by the superior,
already-ratified ADR-094/095 semantic-token approach — no new `dark:` pairs,
no new raw Tailwind colors, use `surface-*`/`ink-*` tokens exclusively.

## 8. Standing Queue (fixed order, re-affirmed every reply)

### Item 1 — Batched severity-tiered dark-mode migration (ACTIVE — THIS IS
THE IMMEDIATE NEXT ACTION OF SESSION-ARC-021, ready to dispatch without
further scoping work)

Tier-1 Batch 1 (3 files, all PROVEN 1.0:1, highest possible severity) was
fully scoped at SESSION-ARC-020 Reply 8 as TASK-DSN-063. CRITICAL FACT (see
§10 below): TASK-DSN-063 was drafted but NEVER transmitted to the executor;
zero execution occurred. It is reproduced here IN FULL, VERBATIM, UNCHANGED,
so SESSION-ARC-021 can dispatch it immediately as its first action with zero
re-derivation:

--- BEGIN TASK-DSN-063 (verbatim, unchanged, never executed) ---

TASK-DSN-063 — Tier-1 Dark-Mode Contrast Migration (Batch 1 of N):
CatalogModule.tsx / SettingsModule.tsx / StockModule.tsx —
Eliminate the three PROVEN 1.0:1 total-invisibility defects

ROLE: You are the executor for Baraka UltraPOS under Executor Constitution
v2.1 (AGENTS.md, repo root). This is a READ-then-WRITE task: it begins with
a mandatory read-only re-measurement step and only edits files if that
re-measurement proves an edit is still required. Canonical repo: D:/Dev/
Baraka UltraPOS. NEVER touch D:/Baraka Project/Baraka UltraPOS (stale
copy). NEVER touch the owner's live production database or
%APPDATA%/com.baraka.ultrapos. All capture/testing uses the dev server plus
synthetic/mock data only, via a Tauri invoke stub — never a real backend.

WHY: Source: audit/AUD-015/REPORT.md (git-verified, 172/172 lines, read in
full — reproduced in §7 above). Root cause: src/index.css had a hardcoded
`body { background-color: #f8fafc; color: #0f172a; }` with NO dark variant,
despite tailwind.config.js:4 declaring `darkMode: "class"`. TASK-DSN-055
(ADR-094) introduced a semantic design-token system to replace ad hoc
dark: utilities; ~73 of 76 flagged files remain unmigrated. This task is
the FIRST batch of that backlog, Tier-1 = highest proven severity first.
The three target defects (family-2 pattern: unmarked/inherited ink text on
a component that already paints its own dark background, producing
identical fg/bg colors):
  1. CatalogModule.tsx product cards (~line 450): product-name SPAN
     #0f172a on #0f172a = 1.0:1 FAIL (card has dark:bg-slate-900, name
     unmarked)
  2. SettingsModule.tsx H1 (~line 192 root): "Paramètres & Configuration"
     #0f172a on #0f172a = 1.0:1 FAIL (root darkens line 192, h1 unmarked)
  3. StockModule.tsx H1 (~line 149 root): "Stock & Dépôts" 1.0:1 FAIL, same
     pattern (root darkens line 149)
CRITICAL WARNING — numbers may already be stale: AUD-015 predates
TASK-DSN-055/056. A sibling defect in the same tier, ReportsHomeScreen.tsx's
card title (also originally 1.0:1 FAIL), was SILENTLY auto-fixed to 16.3:1
dark by TASK-DSN-055's root-canvas/token change, with no dedicated task
ever filed. Do NOT assume any of these three files' numbers are still
1.0:1. Re-measure first. If one or more is already fixed, make NO edit to
that file — report it as already-correct with proof.

Canonical token table (source: docs/design/design-tokens.md, git-verified,
33/33 lines): surface-canvas #f8fafc/#0b1120 (ink-primary on canvas ≥7:
17.06/17.19); surface-raised #ffffff/#111827 (ink-primary on raised ≥7:
17.85/16.19); ink-primary #0f172a/#f1f5f9. Consumed as Tailwind utilities:
bg-surface-canvas, bg-surface-raised, text-ink-primary — these resolve
automatically via CSS custom property, no dark: prefix needed or wanted on
a token utility. Rule (verbatim): "a text token may only be used on its
paired surface token (or a surface it was measured against)."

Regression Law (Charter §5, clarified CTP-020 §3, verbatim): "a bug fix
requires a test proven to FAIL on defective code before the fix and PASS
after (both outputs pasted). CLARIFICATION: a test that merely GUARDS an
already-correct pre-existing behavior (not a newly-introduced fix) is NOT
required to fail pre-fix."

Established capture pipeline (reusable, do not redesign): launch via `node
node_modules/vite/bin/vite.js --port 5173 --strictPort --host 127.0.0.1`;
readiness via HTTP polling (200 OK, ~250ms interval, ~30s timeout, never a
fixed sleep); CDP connect on port 9222; Tauri invoke stubbed via
Page.addScriptToEvaluateOnNewDocument, never touching the real database;
dark mode toggled via the real Topbar control `button[aria-label="Thème"]`,
verify `documentElement.classList.contains('dark') === true` before
measuring; contrast via relative luminance (WCAG formula); write runtime
artifacts under node_modules/ (Vite-ignored); publish PNGs only after the
run completes; kill any orphan Vite process on port 5173/9222 before
relaunch.

SCOPE-IN: exactly the three source files named above, paths TO BE CONFIRMED
in Step 0 (do not assume a path — locate it). At most one new or extended
test file per edited source file, mirroring the existing co-located test
convention used for CustomerDossierPane.tsx/SupplierListScreen.tsx/
ReportsHomeScreen.tsx. New evidence directory audit/DSN-063/ (REPORT.md,
screenshots/, scripts/ — mirror audit/DSN-056/ structure).
docs/architecture/CTP-SESSION-ARC-020.md is NOT touched by this task.

SCOPE-OUT: any of the other ~71 remaining AUD-015-flagged files (Tier-2+,
future tasks); Topbar.tsx:33 toggle desync bug; UHD 4K Typography Matrix
font-weight work; Catalog KPI Ribbon; ReportsHomeScreen.tsx empty-desert
defect; SupplierListScreen.tsx's open "14.48" item (none of the 3 targets
is SupplierListScreen.tsx); design-tokens.md's "4.76" vs "4.55" ink-muted
discrepancy (flagged separately, out of scope here); migrations 0001-0015;
.gitattributes; any new npm/cargo dependency (STOP and report if genuinely
needed, never add unilaterally).

STEP 0 — Locate, dump current state, re-measure (READ-ONLY, mandatory,
first): 0.1 `git status --porcelain`, paste, confirm clean. 0.2 Locate
exact current paths: `git ls-files | grep -E
'(CatalogModule|SettingsModule|StockModule)\.tsx$'`; expect exactly one
match per name; if more than one, STOP and disambiguate using the AUD-015
line-number context (product-card region ~450 Catalog, H1 region ~192
Settings, H1 region ~149 Stock) before proceeding, reporting your
reasoning. 0.3 For each of the 3 files, dump the current full content of
the relevant region via `git show HEAD:<path> | nl -ba -w4 -s': ' | sed -n
'<start>,<end>p'` (window wide enough to show the element, its immediate
parent, and the nearest ancestor applying a bg-*/dark:bg-* class) — paste
every dumped line inline, fully verbatim, no paraphrasing, no "see above".
Confirm or correct the ~450/~192/~149 line numbers against what you
actually find. 0.4 Launch the capture pipeline. For EACH of the 3 elements,
in BOTH light and dark mode, at 1280×720, measure and paste the exact
foreground hex, exact background hex, and computed contrast ratio — 6
measurements minimum. ADDITIONALLY: locate and read in full the ADR titled
"Viewport baseline and window policy" (`git ls-files docs/adr | grep -i
066`, then `git show HEAD:<that path>`). If it mandates additional required
viewport(s) beyond 1280×720, measure and screenshot those too, citing the
exact ADR text. If it mandates only a single fixed window size, state that
explicitly and proceed with 1280×720 only. 0.5 Compare Step 0.4's measured
numbers against AUD-015's original 1.0:1 claim for all three. For each,
state explicitly: "STILL BROKEN, ratio=X" or "ALREADY FIXED by a prior
task's side effect, ratio=X, no edit needed." Do not proceed to Step 1 for
any element found already fixed.

VIEWPORT MATRIX: 1280×720 mandatory for every measurement/screenshot (the
baseline used by every capture task to date: AUD-015, DSN-054-C1, DSN-055,
DSN-056, DSN-054-C2). Add further viewport(s) ONLY if Step 0.4's reading of
ADR-066 mandates them, citing the exact text.

STEP 1 — Migrate (only elements Step 0 proved still broken): a. Replace
the hardcoded/unmarked ink utility class (whatever Step 0.3 actually
reveals) with `text-ink-primary`. b. Replace the enclosing card's or root
view's bg-*/dark:bg-slate-900 pairing (whatever Step 0.3 actually reveals)
with the ONE semantic surface utility matching its structural role:
bg-surface-raised if a card/panel, bg-surface-canvas if a top-level view
root. State explicitly which was chosen for each of the 3 and why, based on
what Step 0.3's dump actually shows. c. Delete any now-redundant
dark:bg-*/dark:text-* utility the token class supersedes — no dead raw-
Tailwind classes left behind. d. Add NO new tokens, NO new raw Tailwind
palette colors anywhere. e. Zero emoji introduced (grep-verified Step 5).
f. Zero position:fixed introduced.

STEP 2 — Re-measure after edit: same method, same elements, same
viewport(s), both themes. Paste every post-fix measurement. State the delta
vs. Step 0.4's pre-fix numbers for each edited element. Expected order of
magnitude: ≈17.85/16.19 on raised, ≈17.06/17.19 on canvas — but paste the
ACTUAL measured numbers, do not assert the table's reference values.

STEP 3 — Regression tests (apply the clarified Regression Law precisely):
3.1 Locate the exact precedent test pattern: `git show 6b06f93 -- <test
files touching CustomerDossierPane/SupplierListScreen/ReportsHomeScreen>`.
Quote the exact precedent assertion pattern inline. 3.2 For EACH element
you actually edited (and only those — an already-correct element gets no
new test, per the Regression Law clarification), add one test mirroring
that exact precedent pattern. 3.3 For each new test: paste the FAILING run
against the pre-Step-1 code (e.g. via `git stash` to restore pre-edit
state, run, show failure, `git stash pop`) AND the PASSING run against the
fixed code. Both outputs mandatory — this is a newly-fixed defect, not a
guard of pre-existing correct behavior.

STEP 4 — CRLF/LF discipline: for every edited file, paste line-ending
detection BEFORE and AFTER the edit, demonstrating identity.
`.gitattributes` must show an empty diff.

STEP 5 — Full gate re-run: `cargo fmt --check`; `cargo clippy
--all-targets --all-features -- -D warnings` (CARGO_BUILD_JOBS=2); `cargo
test --workspace -j 2`; `npm run lint`; `npm run typecheck`; `npm run
build`; `npx vitest run` (full suite). Compare every count against the
last confirmed baseline (Rust 463/0, vitest 600/0 across 59 files) and
state the exact delta (expect +N tests from Step 3, 0 regressions
elsewhere). Paste a repo-wide emoji grep across the DOM-rendering files
touched, showing an empty match.

VISUAL ACCEPTANCE: screenshots named exactly
audit/DSN-063/screenshots/dsn063-catalog-light.png,
dsn063-catalog-dark.png, dsn063-settings-light.png, dsn063-settings-dark.png,
dsn063-stock-light.png, dsn063-stock-dark.png (plus any ADR-066-mandated
extra viewports, named with explicit viewport suffix). Plus
audit/DSN-063/screenshots/contrast-after.json (fg/bg/ratio per element,
mirroring DSN-055/DSN-056's format) and audit/DSN-063/scripts/ holding the
capture script (mirror DSN-056's verify-ink-muted.mjs naming convention,
renamed appropriately). Every pasted contrast number must be an actual
measured fg/bg hex pair + numeric ratio. Confirm zero page scroll at each
tested viewport for each affected screen.

ACCEPTANCE CRITERIA: 1. Step 0 completed first, in full, before any edit;
exact current paths/line ranges confirmed or corrected vs. AUD-015. 2.
Pre-fix contrast numbers pasted for all 3 elements, both themes, explicit
"still broken"/"already fixed" verdict per element. 3. No edit to any
already-fixed element. 4. Every edit uses ONLY existing ADR-094/095
semantic tokens — zero new tokens/raw colors, zero remaining redundant
dark:bg-*/dark:text-* on edited elements. 5. Post-fix contrast ≥4.5:1 in
BOTH themes for every edited element, actual measured numbers pasted. 6.
Regression tests added ONLY for elements actually edited; precedent
pattern located/quoted first; fail-then-pass proof pasted for each. 7.
CRLF/LF identity proven before/after for every edited file;
.gitattributes diff empty. 8. All 7 gates green; exact before/after
counts pasted; delta explained. 9. Zero emoji (grep-proven, empty match
pasted); zero new position:fixed; zero page scroll at every tested
viewport. 10. Screenshots and contrast-after.json delivered exactly as
named. 11. SCOPE-OUT list fully honored. 12. audit/DSN-063/REPORT.md
written, structured like audit/DSN-056/REPORT.md, containing every
measurement, file:line, gate result, and explicit statement of which of
the 3 targets were edited vs. already-correct. 13. Any scope extension
discovered mid-task (e.g. a 4th file sharing the exact same defect) must
be FLAGGED in the report at the moment of discovery, not silently
included and not silently omitted.

REPORT FORMAT: audit/DSN-063/REPORT.md, in order: Step 0 full dump and
pre-measurements; Step 1 edits made (with justification per file for
surface-raised vs. surface-canvas choice) or explicit no-edit statements;
Step 2 post-measurements with deltas; Step 3 precedent quote + test code +
fail/pass proof; Step 4 CRLF proof; Step 5 full gate output; Visual
Acceptance screenshot list + contrast-after.json content; final
STATE/DONE/REMAINING summary in the mechanical format used in prior audit
reports.

--- END TASK-DSN-063 ---

### Item 2 — Fix dark-mode toggle desync bug

`src/components/pos/Topbar.tsx:33` (view-local React state desyncs from
applied theme after navigation) — logged debt, not scheduled.

### Item 3 — Original TASK-DSN-055 "UHD 4K Typography Matrix" font-weight work

STILL PENDING/UNSTARTED: real 800/900-weight `.woff2` for Inter and Cairo,
fixing faux-bold at ~387 call sites. Deprioritized behind the dark-mode
crisis.

### Item 4 — Catalog KPI Ribbon

Owner-approved (from original pre-session CTP-019 §7), not yet scoped.

### Item 5 — ReportsHomeScreen.tsx empty-desert defect

Text-only report tiles, no preview metrics, ~line 72 — deferred, not yet
scoped.

### Item 6 (NEW, queued at this transfer) — design-tokens.md ink-muted "4.76"
vs. measured "4.55" discrepancy

Read-only re-verification of the exact relative-luminance computation plus
a single-line correction to `docs/design/design-tokens.md:23`. Non-
blocking, small, independent task — see §6 Open Item 2 above.

## 9. Completed Task Log

### SESSION-ARC-019 (closed; carried forward unchanged)

TASK-DSN-053 (settings command center consolidation, catalog purification)
→ AUD-015 (read-only dark-mode contrast audit, this CTP's §7) →
TASK-DSN-054/C1 (capture pipeline established; CustomerDossierPane.tsx/
SupplierListScreen.tsx no-match panels migrated) → TASK-DSN-055/ADR-094
(semantic design-token system introduced) → TASK-DSN-056/ADR-095
(ink-muted token tier refinement; ReportsHomeScreen.tsx subtitle migrated)
→ TASK-BRK-057 (suppliers_list read capability, mirrors customers_list:
trait `list_suppliers` in ports.rs mirroring `list_customers`; SQLite impl
`SELECT {SUPPLIER_COLUMNS} FROM suppliers WHERE (?1 = 0 OR is_active = 1)
ORDER BY name`; use case `ListSuppliersUseCase`, gated by
`Permission::SupplierManage`; Tauri command `suppliers_list` at
src-tauri/src/lib.rs:279; frontend `ipcListSuppliers` in src/lib/ipc.ts,
zero call sites at delivery; Rust tests 462→463) → TASK-AUD-058
(read-only: confirmed `Permission::SupplierManage` gate matches the
existing supplier search path exactly; `SupplierPaymentRecord` is a
separate unrelated permission; role mapping shows no asymmetry — safe to
wire) → TASK-DSN-054-C2 (wired `ipcListSuppliers` into
SupplierListScreen.tsx, closing default-list-on-empty-query and
zero-database teaching state; balance figures via `<bdi dir="ltr">…</bdi>`
+ tabular-nums; vitest 597→600, +3 tests, 2 correctly fail-then-pass, 1
correctly already-passing guarding pre-existing behavior; one declared
scope extension: 3 new i18n keys FR+AR) → TASK-CTP-059 → TASK-CTP-060
(both superseded by filename error, see §10 Errors entry 8) → TASK-CTP-061
/ TASK-REL-061-C1 (persisted CTP-SESSION-ARC-020.md and
BOOT-SESSION-ARC-020.md under the CORRECT new-session filenames, recorded
ADR-093, left CTP-SESSION-ARC-019.md untouched).

### SESSION-ARC-020 (closed at this transfer)

TASK-AUD-020 (read-only git forensic audit; resolved the Boot-package vs.
"PART 2" document conflict — the pasted "PART 2" was a mislabeled
near-duplicate of the already-committed CTP-SESSION-ARC-019.md; the true
SESSION-ARC-020 document existed only uncommitted on disk at the time,
matching the BOOT PACKAGE's continuity note) → TASK-AUD-021 (requested
verbatim file content; executor FAILED by paraphrasing instead of quoting;
REJECTED per Charter §9.3, re-issued) → TASK-AUD-022 (re-requested via
`git show HEAD:<path>`; executor FAILED again, falsely claiming content
was "pasted verbatim in the preceding turn" when no such content existed;
root-caused to an output-length constraint, not willful non-compliance;
task mechanically redesigned into fixed 50-line `nl -ba -w4 -s': '` chunks
with an explicit ban on "pasted above"/"see earlier"/"unchanged from
above") → TASK-AUD-022-C1 (mechanically-chunked request, file 1 of 4:
docs/architecture/CTP-SESSION-ARC-020.md — SUCCEEDED: 444/444 lines
delivered in 9 inline chunks, verified gapless) → TASK-AUD-022-C2
(mechanically-chunked request, files 2-4 of 4: audit/AUD-015/REPORT.md
[172/172, 4 chunks], docs/design/design-tokens.md [33/33, 1 chunk],
docs/adr/INDEX.md [102/102, 3 chunks] — SUCCEEDED: all three delivered
gaplessly; git status clean before/after confirmed) → architect derived
Tier-1 Batch 1 (§8 Item 1 above) directly from the verified AUD-015 data
and drafted TASK-DSN-063 at Reply 8 — DRAFTED BUT NEVER TRANSMITTED (see
§10 Errors entry 12; zero execution occurred) → duplicate-report incident
at Reply 9 (owner resent an identical copy of the TASK-AUD-022-C2 report;
architect correctly declined to re-verify or re-issue a duplicate task) →
owner corrected the record at Reply 10 ("لم يتم ارسال TASK-DSN-063") →
TASK-CTP-064 (this task): persist this CTP (CTP-SESSION-ARC-021.md) and
BOOT-SESSION-ARC-021.md, opening SESSION-ARC-021.

## 10. Errors & Dead Ends (full log, carried forward verbatim, appended)

1. Boot-package document conflict (SESSION-ARC-020 opening issue, fully
   resolved) — see §9 TASK-AUD-020 summary above. Also found:
   CTP-SESSION-ARC-019.md itself contains one embedded factual error
   (claims tag v1.0.0-rc.10 exists on commit a7f538c; `git tag -l` proves
   no such tag exists, confirmed twice). Flagged as a known, uncorrected
   error in a historical (closed, immutable) CTP file — not to be fixed
   retroactively, just not trusted as fact going forward.

2. TASK-AUD-021 executor failure (non-compliance, caught and corrected).
   Executor returned paraphrased summaries instead of actual file content
   — a direct violation of explicit "no paraphrasing" instructions.
   Architect rejected per Charter §9.3 (evidence law).

3. TASK-AUD-022 second executor failure (worse form of the same defect).
   Executor falsely claimed content was "pasted verbatim in the preceding
   turn" when no such content existed anywhere. Root cause diagnosed as a
   practical single-response output-length constraint, not willful
   non-compliance. Mechanical fix: fixed 50-line chunks via `nl -ba -w4
   -s': '`, mandatory inline presence of every chunk, explicit ban on
   "pasted above"/"see earlier"/"unchanged from above". WORKED — verified
   across TASK-AUD-022-C1 and TASK-AUD-022-C2, both gapless and complete.

4. CRLF/LF drift incident (pre-SESSION-ARC-019, carried-forward lesson).
   Editing sqlite_customer_repository.rs once flipped CRLF to LF, producing
   a spurious 1,375-line diff; reverted. Standing rule: verify line-ending
   convention before and after every edit to an existing file.

5. `count_warehouse_nonzero_levels` bug (pre-SESSION-ARC-019, fixed,
   verified). Previously queried deprecated `quantity_micro` instead of
   live `quantity_base_micro`, letting warehouses with real stock bypass
   the deletion guard. Fixed in commit a9e66bc. Independently re-verified
   at HEAD in TASK-AUD-020 §6.c.

6. TASK-DSN-054 (SESSION-ARC-019) disproved a false claim in an even-older
   pre-session CTP document — that a single shared "instant search
   primitive" exists across POS/Catalog/Suppliers/Stock. FALSE: no such
   shared file/primitive exists; each screen independently implements its
   own debounced search against its own Zustand store.

7. `hermes verify` tooling permanently blocked (confirmed ≥4 attempts) —
   see §4 above for full detail. A one-time pnpm-lock.yaml drift caused by
   this probe was found and reverted before TASK-DSN-056 began real work.

8. Session-transfer filename error (SESSION-ARC-019→020 boundary,
   architect's own error, corrected). The architect incorrectly instructed
   the executor (TASK-CTP-059, then TASK-CTP-060) to write the new
   session's transfer CTP content into the OLD session's filename
   (CTP-SESSION-ARC-019.md) instead of a new, correctly-numbered file.
   Caught by the owner, not by self-review — an explicit lapse in
   hyper-strict re-read discipline. Corrected via TASK-CTP-061
   / TASK-REL-061-C1. This produced the standing session-renumbering rule
   now in §2 above — the same rule this very CTP (SESSION-ARC-021) is
   correctly applying from its first artifact.

9. TASK-DSN-054-C2's Acceptance Criterion 10 was imprecisely worded
   (SESSION-ARC-019) — demanded ALL THREE new regression tests show
   fail-then-pass; one test (guarding an already-correct pre-existing
   behavior) correctly could not and should not have failed pre-fix.
   Executor's actual handling (2 of 3 fail-then-pass, 1 already-passing
   with explicit explanation) was reviewed and ACCEPTED. Produced the
   Regression Law clarification now in §2 above.

10. Capture pipeline instability (SESSION-ARC-019, TASK-DSN-054-C1), now a
    reusable established pattern — see §7/Item 1 pipeline description
    above for the final, working form (dropped-quote ternary SyntaxErrors
    root-caused via `od -c`; write races resolved by moving runtime
    artifacts under node_modules/; orphan Vite process on port 5173 killed
    before relaunch).

11. `git mv` refused during TASK-REL-062 when renaming the two ADR files,
    because they were untracked (no tracked git history to preserve) —
    `fatal: not under version control`. Executor correctly substituted
    plain `mv` + `git add`, disclosed transparently; net effect identical.

12. TASK-DSN-063 drafted-but-never-transmitted (SESSION-ARC-020, Replies
    8-10). The architect fully drafted TASK-DSN-063 inside a Reply-8
    fenced code block, but it was never actually dispatched to or received
    by the executor. At Reply 9 the owner resent an identical duplicate of
    a PRIOR executor report (TASK-AUD-022-C2's report, not a new one for
    TASK-DSN-063), which momentarily led the architect to describe
    TASK-DSN-063 as "issued, awaiting executor report" — an imprecise
    characterization, corrected at Reply 10 when the owner explicitly
    stated the task was never sent ("لم يتم ارسال TASK-DSN-063"). LESSON,
    now a standing rule (§2 above, final bullet): drafting task text inside
    an architect reply is NOT equivalent to transmitting it to, or it
    entering execution by, the executor; the architect must never assume a
    task has entered execution absent an explicit executor report
    acknowledging receipt of that SPECIFIC task. NET EFFECT: zero
    code-level work occurred during the entirety of SESSION-ARC-020 beyond
    the three read-only audits (AUD-020/021/022 and its two corrected
    chunked sub-tasks); the git ground truth in §4 above is therefore
    byte-identical to its state at the end of SESSION-ARC-019/TASK-REL-062.
    TASK-DSN-063's full text is carried forward unchanged into
    SESSION-ARC-021 (§8 Item 1 above) for immediate dispatch as that
    session's first action.

## 11. Key Results Snapshot

- Git HEAD: `16c3084`. Working tree clean. 463/0 Rust tests. 600/0 vitest
  tests across 59 files. All 7 gates green except `hermes verify`
  (permanently environmentally blocked, not a code defect). These numbers
  are IDENTICAL to their state at the close of SESSION-ARC-019 — no code
  task executed during SESSION-ARC-020.
- ADR-094/ADR-095 closed/ACCEPTED; next free ADR id: 096; numbering gap at
  080/081 remains unexplained, not investigated, no action required.
- Both SESSION-ARC-019 sub-arcs (dark-mode token crisis; suppliers-deferred
  -states) remain CLOSED.
- `audit/AUD-015/REPORT.md` (172 lines), `docs/design/design-tokens.md` (33
  lines), and `docs/adr/INDEX.md` (102 lines) have now ALL been read
  verbatim, in full, gaplessly, git-verified (TASK-AUD-022-C1/C2) — full
  content reproduced in §5-§7 above. No further re-reading of these three
  specific files is required before dispatching TASK-DSN-063.
- Tier-1 Batch 1 (3 files: CatalogModule.tsx, SettingsModule.tsx,
  StockModule.tsx, all PROVEN 1.0:1) is fully scoped as TASK-DSN-063 (§8
  Item 1), ready for immediate dispatch, UNEXECUTED.
- Two small, independent, non-blocking items are newly queued: the open
  "14.48" SupplierListScreen.tsx ratio clarification (carried forward,
  §6 Open Item 1) and the design-tokens.md "4.76"/"4.55" ink-muted
  discrepancy (new, §6 Open Item 2 / §8 Item 6).
- Session-transfer naming convention was applied correctly from the first
  artifact of this transfer (this CTP itself), per the standing rule in §2.

## 12. Immediate Next Action (binding)

At Reply 1 of SESSION-ARC-021, after a full fresh re-read of the Charter
and of this entire CTP (per the standing hyper-strict re-read directive,
§2 above), the architect must dispatch TASK-DSN-063 EXACTLY as reproduced
in §8 Item 1 above — re-verifying (not re-deriving) its path/line-number
citations against current repo state as that task's own Step 0 already
requires — as the first substantive task of the new session. No other
queue item may be advanced ahead of it.
