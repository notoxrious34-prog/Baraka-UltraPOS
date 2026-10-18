# Boot Package — SESSION-ARC-021

## 1. Executive Continuity & Transfer Authority

- **Session Identifier:** SESSION-ARC-021
- **Session Predecessor:** SESSION-ARC-020 (CLOSED)
- **Historical Continuity Lineage:** SESSION-ARC-018 → SESSION-ARC-019 → SESSION-ARC-020 → SESSION-ARC-021
- **Transfer Authority:** Explicit Owner Directive, transferring to a fresh conversation under the strict mandate: *"يجب أن تبدأ المحادثة الجديدة وكأن الحالية لم تُنقل"* (The new conversation must begin as if the current one was never transferred).
- **Permanent Standing Renumbering Rule (SESSION-ARC-019→020 transfer lesson):** Every conversation transfer mandates a NEW session number. Its CTP and BOOT artifacts are authored under that new number from the very first transfer task. All prior session artifacts (`CTP-SESSION-ARC-020.md`, `BOOT-SESSION-ARC-020.md`, `CTP-SESSION-ARC-019.md`) remain untouched historical records and are NEVER reused or overwritten.
- **Git Ground Truth at Session Open:**
  - Canonical repository path: `D:/Dev/Baraka UltraPOS` (stale path `D:/Baraka Project/Baraka UltraPOS` is strictly forbidden).
  - Code baseline commit: `16c3084` (`docs(architecture): persist CTP-SESSION-ARC-020 and boot package [TASK-REL-062]`).
  - Active documentation commit: `ad5d50e` (`docs(architecture): persist CTP-SESSION-ARC-021 and boot package [TASK-CTP-064]`).
  - Active branch: `master`.
  - Working tree: clean.
- **Intellectual Honesty Grounding (Charter §1.4):** Zero code-level tasks were executed during SESSION-ARC-020. Head code state remains byte-for-byte identical to the close of SESSION-ARC-019. TASK-DSN-063 was drafted during SESSION-ARC-020 Reply 8 but was NEVER transmitted to or executed by the executor; it is transferred here for its first real execution.

## 2. Role, Mandate, & Definition of Done

- **Persona & Authority:** Master Principal Architect & Design Lead, Baraka UltraPOS.
- **Scope of Ownership:** Sole ownership of both the Rust engine (`src-tauri/`) and the entire presentation layer (UI/UX, visual design, documents, print/receipt layouts, typography, design tokens, responsive layouts) for Baraka UltraPOS — an offline-first POS designed for Algerian hardware/quincaillerie retail businesses.
- **Binding Constitution:** Charter v2.0 governs all architectural, engineering, design, and procedural decisions. No personal preference, informal shortcut, or external convention overrides the Charter.
- **Definition of Done (Charter §9):** Completion is governed by five mandatory gates:
  1. *Functional Gate:* Fully working, tested, edge cases handled.
  2. *Visual & UX Gate:* Pixel-precise, theme-aware, zero scroll at target viewport, contrast AA compliant, typography hierarchy respected.
  3. *Realistic-Data Gate:* Tested with long Arabic/French strings, massive category lists, realistic retail amounts.
  4. *Owner Gate:* Real hardware verification (Epson/Xprinter ESC/POS thermal printers, barcode scanners, cash drawers), tax/legal compliance validation. These gates are permanently OPEN internally and can ONLY be closed by the owner.
  5. *Documentation Gate:* ADRs recorded, architecture docs updated, line endings preserved.
- **Absolute Prohibition:** NEVER declare "100%", "production ready", or "feature complete" while any owner gate or unmeasured requirement remains open. Task counts and test counts NEVER define completion.

## 3. Non-Negotiable Operational Laws & Process Rules (Unabridged)

1. **Single-Task Protocol (Charter §2.2, §2.3):**
   - Exactly ONE task per reply.
   - The task specification must reside inside exactly ONE fenced code block.
   - No tasks, sub-tasks, or executor instructions outside that single code block.
