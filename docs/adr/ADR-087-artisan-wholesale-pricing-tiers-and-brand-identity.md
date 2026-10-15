# ADR-087: Artisan & Wholesale Pricing Tiers, Margin Guard, and Official Brand Identity

Status: ACCEPTED
Date: 2026-10-14
Session: SESSION-ARC-015 (BRK-068 / DSN-047)

## Context

Baraka UltraPOS serves a dual commercial identity: a neighbourhood
quincaillerie (retail walk-in trade) and a general-retail supplier serving
artisans (plumbers, electricians, masons) and grossistes (resellers) who
expect professional pricing. Before Pillar 4, every customer paid the single
retail unit price, and nothing in the pipeline detected a sale priced below
its purchase cost (vente à perte). In parallel, the store owner ratified an
official brand mark: gold 3D calligraphy "بركة" inside a modern smartphone
frame (`D:/Dev/BarakaInbox/Baraka_Logo.png`, read-only reference).

## Decision

1. **Migration 0015** (append-only; 0001–0014 frozen) adds
   `customers.customer_tier TEXT NOT NULL DEFAULT 'retail'` and
   `product_units.wholesale_price_centimes INTEGER` (nullable; SQLite
   `ADD COLUMN` carries no `CHECK` — Rust domain validation is authoritative).
2. **Domain `CustomerTier`** (`Retail` / `Artisan` / `Wholesale`, loose
   `FromStr` with `Retail` fallback) on `Customer`; `ProductUnit` gains
   `wholesale_price: Option<Money>` and `price_for_tier(tier)` —
   `Retail` resolves the sale price, `Artisan`/`Wholesale` resolve the
   wholesale price when set, otherwise the sale price. `verify_cart_line`
   takes the resolved tier; checkout, draft, and preview totals all resolve
   the tier from `customers.get_customer(customer_id)` (guest or
   missing → `Retail`), so preview and commit can never diverge
   (`sale_preview_commit_parity`, 3/3 green).
3. **Integer-exact Margin Guard**: `calculate_margin_basis_points` uses
   `i128` intermediates (`((selling − cost) × 10000) / selling`, zero floats),
   returns `-10000` on total loss, and flags `is_below_cost`. Verified
   vectors: `(15000,10000)→3333`, `(8000,10000)→−2500⊗`,
   `(0,10000)→−10000⊗`, `(0,0)→0`, `(8500,7000)→1764`. The POS cart pulses
   `⚠️ Vente à perte / بيع بالخسارة` with the exact margin; tier pricing is
   customer-driven system-wide — no operator price override exists.
4. **Official brand**: the source logo was mathematically de-matted
   (smoothstep alpha 4→64, source-luminance fringe snap, zero white rim)
   into `src/assets/logo/baraka-logo.png` (478×512) and
   `baraka-logo-icon.png` (128×128), embedded in TopBar, NavRail, login,
   About tab, and regenerated as multi-resolution Windows icons
   (`src-tauri/icons/`: ICO with 16–256 mipmaps + full PNG set).

## Consequences

- Zero floats preserved across all financial paths; margins are advisory
  (the sale still commits — the guard warns, it does not block).
- Tier is a customer attribute, editable in the customer modal; wholesale
  price is a per-unit attribute, editable in the product create/edit modals
  (blank clears it). Seeding law holds: fixtures carry structurally valid
  tier/wholesale data.
- Brand assets are build inputs; the de-matting one-shot tooling was deleted
  (NO_LEFTOVERS) and is reproducible from this record.
