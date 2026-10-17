# CTP-SESSION-ARC-019

**Continuity Transfer Package**
**Chain:** SESSION-ARC-018 → SESSION-ARC-019
**Status:** ACTIVE HANDOFF
**Classification:** Internal Architecture Document (English technical register)
**Supersedes:** None (first issuance of this CTP; prior TASK-REL-061 attempt voided for incompleteness)

---

## 0. Document Purpose and Scope

This Continuity Transfer Package (CTP) is the authoritative, non-summarized record transferred from SESSION-ARC-018 to SESSION-ARC-019. It exists to satisfy Charter §15 (no summarization across session boundaries) and Charter §16 (no shortcuts in CTP packages). Every fact in this document is carried forward in full fidelity from the ARC-018 session record. The executor and the next-session architect must treat this file as ground truth for repository state, completed work, in-flight work, constraints, and unresolved items as of the close of SESSION-ARC-018.

This CTP does not introduce new scope. It transcribes and organizes what was already established in SESSION-ARC-018. Any new scope for SESSION-ARC-019 is confined to Section 7 (Pending Work), which reflects decisions already made and Owner-approved before the session boundary.

---

## 1. Repository Identity at Handoff

- Project: Baraka UltraPOS (offline-first point-of-sale system for Algerian hardware/quincaillerie retail)
- Working tree status at handoff: clean
- HEAD commit at handoff: `0812169` — `feat(ui): settings command center consolidation and catalog purification [TASK-DSN-053]`
- Last annotated release tag: `v1.0.0-rc.10` on commit `a7f538c`
- Frozen migration range: `0001` through `0015` (immutable, `* -text` in `.gitattributes`, CRLF-preserving)
- Primary stack: Rust (Tauri backend, SQLite via rusqlite, WAL mode, `synchronous=FULL`) + React/TypeScript frontend (Vite, Vitest, ESLint, TypeScript strict)

---

## 2. SESSION-ARC-018 Commit Ledger (Full, In Order)

1. `a9e66bc` — `feat(engine): dual-track safe deletion, warehouse update, and configurable fiscal stamp [TASK-BRK-074]`
2. `e81d80c` — `feat(ui): safe deletion modals across entities and configurable fiscal stamp toggle [TASK-DSN-051]`
3. `591dc0d` — `feat(catalog): starter pack lifecycle with dynamic detection and safe dual-track uninstall [TASK-DSN-052]`
4. `a7f538c` — `docs(release): publish v1.0.0-rc.10 offline handbook and release notes [TASK-REL-060]` (tagged `v1.0.0-rc.10`, annotated)
5. `0812169` — `feat(ui): settings command center consolidation and catalog purification [TASK-DSN-053]`

No commits beyond `0812169` exist on the mainline as of this handoff. `TASK-REL-061` (original, defective) produced no commit. This re-issuance (`TASK-REL-061-C1`) is the first commit-producing action following `0812169`.

---

## 3. Completed Work Inventory — The Five Owner Customization Specifications (All Completed, Released in rc.10)

### Mod 1 — Warehouse Selector Default
`ProductCreateModal.tsx` now presents a warehouse selector that defaults to the active depot, falling back to `DEP-01` when no other depot is marked active. Implemented in `TASK-BRK-074` / `TASK-DSN-051`.

### Mod 2 — Safe Dual-Track Deletion
Engine-level guard logic: an entity (product, warehouse, customer) is hard-deleted only if it has zero references in transactional history; otherwise it is soft-deactivated (`is_active = 0`). Three protections are enforced at the repository layer:
- `DEP-01` depot can never be deleted or deactivated, unconditionally.
- Any warehouse carrying non-zero stock (`quantity_base_micro != 0` in the live stock table, migration 0003) cannot be deleted.
- Any customer with `balance != 0` cannot be deleted or deactivated.
UI-side: confirmation modals across Products, Warehouses, and Customers correctly reflect which track (hard-delete vs. soft-deactivate) will occur before the user confirms, implemented in `TASK-DSN-051`.

