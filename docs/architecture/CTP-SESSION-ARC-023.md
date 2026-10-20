# CONTEXT TRANSFER PACKAGE — SESSION-ARC-023 (Opening Edition — Continues SESSION-ARC-022)

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
  * `docs/architecture/BOOT-SESSION-ARC-023.md`
  * `docs/architecture/CTP-SESSION-ARC-023.md`
  * `docs/adr/INDEX.md`
  * `audit/AUD-027/REPORT.md`, `audit/AUD-028/REPORT.md`, `audit/AUD-029/REPORT.md`, `audit/AUD-030/REPORT.md`, `audit/AUD-031/REPORT.md`
  * `audit/DSN-071/REPORT.md`

## 1. SESSION IDENTITY & HISTORICAL LINEAGE
- **Session ID**: SESSION-ARC-023
- **Predecessor Session**: SESSION-ARC-022 (CLOSED)
- **Lineage Chain**: SESSION-ARC-017 → SESSION-ARC-018 → SESSION-ARC-019 → SESSION-ARC-020 → SESSION-ARC-021 → SESSION-ARC-022 → SESSION-ARC-023
- **Baseline Git Commit at Session Open**: `cf9d39c` (Codebase + CTP-022 committed)
- **Active Git Branch**: `master`
- **Canonical Repository Path**: `D:/Dev/Baraka UltraPOS` (STRICT: `D:/Baraka Project/Baraka UltraPOS` is stale and forbidden).

## 2. EXECUTIVE SUMMARY & ACHIEVEMENTS OF SESSION-ARC-022
In SESSION-ARC-022, the system underwent a major architectural, security, and authorization transformation:
1. **Effective Permission Set Contract (ADR-100 / TASK-BRK-070)**:
   - Upgraded Rust IPC contract: `LoginResponseDto` now carries `permissions: Vec<String>` containing the effective domain permission set (sorted lexicographically).
   - Frontend `src/stores/session-store.ts` upgraded to store permissions and expose `hasPermission(permission: string): boolean`.
   - Established foundation for granular capability gating without compromising Rust authority.
2. **Users Module Tokenization & Proactive Protection (TASK-DSN-071 & DSN-071-C1)**:
   - 100% semantic design tokens applied across `src/features/users/` (zero raw slate/gray classes remaining).
   - WCAG AA contrast compliance verified across 40 measurement points (min 4.51:1, max 17.85:1).
   - Proactive last-admin protection: deactivation button disabled with clear badge and explanatory guidance.
   - Bidi `<bdi dir="ltr">` isolation enforced on all usernames and numeric values.
   - 4 proof screenshots captured via headless CDP at 1280x800 with zero page scroll.
3. **Navigation Shell Granular Gating & Over-Gating Elimination (TASK-DSN-072)**:
   - Completely eradicated crude role-string checks (`isManager`, `isAdmin`) from `src/App.tsx`.
   - Vendeur granted access to Catalog and Stock views; Caissier granted access to Customers and Cash Reports.
   - Access-denied token fallback card rendered on unauthorized direct view switching.
   - Added `src/navigation-permissions.test.tsx` (14 SSR tests).
4. **Customers & Credit Ledger Action-Level Authorization (TASK-DSN-073)**:
   - Replaced all legacy role checks in `src/features/customers/` with `hasPermission`.
   - Enacted the "Disabled with Guidance" UX pattern: Credit limit input and Payment recording buttons remain visible when unauthorized but disabled with localized tooltip guidance.
   - Preserved CRLF line endings on `src/lib/i18n.ts` (2320 lines) and `src/App.tsx` (309 lines).
   - Added `src/features/customers/customers-permissions.test.ts` (13 tests).
5. **Session Reactivity Fix & Live End-to-End Drill (TASK-AUD-031 & AUD-031-C1)**:
   - Diagnosed and fixed Zustand v5 selector reactivity trap in `src/App.tsx`: replaced non-reactive `getSessionToken()` check with reactive `useSessionStore((state) => state.session)`.
   - Executed live headless CDP drill (`audit/AUD-031/drill.mjs`) against running Vite server: achieved **29/29 passing assertions** across all 4 domain roles (Admin, Vendeur, Caissier, Gerant) with zero unhandled console exceptions and zero page scrollbars.