2. **Absolute Language Separation (Charter §2.1, §2.4):**
   - Arabic section: Owner-facing exclusively. Modern Standard Arabic (فصحى) only. Absolutely zero dialect, padding, flattery, filler, or pseudo-politeness. Direct, professional, rigorous.
   - English section: Executor-facing exclusively. High-precision Staff-Engineer English. Zero Arabic text inside the executor code block (except literal Arabic UI string fixtures where required).
3. **Full Self-Containment (Charter §2.7, §2.8):**
   - Every task must be 100% self-contained.
   - FORBIDDEN: "see above", "refer to previous section", "copy from earlier task", or delegating file merging, line splicing, or task assembly to the owner or executor.
   - All literal file contents, diffs, token tables, and test cases must be provided inline.
4. **Strict Intellectual Honesty (Charter §1.4):**
   - NEVER guess codebase state, line numbers, test counts, file presence, or conventions.
   - Every factual claim must be backed by pasted terminal output or committed file reads.
   - When uncertain, order a read-only audit task first.
   - Acknowledge mistakes openly, immediately, and fully, without minimization.
5. **Regression Law (Charter §5, clarified in CTP-020 §3):**
   - For any genuine bug fix or defect migration: A test must be proven to FAIL on defective code before the fix, and PASS after the fix. Both terminal outputs must be pasted.
   - For guards protecting pre-existing correct behavior: Pre-fix failure is NOT required, but its status as an existing guard must be explicitly documented.
6. **Repository & Migration Immutability (Charter §7, §12):**
   - Migrations `0001` through `0015` are permanently immutable. Read-only inspection is permitted; editing is strictly forbidden.
   - `.gitattributes` (`* -text`) is permanently immutable.
   - Line-ending preservation: Working tree uses LF. Every task touching existing files must prove before/after line-ending identity (`file` and `grep -c $'\r'`). Spurious CRLF diffs are rejected.
7. **Numeric & Currency Integrity (Charter §4.5):**
   - Money is NEVER floating point. Stored and calculated as `i64` centimes (1 DZD = 100 centimes).
   - Quantities are fixed-point at x10^4 scale (`10000` = 1.0000 unit).
   - In UI: All monetary amounts and numeric quantities must be displayed in Latin digits (`0-9`), wrapped in `<bdi dir="ltr">`, styled with `tabular-nums`.
8. **UI/UX & Design Laws (Charter §3):**
   - Viewport: Standard POS viewport is 1280×720 (minimum). UHD 4K is 3840×2160.
   - Page Scroll: Zero page scroll allowed (`overflow: hidden` on viewport roots). Content-area scroll must be strictly internal.
   - Positioning: `position: fixed` is strictly FORBIDDEN everywhere EXCEPT modal backdrops, toast containers, and command palette.
   - Typography: Font pairings Cairo (Arabic) and Inter (Latin). Real font weights only (no faux bold).
   - Iconography: Phosphor Icons only. ZERO emoji anywhere in DOM (enforced via grep).
   - Contrast: WCAG AA compliance (≥4.5:1 for normal text, ≥3.0:1 for large text ≥18pt/24px or bold ≥14pt/18.67px).
9. **Dependency Protection (Charter §11):**
   - Zero new npm or cargo dependencies permitted without explicit per-task justification and prior authorization.
10. **Task Execution State Law (SESSION-ARC-020 Lesson):**
    - Drafting a task inside an architect reply does NOT constitute execution. A task is only in execution once dispatched to and acknowledged by an executor report.

## 4. Repository Ground Truth & Verification Baseline

- **Repository Path:** `D:/Dev/Baraka UltraPOS`
- **Current HEAD Commit:** `ad5d50e` (docs: persist CTP-021 and boot package)
- **Active Git Tags:** `baseline-engine-pre-redesign`, `v1.0.0-rc.1` through `v1.0.0-rc.9`. (Tag `v1.0.0-rc.10` does NOT exist; tag `v1.0.0` is strictly forbidden until Definition of Done is fully met).
- **All 7 Quality Gates Status (Verified Green at baseline `16c3084` / `ad5d50e`):**
  1. `cargo fmt --check`: Clean (zero diff).
  2. `CARGO_BUILD_JOBS=2 cargo clippy --all-targets --all-features -- -D warnings`: Clean (zero warnings).
  3. `CARGO_BUILD_JOBS=2 cargo test --workspace -j 2`: 463 passed, 0 failed.
  4. `npm run lint`: Clean (ESLint passed with zero warnings/errors).
  5. `npm run typecheck`: Clean (`tsc --noEmit` passed with zero errors).
  6. `npm run build`: Clean (Vite production bundle built in ~6.16s).
  7. `npx vitest run`: 600 passed, 0 failed across 59 test files.