### Mod 3 — Universal Instant Search
Instant, debounced search implemented uniformly across: POS Cockpit, Catalog, Suppliers, and Stock Overview. Shares a common search primitive to guarantee consistent latency and highlighting behavior.

### Mod 4 — Starter Pack Lifecycle (300-Item Hardware Catalog)
CSV-driven provisioning of a 300-item starter hardware catalog into a target depot, with dynamic detection of installed/uninstalled state and a safe dual-track uninstall path (hard-delete items with no transactional history, soft-deactivate items that have been sold/moved), producing a summary report of the outcome. Implemented in `TASK-DSN-052`.

### Mod 5 — Configurable Algerian Fiscal Stamp (Timbre)
The 1% fiscal stamp (timbre) applied to eligible transactions is now toggleable via a SQLite-persisted setting `fiscal.timbre_enabled`, exposed through a Settings UI control. Default state and toggle behavior verified against existing invoice/ticket generation paths. Implemented across `TASK-BRK-074` (engine) and `TASK-DSN-051` (UI).

**Verification status:** All five specifications passed their respective gates (cargo test, clippy, vitest, eslint, tsc, vite build) as recorded in SESSION-ARC-018 and were included in the `v1.0.0-rc.10` release build.

---

## 4. Completed Work Inventory — Release Candidate v1.0.0-rc.10 Packaging

- `release-v1.0.0/GUIDE-UTILISATION.html` updated with new §§12–14 covering the five Owner Customizations, built at 33,825 bytes, SHA-256 `b2faabf63e3a31d68a96ee8376c42e024441752e16e666db8de252934f3bffad`.
- NSIS installer `Baraka UltraPOS_1.0.0_x64-setup.exe` built with `CARGO_BUILD_JOBS=2`, 222,308,475 bytes, SHA-256 `5512cb43106ce911a1f31659df9000b9f41011f01b59c66005efd04730c0df5d`.
- Delivery archive `Baraka-UltraPOS-v1.0.0-rc.10-Delivery.zip`, 222,342,588 bytes, SHA-256 `06a020d56fba5814f6d4a558af6f9d6a5c9d3369b7d20d6d396f4788dd73b4b0`.
- Published and committed in `a7f538c`, tagged `v1.0.0-rc.10` (annotated tag object on that commit).

---

## 5. Completed Work Inventory — Master UI/UX UHD 4K Reorganization (Phase 1 of 2, Owner-Approved)

### 5.1 Settings Command Center Consolidation (Completed in `0812169`)
`SettingsModule.tsx` was reorganized from its prior flat/ad-hoc structure into a Two-Tier Command Center: 4 top-level groups containing 8 tabs in total, consolidating all administrative surfaces that were previously scattered, including:
- Rayons & Catégories (category/aisle management) — migrated into the command center.
- Starter Pack Quincaillerie (the Mod 4 lifecycle UI) — migrated into the command center.
- Échanges & Données (import/export, data exchange tooling) — migrated into the command center.

### 5.2 Catalog Purification (Completed in `0812169`)
`CatalogModule.tsx` was stripped of administrative concerns (category management, data exchange, starter-pack controls — all relocated to Settings per 5.1) and purified into a focused operational surface limited to: sales-facing browsing, search, and stock visibility.

### 5.3 Remaining Scope of the UHD 4K Initiative (Not Yet Started — see Section 7)
The following items were approved by the Owner as part of the same overall initiative but were explicitly deferred past the `0812169` commit to SESSION-ARC-019:
- Elimination of "Empty Deserts" across Suppliers, Customers, and Reports screens.
- Typography and rendering upgrade to UHD 4K crispness, including resolution of faux-bold blurring (root cause identified, fix not yet applied — see Section 8, Errors & Dead Ends, "Typography faux-bold root cause").