## 3. ARCHITECTURE DECISION RECORDS (ADR) LOG
- **ADR-001 through ADR-065**: Historical reconstructed architecture baseline.
- **ADR-066 through ADR-095**: Recorded in repository `docs/adr/INDEX.md`.
- **ADR-096**: *Law of Exploratory Pre-Verification* (Mandatory read-only AUD task before any execution/modification task).
- **ADR-097**: *Persistent Global Theme Architecture* (Centralized Zustand theme store with `localStorage` persistence).
- **ADR-098**: *Universal Monotonic-Ticket Debounced Search Pattern* (Standard 175–200ms debounce with monotonic sequence ticket counter).
- **ADR-099**: *POS Quick-Products Palette Architecture & Alt+1..8 Shortcut Mapping*.
- **ADR-100**: *Effective Permission Set Contract & Granular UI Gating* (Rust IPC carries `permissions: Vec<String>`, frontend gates via `hasPermission`).
- **Next Free ADR ID**: ADR-101.

## 4. CODEBASE QUALITY GATES STATUS AT CLOSE OF SESSION-ARC-022
- **G1** (`cargo fmt --check`): Clean
- **G2** (`cargo clippy -D warnings`): Clean (1.08s)
- **G3** (`cargo test --workspace -j 2`): 464 passed, 0 failed, 5 ignored
- **G4** (`npm run lint`): Clean (exit 0)
- **G5** (`npm run typecheck`): Clean (exit 0)
- **G6** (`npm run build`): Clean (5.63s, JS 728.47 kB)
- **G7** (`npx vitest run`): 675 passed across 71 test files (100% green)

## 5. REPOSITORY TAXONOMY & KEY ASSETS DELIVERED
- Navigation & Shell: `src/App.tsx` (CRLF, 309 lines, granular permission gating, reactive session subscription).
- Users Module: `src/features/users/` (tokenized, proactive last-admin protection, bidi isolated).
- Customers Module: `src/features/customers/` ("Disabled with Guidance" pattern on credit limits & payment).
- Session Store: `src/stores/session-store.ts` (`permissions: string[]`, `hasPermission`).
- Shared i18n Catalog: `src/lib/i18n.ts` (CRLF, 2320 lines, bilingual FR/AR guidance keys).
- Verification Drills:
  * `audit/DSN-071/capture.mjs` (CDP 4-screenshot token verification).
  * `audit/AUD-031/drill.mjs` (CDP 29-step end-to-end auth & permissions drill).

## 6. MASTER ERRORS & LESSONS LOG (TRAPS REGISTER)
- Traps 1–15: Historical traps documented in previous CTPs.
- **Trap 16: Zustand v5 Reactivity Trap with Component-Level Non-Reactive Accessors**:
  * Root Cause: Calling `getState()` or an accessor like `getSessionToken()` inside a React component body does not create a reactive subscription in Zustand v5. State transitions (such as login/logout) do not trigger component re-render unless a state selector (`useSessionStore((state) => state.session)`) is subscribed.
  * Permanent Resolution: Always subscribe reactively to the actual state property in UI components. Relegated `getSessionToken()` to non-React hooks or SSR static contexts only.

## 7. HUMAN CONTEXT & OPERATIONAL MANDATES
- Owner communication: Classical Arabic (فصحى) only, professional, respectful, direct.
- Executor communication: High-precision Staff-Engineer English.
- Core domain: Algerian hardware store (Quincaillerie Générale). Fast barcode-less sales, trade credit, accurate role delegation.
- Zero tolerance for unisolated numbers, page scrollbars, or untested role gating.

## 8. STANDING QUEUE & ROADMAP FOR SESSION-ARC-023
1. **TASK-AUD-032**: Exploratory audit of `CustomersModule.tsx` module-level role gate (`isManagerRole`) and remaining action buttons in Stock & Catalog (`src/features/stock/`, `src/features/catalog/`).
2. **Batch 2(b) Action Permissions**: Migrate `CustomersModule.tsx` root gate and Stock/Catalog action buttons (Ajustement, Transfert, Nouveau Produit, Nouveau Dépôt) to domain permissions with "Disabled with Guidance".
3. **Batch 2(c) Action Permissions**: Settings & Backup restore permission gating (`SYSTEM_SETTINGS_MANAGE`).
4. **FTS5 Arabic/French Normalization Engine Upgrade** (Search upgrade for Algerian quincaillerie).
5. **Real Thermal Printer Verification Drill** (Open owner gate).
6. **Tax & Legal Validation by Certified Algerian Professional** (Open owner gate).

## 9. BOOT INSTRUCTIONS FOR SESSION-ARC-023
- **First Sentence to Say**: "أؤكد استلام حزمة الإقلاع BOOT-SESSION-ARC-023، واستعادة الذاكرة المعمارية الكاملة لنظام Baraka UltraPOS. نحن جاهزون للانطلاق فوراً في الجلسة 023."
- **First Operational Task**: Issue `TASK-AUD-032` (Exploratory Audit of CustomersModule Root Gate, Stock Adjustments, Transfers, and Catalog Actions).