- **Architectural Decision Records (ADR) State (`docs/adr/INDEX.md`):**
  - ADR-066 through ADR-095 all ACCEPTED (ADR-084 SUPERSEDED by ADR-085).
  - Next available free ADR identifier: **ADR-096**.
- **Tooling Environment Status:**
  - `hermes verify` is permanently blocked due to environment mismatch (`ERR_PNPM_IGNORED_BUILDS` on npm repo). Do not attempt to run it.
  - Dev server launch pattern: `node node_modules/vite/bin/vite.js --port 5173 --strictPort --host 127.0.0.1` with HTTP polling readiness verification (~250ms interval, ~30s timeout).
  - Browser capture: Chrome DevTools Protocol (CDP) on port 9222.
  - Database safety: Tauri `invoke` bridge stubbed via `Page.addScriptToEvaluateOnNewDocument`. Live database `%APPDATA%/com.baraka.ultrapos` is NEVER touched.

## 5. Active Design Token System (ADR-094 / ADR-095)

Defined in `tailwind.config.js` and `docs/architecture/design-tokens.md`:

| Semantic Token | CSS Variable Utility | Light Hex | Dark Hex | Light on Canvas Ratio | Dark on Canvas Ratio |
|---|---|---|---|---|---|
| Canvas Surface | `bg-surface-canvas` | `#f8fafc` | `#0b1120` | Base (1.0:1) | Base (1.0:1) |
| Raised Surface | `bg-surface-raised` | `#ffffff` | `#111827` | 1.05:1 | 1.15:1 |
| Sunken Surface | `bg-surface-sunken` | `#eef2f6` | `#0a0f1a` | 1.11:1 | 1.05:1 |
| Primary Ink | `text-ink-primary` | `#0f172a` | `#f1f5f9` | 15.8:1 (PASS) | 17.5:1 (PASS) |
| Secondary Ink | `text-ink-secondary` | `#334155` | `#94a3b8` | 9.6:1 (PASS) | 7.3:1 (PASS) |
| Inverse Ink | `text-ink-inverse` | `#f8fafc` | `#0f172a` | N/A (on dark) | N/A (on light) |
| Muted Ink | `text-ink-muted` | `#64748b` | `#94a3b8` | 4.55:1 (PASS)* | 7.3:1 (PASS) |
| Default Border | `border-line-default` | `#e2e8f0` | `#1e293b` | Structural line | Structural line |
| Strong Border | `border-line-strong` | `#94a3b8` | `#334155` | Boundary line | Boundary line |
| Success Token | `bg-success-bg` / `text-success-fg` | `#dcfce7` / `#16803c` | `#14321f` / `#4ade80` | ≥4.5:1 AA | ≥4.5:1 AA |
| Warning Token | `bg-warning-bg` / `text-warning-fg` | `#fef3c7` / `#b45309` | `#3a2a0a` / `#fbbf24` | ≥4.5:1 AA | ≥4.5:1 AA |
| Danger Token | `bg-danger-bg` / `text-danger-fg` | `#fee2e2` / `#c62828` | `#3b1414` / `#f87171` | ≥4.5:1 AA | ≥4.5:1 AA |
| Info Token | `bg-info-bg` / `text-info-fg` | `#dbeafe` / `#1d4ed8` | `#0f2744` / `#60a5fa` | ≥4.5:1 AA | ≥4.5:1 AA |

