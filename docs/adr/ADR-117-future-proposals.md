# ADR-117: Future High-Level Proposals — Documented, NOT Implemented

Status: PROPOSED (parking lot — no code, no migration, no UI change).

## P1 — Quantity-Break Engine
- Value: Super-Gros is a flat price today; quantity breaks (≥12/≥48/≥120)
  raise wholesale margin automatically.
- UX: break strip in the cart (`12+ → −3 %`) updating live.
- Path: `price_breaks(product_unit, min_qty, price)` + `price_for_tier(qty)`
  above `price_for_tier` (shape untouched).

## P2 — Replenishment Radar
- Value: alert before stock-out from sales velocity, not a fixed
  threshold (critical for fast sizes).
- UX: `Top Ruptures` board in reports + badge on the product card.
- Path: `sales_velocity(product, size, 30d)` from the ledger +
  read-time `days_of_cover`; zero new schema.

## P3 — Quote-to-Picklist (Devis → BL)
- Value: wholesalers request quotes that convert to orders in one tap.
- UX: `تحويل إلى BL` on the Devis creates a draft with frozen prices.
- Path: reuse `DocumentConversion` (BL→Facture exists) — add Devis→BL
  with the same atomic guarantees.

## P4 — Size-Run Intelligence
- Value: suggest the optimal size split for a Série from sales history
  (kills dead stock).
- UX: in the Série builder: `التوزيع المقترح: 2M+4L+4XL+2XXL (ثقة 87%)`.
- Path: aggregate `size × qty` from `document_items` into percentages,
  read-only.

## P5 — Multi-Counter Sync (Offline-First Hardening)
- Value: the real growth gate — more than one cashier without sequence
  collisions.
- UX: `متصل/معزول` badge + visible sync queue.
- Path: per-terminal `sequence` (`terminal_id` exists) + an `outbox`
  table for prints; needs its own ADR before any code.

## P6 — Voice Picking (experimental, behind a feature flag)
- Value: faster wholesale order picking (`"دزينة L أبيض"`).
- UX: mic button in search, visual confirm before adding.
- Path: Web Speech API in the frontend only → same `catalog_search`;
  zero engine touch.

## Order
`BRK-111` → `DSN-106` → `DOC-048` → P1 → Wave 3 → P2/P4 → P5 (own ADR).
