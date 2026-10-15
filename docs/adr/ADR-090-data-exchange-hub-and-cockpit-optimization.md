# ADR-090: Data Exchange Hub, POS Fast-Sale Optimization, and Responsive Catalogue Layout

Status: ACCEPTED
Date: 2026-10-14
Session: TASK-REL-055 (+C1) forensic drill & CSV repair / TASK-DSN-048 cockpit compaction

## Context

Owner field testing of v1.0.0-rc.6 found presentation-layer friction, not
engine defects: (1) the "Télécharger le modèle CSV standard" and "Exporter
CSV" buttons silently did nothing in desktop WebView2; (2) the catalogue
header carried five scattered action buttons; (3) the Caisse `Topbar`
(56 px brand bar) duplicated the `NavRail` brand directly above it; (4) POS
quick-sale tiles carried an administrative "Modifier" button that
distracted from scan-and-sell flow; (5) nested unconstrained containers
produced double horizontal scrollbars in the catalogue. The 10-pillar
forensic drill (`crates/infrastructure/tests/forensic_full_system_drill.rs`,
11/11 green) proved the engine behind these screens correct, so this ADR
restructures presentation only: zero Rust, migration, IPC, or
dependency changes.

## Decision

1. **One Data Exchange Hub for all catalogue file operations.**
   `src/features/catalog/components/DataExchangeMenu.tsx` ("Échanges &
   Données" / "استيراد وتصدير") replaces the three scattered header
   buttons. Four operations, one trigger next to "+ Nouveau Produit":
   CSV/TXT catalogue import (`onOpenImportModal`), 300-item quincaillerie
   starter pack preload (`onImportStarterPack`), Excel-compatible CSV
   export with UTF-8 BOM (`onExportCsv`), standard template download
   (`onDownloadTemplate`). Panel closes on outside click or Escape;
   `role="menu"` / `role="menuitem"` semantics; all labels localized
   FR/AR in `src/lib/i18n.ts` (`cat.dataExchange`, `cat.exchange*`).
2. **Desktop download root cause is fixed at the lifecycle level.**
   `src/features/catalog/desktop-download.ts` (`downloadTextFile`): the
   anchor stays attached for the synthetic click and
   `URL.revokeObjectURL` is deferred `DOWNLOAD_URL_LIFETIME_MS = 30_000`
   instead of synchronous revocation in the same task, which voided the
   `blob:` URL before WebView2's asynchronous pickup. Both UIs confirm
   with an inline `role="status"` filename note (8 s auto-dismiss).
3. **Caisse cockpit compaction: 56 px → 40 px, brand in one place.**
   `src/components/pos/Topbar.tsx` drops the logo + title/subtitle block
   (brand lives in the `NavRail` only) and compacts `h-14` to `h-10`;
   cash-session float, offline pill, depot selector, Operations menu,
   parked badge, user chip, clock, locale/theme toggles, and logout keep
   exact order and alignment. 16 px reclaimed for an extra POS tile row.
4. **Fast-sale tiles: 100% click-to-add, live stock chips.**
   `src/components/pos/ProductDiscovery.tsx` removes the "Modifier"
   affordance and its edit-store wiring from the grid; the whole tile is
   the hit area (`cursor-pointer` + hover border transition). Each tile
   carries a stock chip from the same `stock_get` source as the hub
   (`{qty} {unit}` emerald, red on zero, "…" while loading).
5. **Empty cart guides the operator.**
   `src/components/pos/CartTable.tsx` replaces bare "Panier vide" with a
   HUD: cart outline icon, scan/click subtitle, `[F1] Recherche` ·
   `[F2] Client` · `[F12] Encaisser` chips (localized).
6. **Catalogue grid: fixed 11-column colgroup, zero horizontal overflow.**
   `table-fixed w-full` with percentage widths
   8/9/20/12/7/9/9/8/8/4/6 (SKU, barcode, designation, rayon, unit,
   purchase, sale, margin, stock, status, actions); designation wraps,
   SKU/barcode/category truncate; the pill strip keeps wheel/touch scroll
   with the native thumb hidden (webkit/Firefox/Edge rules).

## Consequences

- Faster till operations: one-click tile add, visible stock, shortcut
  guidance, and a single obvious file-exchange entry point.
- Zero horizontal overflow at viewports ≥ 1024 px by construction
  (proportional columns, no nested x-scroll containers).
- No engine, schema, IPC, or dependency changes; Vitest suite grows
  505 → 526 (catalogue 146/146 incl. the deferred-lifecycle download
  contract), Rust workspace steady at 435/0.
- Full forensic evidence:
  `docs/operations/FORENSIC-AUDIT-REPORT-v1.0.0.md`; drill:
  `crates/infrastructure/tests/forensic_full_system_drill.rs`.
- Validation gate: owner re-verification of CSV save + menu on desktop
  hardware; the inline filename note makes success self-evident.