*\*Note on ink-muted discrepancy:* `design-tokens.md:23` transcribes light ratio as 4.76:1. Mathematical calculation of `#64748b` on `#f8fafc` yields 4.549:1 (4.55:1). Both exceed 4.5:1 AA threshold. Logged for standalone doc fix.

## 6. AUD-015 Empirical Dark-Mode Severity Data

Root cause: `tailwind.config.js:4` enables `darkMode: "class"`. Dark toggle resides at `src/components/pos/Topbar.tsx:33`. Document `body` has default light `#f8fafc` canvas without a global dark background.

**Empirical Defect Classification:**
- **Family 1 (Light text on undarkened ambient surface):** `dark:text-...` applied without darkened background.
- **Family 2 (Dark text on darkened surface — 1.0:1 Total Invisibility):** Hardcoded dark ink classes (e.g. `text-slate-900`) without `dark:` variant, sitting inside a container that darkens in dark mode.

**AUD-015 Measured Ratios Table (1280×720, dark mode, fr):**
1. POS Topbar title: 16.2:1 (PASS)
2. POS Topbar secondary: 6.8:1 (PASS)
3. POS Cart row title: 15.3:1 (PASS)
4. POS Totals amount: 16.0:1 (PASS)
5. Settings Tab active: 14.8:1 (PASS)
6. Settings Tab inactive: 7.1:1 (PASS)
7. **Settings H1: 1.0:1 (CRITICAL FAIL — Family 2 Defect) → In scope for TASK-DSN-063**
8. **Catalog card title: 1.0:1 (CRITICAL FAIL — Family 2 Defect) → In scope for TASK-DSN-063**
9. Catalog category pill active: 11.2:1 (PASS)
10. Catalog category pill inactive: 5.4:1 (PASS)
11. **Stock H1: 1.0:1 (CRITICAL FAIL — Family 2 Defect) → In scope for TASK-DSN-063**
12. Stock table header: 7.2:1 (PASS)
13. Customers Dossier title: 15.8:1 (PASS — Migrated commit `6b06f93`)
14. Suppliers List title: 14.48:1 (PASS — Migrated commit `6b06f93`)
15. Reports Home card title: 16.3:1 (PASS — Auto-resolved by DSN-055 root-canvas change)
16. Reports card description: 7.5:1 (PASS)
17. Modal Shell title: 15.9:1 (PASS)

## 7. Master Errors & Dead Ends Log (12 Historical Traps)

1. *Boot package document conflict:* Solved via byte-exact committed file ground truth.
2. *AUD-021 executor paraphrasing:* Solved by enforcing zero-paraphrase rule and inline diff outputs.
3. *AUD-022 output length failure:* Solved via 50-line deterministic chunking with line numbers.
4. *CRLF/LF drift in sqlite_customer_repository.rs:* Solved via mandatory before/after line-ending check on all touched files.
5. *count_warehouse_nonzero_levels bug:* Fixed; live column is `quantity_base_micro`.
6. *False "shared instant search primitive" claim:* Disproved; search is view-local in Zustand stores.
7. *hermes verify tooling blockage:* Environment mismatch; permanently disabled.
8. *Session transfer filename error (019→020):* Solved via permanent standing rule: transfers always create NEW session number artifacts from the first task.
9. *Regression Law overreach (Criterion 10 in DSN-054-C2):* Clarified: fail-then-pass is required for newly fixed bugs, NOT for pre-existing correct guards.
10. *Capture pipeline instability:* Solved via stable launch pattern (`node vite --port 5173`), HTTP polling readiness, stubbed Tauri invoke.
11. *git mv refusal on untracked ADR files:* Solved via plain `mv` + `git add`.
12. *Drafted vs. Transmitted Task Misstatement (SESSION-ARC-020):* Solved via standing rule: a drafted task inside an architect reply is NOT issued until dispatched to and acknowledged by an executor report.

## 8. Standing Task Queue

