# ADR-116: Vertical Profiles & Super-Gros Tiers

Status: Accepted (TASK-BRK-117, Wave 2).

## Context

The counter serves two trades with different tier vocabularies
(underwear: `Détail / Gros / Super-Gros`; hardware: `Détail / Artisan /
Gros`). Tier math had to stay universal while each trade sees only its
own ladder, and bulk (`Super-Gros`) prices needed a home.

## Decision

- Domain: `VerticalProfile` (`apparel_wholesale_retail` (default) |
  `hardware_general` | `hybrid_enterprise`) with `offered_tiers()` and
  `default_tier()` (always `Retail`). Profiles live in configuration +
  `store_identity`, never in core math (ADR-110).
- `CustomerTier::SuperGros` (`super_gros` + tolerant aliases);
  `price_for_tier` shape untouched — the new rung falls back
  bulk → wholesale → sale.
- Persistence: `vertical.profile` key in the existing `settings` table
  (absent = default; `SystemSettingsManage`-gated switch, audited).
  One additive migration: `0021` adds
  `product_units.super_gros_price_centimes NULL` (absent = fallback,
  no backfill).
- Edge: `vertical_profile_get` (any session) / `vertical_profile_set`
  (gated) return the profile plus the derived tier vocabulary, so the
  POS (DSN-105) never hardcodes trade surfaces.
- FTS legacy-seed harness now writes genuine pre-16 rows via raw SQL:
  current writers target the post-21 schema and must fail on old
  schemas by construction. This fixes the whole class, not one column.

## Consequences

- One engine, two storefronts, zero math forks (Pillar 5).
- `BRK-111` (last-price memory) and `DSN-105` (profile switch UX) build
  on this edge without new migrations.
