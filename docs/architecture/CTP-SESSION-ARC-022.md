# CONTEXT TRANSFER PACKAGE — SESSION-ARC-022

## 0. CHARTER v2.0 STATUS & GOVERNING INVARIANTS
- **Charter Status**: Appointment Charter v2.0 is fully binding, active, and sovereign.
- **Governing Invariants (Untouchable without ADR)**:
  * Offline-first absolute: SQLite WAL, `synchronous=FULL`, `foreign_keys=ON`, `busy_timeout=5000`.
  * Financial integrity strictly in Rust domain: Money = `i64` centimes, quantities/factors = fixed-point 10^4. Zero floats in financial paths.
  * Sale is ONE atomic transaction (idempotency claim, BEGIN IMMEDIATE, sequence allocation, document + items, stock movements, payments, credit ledger, cash movements, audit event, print job, COMMIT).
  * Finalized documents immutable; ledgers append-only via database triggers.
  * Law of Intellectual Honesty (§1.4): zero guessing; unknown facts verified via read-only audit.
  * Law of Regression (§5): every bug fix demonstrated to fail before fix is accepted.
  * Law of Exploratory Pre-Verification (ADR-096): absolutely no execution task (DSN/BRK) without prior read-only audit (AUD).
  * Floating UI prohibition: zero `position: fixed` elements except modal overlays, toasts, and command palette.
- **Required Documents for Incoming Architect**:
  * `docs/architecture/CHARTER-v2.0.md`
  * `docs/architecture/BOOT-SESSION-ARC-022.md`
  * `docs/architecture/CTP-SESSION-ARC-022.md`
  * `docs/adr/INDEX.md`
  * `docs/architecture/design-tokens.md`
  * `audit/AUD-023/REPORT.md`, `audit/AUD-024/REPORT.md`, `audit/AUD-025/REPORT.md`, `audit/AUD-026/REPORT.md`

## 1. SESSION IDENTITY & HISTORICAL LINEAGE
- **Session ID**: SESSION-ARC-022
- **Predecessor Session**: SESSION-ARC-021 (CLOSED)
- **Lineage Chain**: SESSION-ARC-017 → SESSION-ARC-018 → SESSION-ARC-019 → SESSION-ARC-020 → SESSION-ARC-021 → SESSION-ARC-022
- **Historical Audit Evidence**: Unbroken chain verified in `audit/AUD-026/REPORT.md` (commit `5f2bc3a`, 140,025 bytes, 1,911 lines).
- **Baseline Git Commit at Session Open**: `5f2bc3a`
- **Active Git Branch**: `master`
- **Canonical Repository Path**: `D:/Dev/Baraka UltraPOS` (STRICT: `D:/Baraka Project/Baraka UltraPOS` is stale and forbidden).
- **Session Reply Count**: 15 replies in SESSION-ARC-021.

## 2. EXECUTIVE SUMMARY & VISUAL ACCEPTANCE HISTORY
In SESSION-ARC-021, the presentation rebuilding reached operational maturity through five major achievements driven by empirical measurement and direct owner feedback:
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
- **ADR-066 through ADR-093**: Recorded in repository `docs/adr/INDEX.md` (Module refactoring, customer dossier, suppliers list).
- **ADR-094**: *Semantic Design Token Architecture* (Elimination of ad-hoc Tailwind colors in favor of surface and ink semantic tokens).
- **ADR-095**: *Dark Mode Contrast Architecture* (WCAG AA compliant dark surface and ink pairings).
- **ADR-096**: *Law of Exploratory Pre-Verification* (Mandatory read-only AUD task before any execution/modification task).
- **ADR-097**: *Persistent Global Theme Architecture* (Centralized Zustand theme store with `localStorage` persistence and immediate root class synchronization, replacing unmount-vulnerable local state).
- **ADR-098**: *Universal Monotonic-Ticket Debounced Search Pattern* (Standard 175–200ms debounce with monotonic sequence ticket counter to eliminate async out-of-order race conditions across all dialog and secondary pickers).
- **ADR-099**: *POS Quick-Products Palette Architecture & Alt+1..8 Shortcut Mapping* (Fast single-keystroke entry for non-barcoded bulk commodities in Algerian quincaillerie).
- **Next Free ADR ID**: ADR-100.