---

## 6. Current Quality Gate Baseline (As Measured at Commit `0812169`)

- `cargo test --workspace`: 462 passed / 0 failed across 57 test suites.
- `cargo clippy -- -D warnings`: clean, zero warnings.
- `cargo fmt --check`: clean.
- `npx vitest run`: 581 passed / 0 failed across 57 test files.
- `eslint`: clean, zero warnings.
- `tsc` (TypeScript strict): clean, zero errors.
- `vite build`: clean, completed in 5.37s.

These are the baseline numbers SESSION-ARC-019 work must not regress. Any task in SESSION-ARC-019 that changes frontend or backend source must re-run the full gate set before being considered complete.

---

## 7. Pending Work for SESSION-ARC-019 (Owner-Approved, Not Yet Started)

The next task after this CTP/BOOT/ADR transition is **`TASK-DSN-054`**, scoped to three concurrent deliverables, all previously approved by the Owner as continuation of the UHD 4K Master UI/UX initiative:

1. **UHD 4K Typography Matrix** — Resolve the faux-bold rendering defect by sourcing and installing real 800-weight and 900-weight `woff2` files for both Inter and Cairo into `public/fonts/`, then auditing the 387 known call sites using `font-black`/`font-extrabold` utility classes to confirm they now resolve to genuine (not browser-synthesized) bold glyphs at 4K scaling.
2. **Catalog KPI Ribbon** — Introduce a KPI ribbon component into the now-purified `CatalogModule.tsx` operational view, surfacing live sales/stock metrics relevant to the sales-facing purpose established in Section 5.2, without reintroducing any administrative concerns that were deliberately relocated to Settings.
3. **Empty Desert Eradication (Suppliers & Customers)** — Remediate the specific empty-state defects already diagnosed in SESSION-ARC-018 (see Section 8 for exact file/line references for `SupplierListScreen.tsx` and `CustomersModule.tsx`); the Reports screen (`ReportsHomeScreen.tsx`) empty-desert defect is in scope for a later task in SESSION-ARC-019 but is not part of `TASK-DSN-054` itself.

`TASK-DSN-054` is the designated first substantive task of SESSION-ARC-019, to be issued as Reply 1 of that session, immediately following the successful close-out of this transition package.

---

## 8. Constraints, Invariants, Corrections, and Known Defects Carried Forward Verbatim

### 8.1 Owner Constraints & Protocol Corrections
- Arabic Modern Standard (فصحى) exclusively for Owner-facing reports; English exclusively for Executor-facing technical task specifications. The two registers must never be mixed within a single document.
- Strict Triad Protocol: exactly one task per reply, enclosed in a single fenced code block.
- Self-Contained Task Law (Charter §2.8): every task must embed all content to be written verbatim within the task block itself; an executor must never be referred back to a prior section or external text to "copy from."
- Absolute prohibition on summarization or shortcuts within CTP packages (Charter §15 and §16) — this document is written in compliance with that prohibition.
- Mandatory Owner approval is required before any execution task that alters visual architecture is issued.

### 8.2 Visual and Rendering Standards
- UHD 4K clarity is the rendering target; crisp contrast is mandatory.
- Icons must be pure vector SVG, 1.75px stroke weight; no emoji permitted in UI controls.
- No faux-bold font synthesis is permitted; only genuine font-weight files may be used to render bold text.

### 8.3 Numerical Display Standard
- All numerical displays use Latin digits (0–9) regardless of active locale.
- Numerical displays must use the `.tabular-nums` CSS feature and be wrapped in `<bdi dir="ltr">` to guarantee correct bidi isolation under RTL (Arabic) layout.

### 8.4 Layout Invariant
- `position: fixed` is prohibited everywhere in the codebase except: modal backdrops, toast notifications, and the command palette.

