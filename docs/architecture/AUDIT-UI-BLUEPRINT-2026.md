# UI Architectural Blueprint — Frontend Tree at v1.0.0-rc.10 (`a7f538c`)

Read-only audit (TASK-AUD-006). Zero production files touched; this document
is the only write. All paths are relative to the repository root; line
numbers refer to commit `a7f538c`.

## 1. Navigation & routing inventory

No router dependency. `src/stores/app-view-store.ts:11` declares:

```ts
export type AppView = "pos" | "stock" | "customers" | "documents"
  | "suppliers" | "reports" | "catalog" | "users" | "settings";
```

`src/App.tsx` renders `NavRail` (`App.tsx:40-76`, `h-10` top bar,
`bg-primary-900`) with one button per view in fixed order
(`App.tsx:65-73`): pos → stock → customers → documents → suppliers →
reports → catalog → settings, plus `users` gated to `role === "ADMIN"`.
View switching is a ternary chain (`App.tsx:136-162`) behind the manager
gate (`isManagerRole`); non-managers and unknown views fall through to
`<PosShell />`. Root container sets `dir`/`lang` and swaps
`font-sans`/`font-arabic` by locale (`App.tsx:125-129`).

| Nav id | Label key | Component | File |
| --- | --- | --- | --- |
| `pos` | `nav.pos` | `PosShell` → `PosShellView` | `src/components/pos/PosShellView.tsx` |
| `stock` | `nav.stock` | `StockModule` | `src/features/stock/StockModule.tsx` |
| `customers` | `nav.customers` | `CustomersModule` | `src/features/customers/CustomersModule.tsx` |
| `documents` | `nav.documents` | `DocumentsModule` | `src/features/documents/` |
| `suppliers` | `nav.suppliers` | `SuppliersModule` | `src/features/suppliers/SupplierListScreen.tsx:26` |
| `reports` | `nav.reports` | `ReportsHomeScreen` | `src/features/reports/ReportsHomeScreen.tsx` |
| `catalog` | `nav.catalog` | `CatalogModule` | `src/features/catalog/CatalogModule.tsx` (1151 lines) |
| `settings` | `nav.settings` | `SettingsModule` | `src/features/settings/SettingsModule.tsx` (71 lines) |
| `users` | `nav.users` (ADMIN only) | `UsersModule` | `src/features/users/` |

## 2. Current Settings architecture

`SettingsModule.tsx` holds local `useState<SettingsTab>` (default
`"identity"`), `session` from `useSessionStore`, `locale` from
`usePosUiStore`, and derives `isAdmin` (`SettingsModule.tsx:19-23`).
Layout: `section.flex.h-full.min-h-0.flex-col.overflow-hidden` → static
`header` (title + pill tab strip, `SettingsModule.tsx:35-55`) → content div
(`SettingsModule.tsx:56-68`) with one conditional branch per tab.

| Tab id | Label key | Component | Session-gated |
| --- | --- | --- | --- |
| `identity` | `settings.tabIdentity` | `StoreIdentityTab` (`locale` only) | no |
| `printer` | `settings.tabPrinter` | `PrinterSettingsTab` (`locale`, `sessionToken`) | yes |
| `backups` | `settings.tabBackups` | `BackupSettingsTab` (+ `isAdmin`) | yes |
| `fiscal` | `settings.tabFiscal` | `FiscalSettingsTab` (+ `isAdmin`) | yes |
| `about` | `settings.tabAbout` | `AboutTab` (`locale` only) | no |

Insertion points for the consolidation (all inside `SettingsModule.tsx`):

- (a) **Rayons & Catégories tab** — extend the `SettingsTab` union
  (`:17`), append to `tabs` (`:25-31`), add a branch in the content div
  (`:56-68`) rendering `CategoryManageModal` content as a tab panel. Note:
  `CategoryManageModal` props are `{ isOpen, token, onClose, onChanged }`
  (`CategoryManageModal.tsx:23-28`); conversion to an always-open panel
  requires dropping `isOpen`/`onClose` and keeping `token` (session-gated
  like `printer`) + `onChanged`.