## 4. ROADMAP & TASK LOG (SESSION-ARC-021)
- `TASK-DSN-063` (Commit `93b0d09`): Tier-1 Batch 1 dark mode contrast verification.
- `TASK-AUD-023` (Commit `145dcef`): Architectural audit of theme state, Topbar desync diagnosis, and AUD-015 defect status.
- `TASK-DSN-064` (Commit `897198d`): Stock table row adjustment confirmation fix, instant search in adjustment picker, and RTL numeric isolation.
- `TASK-DSN-065` (Commit `ea9712f`): Global persistent theme store, Topbar desync elimination.
- `TASK-AUD-024` (Commit `586042d`): Inventory of all secondary search inputs across dialogs/drawers and AST verification of final 5 contrast targets.
- `TASK-DSN-067` (Commit `1b56684`): Universal instant search completion (StockTransferModal, PurchaseOrder, ReturnModal) using shared debounced search utility.
- `TASK-DSN-068` (Commit `5f8fb86`): Remediation of final 5 Family-2 contrast targets, achieving 100% WCAG AA contrast across dark mode.
- `TASK-AUD-025` (Commit `359fdd0`): Pre-verification of Alt+1..8 shortcuts and POS layout slotting.
- `TASK-DSN-069` (Commit `5c11a2e`): Quick-Products Palette with Alt+1..8 keyboard bindings.
- `TASK-DSN-069-C1` (Commit `3d955cb`): Commit `docs/architecture/CTP-SESSION-ARC-021.md`.
- `TASK-AUD-026` (Commit `5f2bc3a`): Consolidate historical CTPs 017 through 021 into `audit/AUD-026/REPORT.md`.
- `TASK-REL-002`: Generate and commit `CTP-SESSION-ARC-022.md` and `BOOT-SESSION-ARC-022.md`.

## 5. CODEBASE TAXONOMY & KEY REUSABLE ASSETS
- Feature modules live under `src/features/` (12 primary modules):
  * `pos/`: Cockpit interface, cart, customer selector, payment, quick products.
  * `catalog/`: Product listing, filtering, search, unit views.
  * `stock/`: Warehouse list, stock levels, adjustment modal, transfer modal.
  * `customers/`: Customer dossier, ledger statements, balance HUD.
  * `suppliers/`: Supplier list, purchases, order creation.
  * `documents/`: Search, details view, reprint, conversion flows.
  * `reports/`: Financial, sales, session, and inventory reporting.
  * `settings/`: Store identity, backup management, printer configuration.
- Shared POS components live under `src/components/pos/` (`Topbar.tsx`, `TotalsFooter.tsx`, `ProductDiscovery.tsx`, `QuickProductPalette.tsx`).
- Core Zustand stores:
  * `src/stores/theme-store.ts`: Reactive theme persistence with immediate DOM synchronization.
  * `src/stores/quick-products-store.ts`: 8-slot quick-product persistence store with fallback catalog seeding.
  * `src/stores/cart-store.ts`: POS atomic cart state, line items, discounts, unit conversion.
  * `src/stores/stock-adjust-modal-store.ts`: Stock adjustment modal state and prefill unit resolution.
- Shared utilities:
  * `src/lib/debounced-search.ts`: Monotonic-ticketed debounced search engine.
  * `src/lib/money.ts`: Strict integer-centimes formatting with LTR numeric isolation.
  * `src/lib/ipc.ts`: 70+ typed invoke wrappers.

## 6. TECHNICAL STATE & ENGINE INVARIANTS
- **Stack**: Tauri v2.11.x, Rust 1.98.1, React 18.3, TypeScript 5.6 strict, Vite 5.4, Tailwind 3.4.
- **Database**: SQLite WAL, synchronous=FULL, foreign_keys=ON, busy_timeout=5000.
- **Migrations**: 15 embedded migrations (`0001` through `0015`). Strictly append-only.
- **Quality Gates Status at Close of Session 021**:
  * G1 (`cargo fmt --check`): Clean
  * G2 (`cargo clippy -D warnings`): Clean
  * G3 (`cargo test --workspace -j 2`): 463 passed, 0 failed, 4 ignored
  * G4 (`npm run lint`): Clean (exit 0)
  * G5 (`npm run typecheck`): Clean (exit 0)
  * G6 (`npm run build`): Clean (5.52s)
  * G7 (`npx vitest run`): 639 passed across 68 test files (100% green)
- **Known Technical Debts**:
  1. Live quantity column is `quantity_base_micro`; dead column `quantity_micro` remains in schema.
  2. Dark-mode toggle in `Topbar.tsx` now backed by global store, but secondary topbars in other views lack direct theme toggles.
  3. FTS5 Arabic normalization engine upgrade pending (search currently LIKE-prefix in sqlite_catalog_repository).
  4. Real thermal printer hardware verification drill remains an open owner gate.
  5. Tax/timbre validation by a certified Algerian professional remains an open owner gate.

## 7. MASTER ERRORS & LESSONS LOG (15 HISTORICAL TRAPS)
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

## 8. HUMAN CONTEXT & OPERATIONAL MANDATES
- Communication to owner: Pure Classical Arabic (فصحى) only, professional, respectful, direct.
- Communication to executor: High-precision technical English (Staff-Engineer level).
- Core domain: Algerian hardware store (Quincaillerie Générale). Fast barcode-less sales (cement, sand, bulk fasteners) are commercially vital.
- Instant live search is mandatory across all search bars in the system.
- Zero tolerance for regressions, floating fixed buttons, or unisolated RTL numbers.

## 9. BOOT INSTRUCTIONS FOR SESSION-ARC-022
- **First Sentence to Say**: "أؤكد استلام حزمة الإقلاع BOOT-SESSION-ARC-022، واستعادة الذاكرة المعمارية الكاملة لنظام Baraka UltraPOS. نحن جاهزون للانطلاق فوراً في الجلسة 022."
- **First Operational Task**: Issue `TASK-AUD-027` (Exploratory Audit of User Management, Authentication Commands, and Role Gating Infrastructure).
