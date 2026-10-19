# CONTEXT TRANSFER PACKAGE — SESSION-ARC-021

## 0. CHARTER v2.0 STATUS
- Appointment Charter v2.0 is fully binding and active.
- Sovereign rules in effect: Law of Intellectual Honesty, Law of Regression, Law of Exploratory Pre-Verification (ADR-096), prohibition of floating UI elements, Money as i64 centimes, offline-first SQLite integrity.

## 1. SESSION IDENTITY & CONTINUITY
- **Session ID**: SESSION-ARC-021
- **Lineage**: SESSION-ARC-018 → SESSION-ARC-019 → SESSION-ARC-020 → SESSION-ARC-021
- **Session Range**: Replies 1 through 11 of SESSION-ARC-021
- **Baseline Git Commit at Session Open**: `16c3084`
- **Active Git Commit**: `5c11a2e`
- **Working Tree State**: Clean, zero untracked leftovers.
- **Repository Path Ground Truth**: `D:/Dev/Baraka UltraPOS` (STRICT: never use `D:/Baraka Project/Baraka UltraPOS`).
- **Quality Gates Status**:
  - G1 (`cargo fmt --check`): Clean
  - G2 (`cargo clippy -D warnings`): Clean
  - G3 (`cargo test --workspace -j 2`): 463 passed, 0 failed, 4 ignored
  - G4 (`npm run lint`): Clean (exit 0)
  - G5 (`npm run typecheck`): Clean (exit 0)
  - G6 (`npm run build`): Clean (5.52s)
  - G7 (`npx vitest run`): 639 passed across 68 test files (100% green)

## 2. EXECUTIVE SUMMARY & MILESTONES ACHIEVED
In SESSION-ARC-021, the presentation rebuild achieved five fundamental operational milestones driven by rigorous empirical measurement and direct owner feedback:
1. **Dark Mode Contrast 100% Remediated (WCAG AA Compliant)**:
   - Proved Tier-1 Module titles (Catalog, Settings, Stock) already resolved (16.3:1) via body-level design tokens (TASK-DSN-063, commit `93b0d09`).
   - Audited all remaining AUD-015 candidates (TASK-AUD-023, commit `145dcef`).
   - Pre-verified the final 5 Family-2 contrast defects (TASK-AUD-024, commit `586042d`) and remediated them with mathematically proven tokens (TASK-DSN-068, commit `5f8fb86`: POS empty state, TotalsFooter sub-labels, ParkedSalesModal, DiscountModal, DocumentTable thead), achieving dark-mode contrast ratios between 5.71:1 and 6.96:1.
2. **Elimination of POS Dark-Mode Desync**:
   - Diagnosed root cause in Topbar local `useState(false)` unmount reset (TASK-AUD-023).
   - Replaced with a persistent Zustand global theme store (`src/stores/theme-store.ts`) synchronized with `localStorage` and `document.documentElement` (TASK-DSN-065, commit `ea9712f`).
   - Zero desync confirmed across route transitions (Caisse → Stock → Caisse).
3. **Stock Adjustment Table Row Blocker Fixed & RTL Numeric Isolation**:
   - Resolved unit ID missing on row prefill, re-enabling instant confirmation when clicking "Ajuster" from table rows (TASK-DSN-064, commit `897198d`).
   - Isolated RTL numeric rendering (`<bdi dir="ltr">`) ensuring `-1 -> 499` renders in proper chronological order in Arabic.
4. **Universal Instant Live Search Standardized (100% Coverage)**:
   - Built shared instant debounced search primitive (`src/lib/debounced-search.ts`, 175-200ms debounce, monotonic ticket race-condition guard).
   - Upgraded Stock Transfer picker, Purchase Order search, and Return document lookup (TASK-DSN-067, commit `1b56684`). All search inputs across the application now respond live as the user types with zero manual Enter/button dependency.
5. **Configurable Quick-Products Palette (Alt+1..8)**:
   - Pre-verified keyboard map and layout slotting (TASK-AUD-025, commit `359fdd0`).
   - Implemented 8-slot quick-products palette in POS cockpit with `localStorage` persistence and `Alt+1..8` keyboard shortcuts (TASK-DSN-069, commit `5c11a2e`).
6. **Enactment of the Law of Exploratory Pre-Verification (ADR-096)**:
   - Binding rule enacted per owner decree: Absolutely no execution task (DSN/BRK) may be issued without prior read-only exploratory verification (AUD).

