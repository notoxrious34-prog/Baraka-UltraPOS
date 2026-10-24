# TASK-DSN-089 REPORT — Drawer product designation render

Wire already carried the fields (BRK-088 `e8661c2`); this task renders them.
`DocumentDetailDrawer.tsx` render ONLY (+ FR/AR i18n key). No wire/query
changes. No commit.

## (1) FAIL-before (quoted)

Live render lines (verified pre-edit):
- `DocumentDetailDrawer.tsx:132` → `{item.product_id}` (A4 sheet cell)
- `:233` → `{item.product_id}` + `:236` → `{item.unit_id}` (details block)

New pins red on old render (5/5):
```
× shows the FR designation with zero UUID text in fr-DZ
    → expected '<div class="flex flex-col gap-3"><sec…' to contain 'Vis TR 4x40'
× shows the AR designation RTL-isolated in ar-DZ
× falls back across locales, then to SKU, never to UUID
× degrades orphans to SKU plus an amber information badge
× declares the orphan badge key in FR and AR
```
Plus the standing DSN-084 goldens pinned the bug:
`expect(html).toContain("prod-vis")` (`:123`), `toContain("prod-ciment")` (`:133`).

## (2) Render lines (post-edit)

- Helpers (`DocumentDetailDrawer.tsx:54-75`): `itemDisplayName`
  (active-locale-first via `locale === "ar-DZ"` — same convention as
  `CartTable.tsx:148` — fallback other-locale → sku → raw id; orphans
  degrade to sku-or-id) + `itemUnitLabel` (nullable).
- Details-lines cell (`:255-282`): name line (`<bdi>` auto-isolation —
  Arabic RTL, Latin LTR) or, when `product_missing`, sku-or-id `dir="ltr"`
  + amber badge `documents.detailsOrphanBadge` (tokens reused from the
  empty-lines notice `:203` — information, not red-error); sub-line
  `SKU · unit · qty` (`dir="ltr"`, nulls filtered, `fmtQty` display-only;
  qty/price columns untouched).
- A4 sheet cell (`:153-157`, same file): designation instead of UUID
  (objective: no UUID in any state; fiscal columns untouched).
- i18n (`src/lib/i18n.ts`, CRLF-safe): `documents.detailsOrphanBadge` =
  "Produit archivé" (FR `:313`) / "منتج مؤرشف" (AR `:1401`).
  Separator note: the `·` is a locale-neutral literal, consistent with
  existing `fiscal.join(" · ")` (`:107`) — no key invented.
- DSN-084 goldens updated to the correct display (`:123` → "Vis TR 4x40";
  `:133` → "إسمنت 50كغ", proving locale-first in ar-DZ).

## (3) Pin lines (all green)

```
✓ shows the FR designation with zero UUID text in fr-DZ
✓ shows the AR designation RTL-isolated in ar-DZ
✓ falls back across locales, then to SKU, never to UUID
✓ degrades orphans to SKU plus an amber information badge
✓ declares the orphan badge key in FR and AR
Test Files  2 passed (2) / Tests  20 passed (20)
```

## (4) PNGs + asserts (`audit/DSN-089/`)

- `drawer-1280x800-fr-light.png` (88,814 B): drawer open on 3-line ticket
  (Vis TR 4x40, Café Arabica 250g, SUCRE-1KG + "Produit archivé" badge).
- `drawer-1280x800-ar-dark.png` (78,186 B): dark RTL, panel mirrored left,
  `برغي 4×40` / `قهوة عربية 250غ` + `منتج مؤرشف` badge (vision-verified).
- `capture.mjs` (CDP vs vite :5173, stubbed `documents_search` + `sales_get`;
  honest fixes en route: locale/theme toggles live only in the Caisse
  topbar — capture goes there and back; theme clicked while FR since its
  aria-label localizes). `capture-results.json`: **CAPTURE GREEN 8/8** —
  names+badge visible, UUID-CLEAN (`/[0-9a-f]{8}-[0-9a-f]{4}-/` absent),
  noPageScroll 1280x800 both shots, **0 console errors**.
- Vite (own PID 1392, started 15:24:22) stopped; port 5173 verified free.
  `vite.log` retained under `audit/DSN-089/`.

## (5) Gates

| Gate | Result |
|---|---|
| G4 `npm run lint` | EXIT 0 |
| G5 `npm run typecheck` | EXIT 0 (incl. `I18nKey = keyof typeof fr` — new key in both dicts) |
| Targeted Vitest (both touched files) | 20/20 |
| G1–G3 | N/A — zero Rust (state reason) |
| G6 | N/A — build untouched |

## (6) Footprint

[MODIFIED] `src/features/documents/DocumentDetailDrawer.tsx`,
`src/lib/i18n.ts`, `src/features/documents/document-detail-drawer.test.tsx`.
[CREATED] `audit/DSN-089/REPORT.md`, `capture.mjs`, `capture-results.json`,
2 PNGs, `vite.log`. No commit. Live DB never opened. No strays.

## (7) STATE / DONE / REMAINING

STATE done. DONE: drawer + A4 sheet show designation; orphans degrade with
badge; pins + 2 vision-verified shots + gates green. REMAINING: owner point
3 fully closed on trial (strip/printer-UX separate tasks). NEXT READY: YES.