- (b) **Starter Pack Lifecycle tab** — same three edits; mount
  `StarterPackBanner` (`src/features/catalog/StarterPackBanner.tsx`) as the
  panel. Banner props are `{ locale, token, warehouses, products,
  onChanged }`; Settings must therefore load `warehouses`
  (`listWarehouses(token)`) and `products` (`ipcSearchCatalog(token, "",
  LIMIT)`) or lift them into a shared store — today both live only in
  `CatalogModule` state (see §3).
- (c) **Échanges & Données tab** — same three edits; mount
  `DataExchangeMenu` + `ProductImportModal`/`catalog-export.ts` flows.
  `DataExchangeMenu` props are `{ onOpenImportModal, onImportStarterPack,
  onExportCsv }` (`components/DataExchangeMenu.tsx:14-18`); the import modal
  needs `token` + `onChanged(refresh)` equivalents.

## 3. Current Catalog architecture & detach map

`CatalogModule.tsx` has two layers: presentational `CatalogHubView`
(`:342`, props `:260-300` incl. optional `banner?: ReactNode` at `:297`)
and the stateful container `CatalogModule` (`:~740-1151`).

Config surfaces to detach (exact anchors):

1. **StarterPackBanner** — import `:16`; `warehouses` state
   (`:766`, populated in `ensureDepot` at `:781` via
   `listWarehouses(tok)`); banner element wired at `:1061-1071`
   (token-gated, `products={products}`, `onChanged={refresh}`); rendered
   inside the hub at `:414` (`{props.banner}`). Detach steps: (i) move the
   `StarterPackBanner` element to the new Settings tab; (ii) relocate the
   `warehouses` load out of `ensureDepot` (keep `depotRef`/`pickDepotId`
   for stock levels at `:797-816`); (iii) remove the `banner` prop
   (`:297`, `:414`, `:1061-1071`) and the `WarehouseItem` import —
   `onStarterPack` (`:293`, `:456`, `:1047-1054`, preset-import flow) can
   retire with it or be re-pointed at Settings.
2. **CategoryManageModal** — import `:23`; open-state `:758`
   (`isCategoryModalOpen`); trigger via `onManageCategories` prop
   (`:298`, pill strip `:514-520`, container `:1058-1060`); modal mounted at
   `:1073-1083` with `token={activeToken}` and
   `onChanged={reloadCategories + refresh}`. Detach steps: convert to panel
   per §2(a); keep `reloadCategories` reachable (categories feed the hub
   pill strip at `:514`).
3. **DataExchangeMenu** — import `:25`; rendered in the hub toolbar at
   `:453-456` (`onOpenImportModal → props.onImport`,
   `onImportStarterPack → props.onStarterPack`, export via
   `catalog-export.ts`); container handlers `onImport` (`:1043-1046`,
   opens `ProductImportModal` at `:1085+`) and `onExport`
   (`handleExport`). Detach steps: move menu + `ProductImportModal` mount
   + export handler to the new Settings tab; hub keeps only search/filter/
   table/delete/toggle paths.

Safe-to-keep in Catalog: toolbar search, `CategoryPillStrip` (read-only
filter), `ProductTable` rows (edit/delete/toggle), `ProductCreateModal`
(depot `<select>` stays — operational), `ProductEditModal`,
`ProductLabelModal`.

## 4. "Empty desert" audit

- **Suppliers** (`SupplierListScreen.tsx:159-161`): initial state renders a
  bare `<p>{t(locale, "sup.noResults")}</p>` when `suppliers.length === 0
  && !loading` — no skeleton, no CTA to `SupplierFormModal`
  (`formMode`/`panel` state at `:36-38` exists but is undiscoverable from
  the empty state). Fix point: empty branch at `:159`.
- **Reports** (`ReportsHomeScreen.tsx:72-85`): `grid grid-cols-2
  md:grid-cols-3` of 9 text-only tiles (`TILES` at `:29-41`, no icons,
  no descriptions, no counts); detail routing is local `active:
  ReportKey | null` state (`:45`) with a `← rep.back` button (`:49-57`).
  No dead state — but tiles carry zero preview data.
- **Customers** (`CustomersModule.tsx:139-177`): split layout
  `div.flex.min-h-0.flex-1` → `CustomerListPane` + `div.flex-1`
  wrapping `CustomerDossierPane` with `detail={selected}` (`:163`,
  `selected` derived at `:123` from `selectedId: string | null` at
  `:55`). `selectedId === null` (initial) yields the dossier empty state
  inside `CustomerDossierPane`; list empty branch at
  `CustomerListPane.tsx:104`. Fix point: dossier null-detail branch.