## 3. ARCHITECTURE DECISION RECORDS (ADR) LOG
- **ADR-001 through ADR-065**: Historical reconstructed architecture baseline.
- **ADR-066 through ADR-095**: Recorded in repository `docs/adr/INDEX.md`.
- **ADR-096**: *Law of Exploratory Pre-Verification* (Mandatory read-only AUD task before any execution/modification task).
- **ADR-097**: *Persistent Global Theme Architecture* (Centralized Zustand theme store with `localStorage` persistence and immediate root class synchronization, replacing unmount-vulnerable local state).
- **ADR-098**: *Universal Monotonic-Ticket Debounced Search Pattern* (Standard 175–200ms debounce with monotonic sequence ticket counter to eliminate async out-of-order race conditions across all dialog and secondary pickers).
- **ADR-099**: *POS Quick-Products Palette Architecture & Alt+1..8 Shortcut Mapping* (Fast single-keystroke entry for non-barcoded bulk commodities in Algerian quincaillerie).
- **Next Free ADR ID**: ADR-100.

## 4. ROADMAP & TASK LOG (SESSION-ARC-021)
- `TASK-DSN-063` (Commit `93b0d09`): Tier-1 Batch 1 dark mode contrast verification.
- `TASK-AUD-023` (Commit `145dcef`): Audit of theme state, Topbar desync, and AUD-015 defect status.
- `TASK-DSN-064` (Commit `897198d`): Stock table row adjustment fix, picker instant search, RTL isolation.
- `TASK-DSN-065` (Commit `ea9712f`): Global persistent theme store, Topbar desync elimination.
- `TASK-AUD-024` (Commit `586042d`): Inventory of secondary search inputs and AST check of contrast targets.
- `TASK-DSN-067` (Commit `1b56684`): Universal instant search completion (StockTransfer, Purchase, Return).
- `TASK-DSN-068` (Commit `5f8fb86`): Remediation of final 5 dark-mode contrast targets (100% WCAG AA).
- `TASK-AUD-025` (Commit `359fdd0`): Pre-verification of Alt+1..8 shortcuts and POS layout slotting.
- `TASK-DSN-069` (Commit `5c11a2e`): Quick-Products Palette with Alt+1..8 keyboard bindings.
- `TASK-DSN-069-C1`: Commit `docs/architecture/CTP-SESSION-ARC-021.md`.

## 5. CODEBASE TAXONOMY & KEY REUSABLE ASSETS
- Feature modules live under `src/features/` (12 primary modules).
- POS sub-components live under `src/components/pos/`.
- `src/stores/theme-store.ts`: Reactive theme persistence with immediate DOM synchronization.
- `src/stores/quick-products-store.ts`: 8-slot quick-product persistence store with fallback catalog seeding.
- `src/lib/debounced-search.ts`: Monotonic-ticketed debounced search engine.
- `src/lib/money.ts`: Strict integer-centimes formatting with LTR numeric isolation.

## 6. MASTER ERRORS & LESSONS LOG (15 HISTORICAL TRAPS)
- Trap 1: Boot package document conflict -> Solved via committed ground truth.
- Trap 2: Paraphrasing executor outputs -> Solved via exact pastes.
- Trap 3: Output truncation -> Solved via deterministic chunking.
- Trap 4: CRLF/LF drift -> Solved via strict before/after `grep -c $'\r'` verification.
- Trap 5: Dead columns -> Live quantity column is `quantity_base_micro`.
- Trap 6: False shared search claims -> Solved via exact AST inspection.
- Trap 7: Stale hermes tooling -> Permanently disabled.
- Trap 8: Session transfer naming -> Incremented per session.
- Trap 9: Regression law overreach -> Fail-then-pass required for fixes, not pre-existing behavior.
- Trap 10: Capture pipeline instability -> Vite HTTP polling + headless CDP.
- Trap 11: git mv refusal -> Plain mv + git add.
- Trap 12: Drafted vs transmitted confusion -> Ground truth is committed git history.
- Trap 13: Speculative execution task issuance -> Law of Exploratory Pre-Verification (ADR-096).
- Trap 14: Directory path assumption drift -> Mandatory path confirmation via AUD tasks.
- Trap 15: Local component state for global document concerns -> Centralized persistent stores.

## 7. HUMAN CONTEXT & OPERATIONAL MANDATES
- Owner communication: Classical Arabic (فصحى) only, respectful, transparent, business-oriented.
- Executor communication: Staff-engineer level English.
- Hardware store domain: Unstable power, non-barcoded bulk goods, trade credit, speed is commercial life.
- Zero tolerance for regressions, floating buttons, or unisolated numbers.