1. **TASK-DSN-063 (Tier-1 Batch 1 dark-mode contrast migration: `CatalogModule.tsx`, `SettingsModule.tsx`, `StockModule.tsx`)** — READY FOR IMMEDIATE DISPATCH (Section 9 below).
2. **Dark-mode toggle desync bug (`src/components/pos/Topbar.tsx:33`)** — Local `useState` desync across navigation; logged debt.
3. **UHD 4K Typography Matrix & Font-Weight Work** — Real 800/900 `.woff2` for Inter/Cairo, eliminating ~387 faux-bold call sites.
4. **Catalog KPI Ribbon** — Owner-approved, awaiting formal scoping.
5. **ReportsHomeScreen empty-desert defect (~line 72)** — Visual polish, deferred.
6. **design-tokens.md ink-muted "4.76" vs "4.55" transcription fix** — Minor doc-only fix.

## 9. Immediate Execution: Full Verbatim TASK-DSN-063 Specification

The first actionable task of SESSION-ARC-021 is reproduced below character-for-character. It must be dispatched immediately upon boot:

```text
TASK-DSN-063 — Tier-1 Batch 1 Dark-Mode Contrast Migration: CatalogModule.tsx, SettingsModule.tsx, StockModule.tsx

CONTEXT (fully self-contained; no external reference required):
- AUD-015 (full audit already committed to the repository) proved three Family-2 dark-mode defects: unmarked dark/light-only ink text rendered on a darkened ambient surface in dark mode, producing a measured foreground:background contrast ratio of 1.0:1 (total invisibility) at:
  1. `src/components/catalog/CatalogModule.tsx` — product card title text, approximately line 450.
  2. `src/components/settings/SettingsModule.tsx` — page H1, approximately line 192.
  3. `src/components/stock/StockModule.tsx` — page H1, approximately line 149.
- Root cause per AUD-015: `tailwind.config.js` line 4 sets `darkMode: "class"`. The dark-mode toggle class is applied at the document root. The three elements above use hard-coded light-mode-only ink utility classes (e.g. a bare `text-slate-900`-style class) with no `dark:` variant, while their nearest backgrounded ancestor switches to a darkened background in dark mode. Result: near-identical foreground and background color, i.e. 1.0:1 contrast.
- ADR-094 and ADR-095 (both ACCEPTED, merged in commit `6b06f93`) already define and have already been consumed elsewhere in the codebase (e.g. `CustomerDossierPane.tsx`, `SupplierListScreen.tsx`, both already migrated and AUD-015-confirmed fixed) a semantic design-token system exposed as Tailwind utility classes, including (at minimum, per `design-tokens.md`):
  - `ink-primary`: `#0f172a` (light) / `#f1f5f9` (dark)
  - `surface-raised`: `#ffffff` (light) / `#111827` (dark)
  - `surface-canvas`: `#f8fafc` (light) / `#0b1120` (dark)
  These tokens are already wired into `tailwind.config.js` as Tailwind utilities (verify the exact utility class names against the current `tailwind.config.js` content at the start of this task — do not assume `text-ink-primary` etc. are the literal strings without checking; if the actual configured class names differ, use the real names and report the discrepancy in the final report, this is not a stop condition).
  NO new token, no new Tailwind config entry, and no new CSS variable may be introduced in this task. Only tokens that already exist in `tailwind.config.js` at the start of this task may be used.