- **POS cockpit** (`PosShellView.tsx:16-40`): `Topbar` (`h-10`,
  `Topbar.tsx:85`), session banner when blocked, `main.grid.grid-cols-12`
  → `ProductDiscovery` (`col-span-7`, `:159`) + `CartPanel`, `KeyFooter`.
  Discovery empty state at `ProductDiscovery.tsx:231`; results grid
  `grid-cols-2 xl:grid-cols-3` at `:239`. No desert — grid is populated on
  mount via discovery load.

## 5. Typography & high-DPI audit

- **Fonts** (`src/index.css:1-73`): fully offline, OFL, `font-display:
  swap`, files in `public/fonts/`: Inter 400/500/600/700 (woff2), Cairo
  400/600/700 (TTF), JetBrains Mono 400/500/700 (woff2). **Gap: Inter 800/900
  and Cairo 800/900 are absent** — every `font-black` (800) and
  `font-extrabold` synthesizes (faux-bold) at 4K. Body defaults to
  `Inter, Cairo, system-ui…` (`index.css:94-99`); `.font-arabic` swaps to
  Cairo-first (`:107-109`); Tailwind tokens mirror this
  (`tailwind.config.js:19-23`: `sans`, `arabic`, `mono`).
- **Numeric stability**: `.tabular-nums` (`index.css:75-77`) +
  `.cockpit-nums` (`:103-105`) + Tailwind `tabular-nums` used on money,
  counts, badges — no layout shift on digit change.
- **4K scalability**: no container queries (`plugins: []`,
  `tailwind.config.js:34`), no `2xl`-tuned type scale, no fluid
  `clamp()` sizes; layout is flex/grid with fixed `h-10` chrome bars and
  `p-4`/`p-2` gutters — structurally fluid but type does not step up with
  viewport. Sharpness levers available without deps: add Inter/Cairo 800
  woff2 faces, `-webkit-font-smoothing: antialiased` already on
  (`index.css:98`), tabular figures already enforced.
- **Direction**: logical properties (`text-start`, `ms-*`, `me-*`,
  `ps`/`pe`) used throughout; `fixed` restricted to modal overlays/toasts
  per charter.

## 6. State stores inventory (`src/stores/`)

`app-view-store.ts` (view routing), `session-store.ts` (auth session),
`pos-ui-store.ts` (locale, toasts), `cart-store.ts` (POS cart — financial
logic OUT of scope), `cash-session-store.ts`, `payment-modal-store.ts`,
`return-modal-store.ts`, `parked-sales` (via `ParkedSalesModal`),
`conversion-modal-store.ts`, `backup-modal-store.ts`,
`product-edit-store.ts`, `stock-adjust-modal-store.ts`,
`store-identity-store.ts`, `supplier-store.ts`,
`warehouse-transfer-store.ts` (transfer/create/manage tabs +
`submitUpdateWarehouse`/`submitDeleteWarehouse`). No store currently holds
catalog products or warehouses globally — both are `CatalogModule`-local
(`products`, `warehouses` at `:766`); consolidation (§2b) needs one or the
other lifted.

## 7. Move inventory (Catalog → Settings)

| Component | From | To | State/props to relocate |
| --- | --- | --- | --- |
| `StarterPackBanner.tsx` + `src/lib/starter-pack.ts` | Catalog hub top (`:414`, `:1061`) | New `pack` tab | `warehouses` load, `products` list, `refresh` |
| `CategoryManageModal.tsx` | Modal (`:1073`) + pill trigger (`:514`) | New `categories` tab (as panel) | `token`, `reloadCategories` |
| `DataExchangeMenu.tsx` + `ProductImportModal.tsx` + `catalog-export.ts` | Toolbar (`:453`) + modal (`:1085`) | New `exchange` tab | `token`, import/export handlers |
| `FiscalSettingsTab` | — (already in Settings) | stays | — |

## 8. Audit integrity

- Only file written: `docs/architecture/AUDIT-UI-BLUEPRINT-2026.md`.
- Production sources, tests, packages, lockfiles: untouched (verify with
  `git status` — sole entry must be this document untracked).
- Gates undisturbed by a read-only pass: Rust 462/0, Vitest 574/574
  (recorded at `a7f538c`; no code changed since).