### 8.5 Financial and Data Invariants
- The system is offline-first, absolutely, with no exception.
- SQLite operates in WAL journal mode with `synchronous=FULL`.
- All monetary values are stored and computed as `i64` centimes; no floating-point monetary arithmetic is permitted anywhere.
- All quantities are stored and computed as fixed-point values scaled by `10^4`; no floating-point quantity arithmetic is permitted anywhere.

### 8.6 Migration Freeze
- Migrations `0001` through `0015` are strictly immutable. They are marked `* -text` in `.gitattributes` to prevent any line-ending normalization or incidental modification. No task may alter the content of any migration file in this range under any circumstance; new schema changes require a new migration file numbered `0016` or higher.

### 8.7 Guard Invariants (Engine-Level, Non-Negotiable)
- The `DEP-01` depot can never be deleted or deactivated, under any condition.
- Any customer whose `balance` is not equal to zero can never be deleted or deactivated.
- Any warehouse carrying non-zero stock can never be deleted.

### 8.8 Errors & Dead Ends Register (Carried Forward Verbatim from SESSION-ARC-018)
- **`count_warehouse_nonzero_levels` bug:** The function queried the deprecated column `quantity_micro` instead of the live column `quantity_base_micro` introduced in migration `0003`, which allowed warehouses holding real stock to bypass the deletion guard. Corrected to reference `quantity_base_micro` in commit `a9e66bc`.
- **CRLF/LF drift incident:** Editing `sqlite_customer_repository.rs` inadvertently flipped the file's line endings from CRLF to LF, producing a spurious 1,375-line diff. The file was reverted to CRLF. Any future edits to this file (or any file under the CRLF convention) must preserve existing line-ending style; verify with a line-ending check before committing.
- **Typography faux-bold root cause (unresolved, scoped into `TASK-DSN-054`):** `public/fonts/` does not contain weight-800 or weight-900 `woff2` files for either Inter or Cairo. As a result, all 387 call sites using the `font-black` or `font-extrabold` Tailwind utility classes are being rendered via browser-side faux-bold synthesis, which produces visibly blurry glyphs on 4K displays. The fix requires sourcing genuine 800/900-weight font files and installing them; this is the primary driver of the "UHD 4K Typography Matrix" item in Section 7.
- **Empty deserts in UI (unresolved, scoped into `TASK-DSN-054` and later SESSION-ARC-019 tasks):**
  - `SupplierListScreen.tsx:159` — hardcoded blank state that is shown unconditionally while awaiting a search query, even when supplier data already exists and could be shown by default.
  - `CustomersModule.tsx:163` — right-hand detail pane is left completely empty whenever `selectedId === null`, instead of showing a useful default (e.g., summary/aggregate view).
  - `ReportsHomeScreen.tsx:72` — report tiles are rendered as text-only labels with no preview metrics, making the screen feel empty of information until a tile is opened. (Explicitly out of scope for `TASK-DSN-054`; deferred to a later SESSION-ARC-019 task per Section 7.)
- **Task formulation rejection (process lesson, not a code defect):** The original `TASK-REL-061` (prior to this `-C1` reissue) instructed the executor to transcribe CTP text from "Section 1" of a prior document rather than embedding the complete text directly in the task block, violating Self-Contained Task Law (Charter §2.8) and the no-shortcuts rule (Charter §15). The executor correctly refused per the DOC-047 precedent rather than fabricate content. This CTP document and its reissued task (`TASK-REL-061-C1`) correct that defect by embedding the full verbatim content directly.

---

## 9. Handoff Certification

This document constitutes the complete, non-summarized transfer of SESSION-ARC-018 state to SESSION-ARC-019. It has been written without omission of any fact recorded in the SESSION-ARC-018 record available at handoff time. SESSION-ARC-019, Reply 1, shall begin with issuance of `TASK-DSN-054` as specified in Section 7, immediately following successful completion of `TASK-REL-061-C1` (this task).

**END OF CTP-SESSION-ARC-019**