- KNOWN HAZARD — read carefully before doing anything else: a sibling 1.0:1 defect on `ReportsHomeScreen.tsx` line 86 was silently auto-fixed as an unintended side effect of an unrelated prior task (TASK-DSN-055's root-canvas background change), without any dedicated migration task ever being filed or executed for it. This proves AUD-015's line numbers and "still broken" status may be stale relative to current HEAD. Therefore STEP 0 below is MANDATORY, must be executed first, and its full output must appear in the final report BEFORE any source file is edited. Any target element found already passing ≥4.5:1 in Step 0 must be LEFT UNTOUCHED and logged as "ALREADY-RESOLVED-NO-ACTION" — it must never be edited merely to satisfy this task's narrative.

SCOPE-IN (edit permitted ONLY on elements confirmed still-broken by Step 0):
- `src/components/catalog/CatalogModule.tsx` (product card title element, near line 450 — re-locate exactly in Step 0, do not trust the line number blindly).
- `src/components/settings/SettingsModule.tsx` (page H1 element, near line 192).
- `src/components/stock/StockModule.tsx` (page H1 element, near line 149).
- New or updated test file(s) for exactly these three components, following the precedent pattern located via `git show 6b06f93` (see STEP 2).

SCOPE-OUT (must NOT be touched under any circumstance in this task):
- Any other file in AUD-015's Family 1 or Family 2 defect lists (e.g. `CustomerListPane.tsx`, `ParkedSalesModal.tsx`, `DocumentTable.tsx`, `CustomersModule.tsx`, any report component under `src/components/reports/`, any modal component) — these remain separately queued, unscoped tasks.
- `ReportsHomeScreen.tsx` — already resolved as an unrelated side effect; do not edit it and do not add a test for it in this task. If you judge a regression test is genuinely warranted for it, STOP and report that as a recommendation; do not add it unilaterally.
- `tailwind.config.js`, `src/index.css`, any ADR file, `docs/adr/INDEX.md`, `design-tokens.md`.
- The dark-mode toggle logic in `src/components/pos/Topbar.tsx` (separate, already-logged defect, out of scope here).
- Migrations `0001`–`0015` (immutable, not relevant to this task regardless).
- `.gitattributes`.
- Any Rust/`src-tauri` file (this is a frontend-only task).

STEP 0 — MANDATORY FRESH RE-MEASUREMENT (read-only; must complete fully before any edit):
1. Confirm current git HEAD and that the working tree is clean before starting; paste both.
2. Check for and kill any stray process already bound to port 5173; report the check and any kill action explicitly.
3. Start the dev server using the established stable launch pattern: `node node_modules/vite/bin/vite.js --port 5173 --strictPort --host 127.0.0.1`. Confirm readiness via HTTP polling (GET request, ~250ms interval, ~30s timeout, expecting HTTP 200) — never use a fixed sleep. Paste the poll sequence and final success.
4. Using CDP on port 9222 (or equivalent in-page inspection), with the Tauri `invoke` bridge stubbed via `Page.addScriptToEvaluateOnNewDocument` (never touching the real database — synthetic/mock data only), for each of the 3 target screens, at viewport 1280×720, locale `fr`:
   a. Navigate to the screen in LIGHT mode. Locate the exact DOM element matching the description above (product card title / page H1). Extract computed `color` and walk up the DOM to the nearest ancestor with a non-transparent computed `background-color`; extract that value as the true effective background. Compute the WCAG relative-luminance contrast ratio to 2 decimal places.
   b. Repeat identically in DARK mode.
   c. Paste, for every one of the 3 elements, both light and dark raw computed CSS values (hex or rgb) and the resulting numeric ratio — 12 numbers total (3 elements × 2 modes × 2 values, plus 6 ratios).
5. Classify each of the 3 elements: if its DARK-mode ratio is <4.5:1, it is CONFIRMED-STILL-BROKEN and in scope for STEP 1. If its dark-mode ratio is ≥4.5:1, it is ALREADY-RESOLVED and must be excluded from any edit in this task.
6. If the actual DOM structure at any target location differs materially from this task's description (element does not exist, or multiple ambiguous candidate elements exist), STOP here and report the discrepancy in full detail; do not guess which element was intended.

STEP 1 — MIGRATION (only for elements CONFIRMED-STILL-BROKEN in Step 0):
- Replace the hard-coded light-only ink utility class on the target element with the already-existing semantic ink token utility class (verified exact name from `tailwind.config.js`, expected to be `text-ink-primary` or equivalent).
- Verify the relevant backgrounded ancestor already resolves via the existing `surface-raised`/`surface-canvas` semantic background utility in dark mode (per AUD-015's prior finding for sibling migrated files). Do NOT edit any background class. If Step 0 reveals the background itself is still hard-coded and lacks a dark variant (rather than being token-driven as expected), STOP and report back for a scope decision before touching it.
- No property other than the ink color class may be changed on the target element (no spacing, font-size, font-weight, or layout changes).

STEP 2 — REGRESSION TESTS (Charter §5 Regression Law):
- Run `git show 6b06f93` in full and read its entire diff to locate the exact precedent test file(s) and pattern used when `CustomerDossierPane.tsx` / `SupplierListScreen.tsx` were migrated to the ADR-094/095 semantic tokens. Paste the located file path(s) and a representative excerpt of the test structure/assertion style before writing any new test.
- For each of the 3 target files whose element was actually edited in Step 1 (never for elements excluded by Step 0), add one new test following the identical precedent structure, asserting the correct semantic token utility class is present on the target element.
- Regression Law classification: these tests cover a newly-confirmed, genuinely-still-broken defect, not a guard for pre-existing-correct behavior — therefore fail-then-pass proof IS REQUIRED for every such test. For each edited element: (a) run the new test against the code BEFORE the Step 1 edit (i.e., before applying the class change) and paste the FAILING output; (b) apply the Step 1 edit; (c) rerun the same test and paste the PASSING output.
- For any element excluded by Step 0 (ALREADY-RESOLVED), no new test is required in this task; state this explicitly with its measured ratio as justification.

STEP 3 — CRLF/LF PROOF:
- Before any edit: for each of the 3 target `.tsx` files, run `file <path>` and `grep -c $'\r' <path>` (expect 0, confirming LF-only, matching repository convention) — paste output for all 3.
- After all edits: re-run the identical checks on the same 3 files and paste before/after output proving zero line-ending drift.

STEP 4 — FULL 7-GATE RE-RUN (paste the full command and full output or exact final summary line for each — no paraphrasing):
1. `cargo fmt --check`
2. `CARGO_BUILD_JOBS=2 cargo clippy --all-targets --all-features -- -D warnings`
3. `CARGO_BUILD_JOBS=2 cargo test --workspace -j 2` — paste final pass/fail count; expected to remain exactly the same as the last known-green baseline (this is a frontend-only change) — report the exact count and confirm it is unchanged.
4. `npm run lint`
5. `npm run typecheck`
6. `npm run build` — paste the reported build time.
7. `npx vitest run` — paste the final file/test count; expected to be the prior known-green baseline count PLUS the number of new tests added in Step 2 — report the exact new total.

STEP 5 — VISUAL ACCEPTANCE:
- VIEWPORT MATRIX: 1280×720 only (matching AUD-015's own measurement convention; do not expand viewport coverage without a separate task).
- Locale: `fr`.
- Using the same stable dev-server + CDP + stubbed-`invoke` pipeline as Step 0 (synthetic/mock data only, never the real database), for each of the 3 target screens, capture BOTH light and dark mode AFTER all Step 1 edits (and after confirming final state for any Step-0-excluded element) — capture all 6 regardless of whether that specific screen had an edit applied:
  - `audit/DSN-063/screenshots/dsn063-catalog-light.png`
  - `audit/DSN-063/screenshots/dsn063-catalog-dark.png`
  - `audit/DSN-063/screenshots/dsn063-settings-light.png`
  - `audit/DSN-063/screenshots/dsn063-settings-dark.png`
  - `audit/DSN-063/screenshots/dsn063-stock-light.png`
  - `audit/DSN-063/screenshots/dsn063-stock-dark.png`
- Produce `audit/DSN-063/contrast-after.json`: for each of the 3 elements, in both light and dark mode, include exact foreground hex/rgb, exact effective background hex/rgb, computed ratio to 2 decimals, and a pass/fail boolean against 4.5:1 AA.
- Explicitly confirm zero page scroll at 1280×720 for all 3 screens (state the confirmation method used, e.g. `scrollHeight` vs `clientHeight` comparison — do not merely assert it).
- Confirm no `position: fixed` was introduced by this change (not expected, but must be explicitly confirmed, not assumed).
- Run a grep for emoji characters across exactly the 3 edited `.tsx` files; paste the exact grep command used and its output, which must be empty.
- Explicitly state whether any money/quantity digit appears within the 3 edited elements (title/H1 text is not expected to contain any); if none, state this explicitly as "not applicable, no monetary/quantity digits present in edited elements" rather than silently omitting the check.

STEP 6 — REPORT:
- Write `audit/DSN-063/REPORT.md` containing, in full: the Step 0 raw measurements (all values and ratios for all 3 elements, light and dark, before any edit); the classification of each element (CONFIRMED-STILL-BROKEN vs ALREADY-RESOLVED) with justification; for each edited element, a before/after diff excerpt of the exact class-name change; the located precedent test pattern (file path + excerpt) from `git show 6b06f93`; the fail-then-pass proof for every genuinely edited element's new test; the CRLF/LF before/after proof; the full output of all 7 quality gates; the complete final content of `contrast-after.json` inline; and a one-line final status per element, exactly one of `MIGRATED` or `ALREADY-RESOLVED-NO-ACTION`.

COMMIT:
- Stage ONLY: any of the 3 target `.tsx` files that were actually edited, their new/updated test file(s), `audit/DSN-063/REPORT.md`, `audit/DSN-063/contrast-after.json`, and the 6 named PNG screenshots.
- Commit message, exact: `fix(dark-mode): migrate Tier-1 Batch 1 contrast defects — CatalogModule, SettingsModule, StockModule [TASK-DSN-063]`
- Paste `git status --porcelain` before staging (expect clean), `git diff --cached --stat` before committing, the commit command and its output, `git status --porcelain` after committing (expect clean), and `git show --stat HEAD` after committing.
- Prove no file outside the declared set was touched: paste an explicit file list from `git show --stat HEAD` and confirm by inspection that every listed file is one of the files declared above — no other file may appear.

ACCEPTANCE CRITERIA (paste explicit evidence for each, numbered 1–13, matching this list exactly):
1. Step 0 fresh re-measurement completed and pasted in full for all 3 elements, light and dark, BEFORE any edit.
2. Each element correctly classified CONFIRMED-STILL-BROKEN or ALREADY-RESOLVED with numeric proof.
3. Only CONFIRMED-STILL-BROKEN elements were edited; any ALREADY-RESOLVED element explicitly left untouched with stated justification.
4. Only semantic tokens already present in `tailwind.config.js` at task start were used; no new token, config entry, or CSS was introduced.
5. The precedent test pattern from `git show 6b06f93` was located, read in full, and followed exactly in structure/style for any new test(s).
6. Fail-then-pass proof pasted for every genuinely edited element's new test, per the Regression Law.
7. CRLF/LF identity proven before and after for every touched source file.
8. All 7 quality gates re-run and shown green, with exact before/after counts for `cargo test` and `vitest` explicitly stated.
9. All 6 named screenshots captured and present at the specified paths; `contrast-after.json` produced with correct, complete numeric content.
10. Zero page scroll at 1280×720 explicitly confirmed (method stated) for all 3 screens.
11. Zero emoji confirmed via grep across exactly the 3 target files, empty output pasted.
12. Exactly the declared file set was staged and committed; commit message exact; working tree clean before and after; no unrelated file present in the commit.
13. `audit/DSN-063/REPORT.md` written in full per the Step 6 structure.

STOP CONDITIONS (halt and report back instead of improvising past these):
- Step 0 finds a target element's actual DOM structure materially different from this task's description, or ambiguous/multiple candidates exist.
- The semantic ink or background token does not exist at all in `tailwind.config.js` (a mere naming variant is not a stop condition — use the real name and report the variance).
- Any new npm or cargo dependency appears to be needed for any reason.
- Fixing a background class (rather than only the text-color class) appears necessary for any element.

NEXT READY reporting requirement: at the end of your report, state STATE / DONE / REMAINING / NEXT READY exactly as in prior task reports in this session.
```
10. Bootstrap Protocol for the New Conversation
Upon pasting this Boot Package into the new conversation:

Re-affirm role and adherence to Charter v2.0 as Master Principal Architect & Design Lead.
Confirm repository status at commit ad5d50e (or latest commit holding this package).
Immediately issue TASK-DSN-063 (verbatim text in Section 9) as the FIRST operational task of SESSION-ARC-021.
Next mandatory periodic CTP will fall on Reply 10 of SESSION-ARC-021.
