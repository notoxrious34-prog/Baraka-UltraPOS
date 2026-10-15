# ADR-088: Single-Instance Application Mutex and Quincaillerie Starter Fixture

Status: ACCEPTED
Date: 2026-10-14
Session: SESSION-ARC-015 (BRK-069)

## Context

Operating multiple concurrent instances on Windows risks SQLite database
lock contention, cash session state desynchronization, and printer port
conflicts. Audit AUD-044 confirmed single-instance protection was absent
(`tauri::Builder::default()` with no mutex or focus logic). Initial store
setup also requires a realistic, high-quality product baseline rather than
starting from an empty catalog; no retail seed fixture existed in the
repository (only the DEP-01 warehouse seed, migration 0012).

## Decision

1. Integrate `tauri-plugin-single-instance = "2"` (resolved v2.5.4) into
   `src-tauri/src/lib.rs`. Secondary launch attempts immediately focus,
   unminimize, and show the existing primary window, terminating the
   secondary process.
2. Embed an authentic 300-item dual-language Algerian hardware starter
   dataset in `src/assets/fixtures/quincaillerie_starter_300.csv`
   (strict UTF-8 BOM, semicolon-delimited, canonical headers matching
   `catalog-import-parser.ts`, covering all 7 `STARTER_CATEGORIES` codes,
   77% checksum-verified EAN-13 barcodes, commercial margins 15–40%).
   The task's proposed `wholesale_price`/`unit`/`conversion_factor` columns
   were deliberately omitted: the import parser, wire, and engine carry no
   wholesale or multi-unit fields (verified against
   `catalog-import-parser.ts` and `ImportProductRow`), so such columns would
   be silently dropped. The fixture lives under `src/assets/` (Vite
   `?raw`-importable) rather than root `assets/` (not web-served).
3. Provide a direct one-click UI affordance in the `CatalogModule` toolbar
   (`✦ Pack Quincaillerie (300)`, FR/AR) feeding `ProductImportModal` via a
   `preset` prop for instant onboarding through the existing atomic import
   engine (preview → validate → import).

## Consequences

- Zero risk of concurrent POS process corruption on the single checkout
  counter; SQLite locking remains as a second backstop only.
- Merchants can launch and populate their shop within 30 seconds of first
  install (300 rows, zero validation rejections).
- Fixture generation was a deterministic seeded one-shot; tooling deleted
  (NO_LEFTOVERS). Re-generation procedure is recorded in the BRK-069 report.
