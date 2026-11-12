# ADR-110: Polymorphic Dual-Vertical Architecture
# (Apparel Wholesale/Retail + Hardware General Trade)

Status: ACCEPTED.
Date: 2026-11-12.
Authority: Owner directive + TASK-REL-020 (Sovereign Creative-Apex Constitution v4.0, Pillar 5).

## Context

`TASK-AUD-049-R1` proved the exact ground truth at `v1.2.1` (`0c93239a`, 281
commits, 19 migrations, ADRs through `ADR-109`): the repository carries NO
apparel-specific schema (no size/color/matrix columns), NO business-activity or
profile switch in `settings`/`store_identity`, and zero occurrences of
`sous-vêtement/underwear` in code or docs. Both commercial activities run today
on ONE universal engine:

- `customers.customer_tier` (`retail` default, `ADR-087`, migration `0015`) +
  `product_units.wholesale_price_centimes` + `price_for_tier`
  (`crates/domain/src/catalog/unit.rs:81-84`) — Retail/Artisan/Wholesale pricing.
- `product_units.factor_micro` (10⁴ fixed-point) + `product_kit_items`
  (`0001:59-80`) — fractional hardware quantities and kit/pack deduction.
- `customer_ledger` + `current_balance_centimes`
  (`crates/domain/src/sales/customer.rs:84-85`) — wholesale credit balances.
- Moroccan-market reality: underwear sells by `Pièce / Boîte ×3 / Demi-Dz ×6 /
  Douzaine ×12 / Balle-Carton`, ordered by `Model × Size × Color` grids and
  `Séries assorties`; hardware sells by `Pièce / m / kg / L` with fractional
  cutting and contractor (`chantier`) credit.

Naive branching (`if shop_type == …` in pricing/stock/math) would fork the two
most sacred paths in the system — financial math and atomic stock deduction —
and double every future bug. This ADR forbids that fork.

## Decision

Adopt **capability-driven polymorphism**: the domain engine stays universal;
verticals become composable **profiles** plus additive universal capabilities.

1. **Vertical profiles** (`apparel_wholesale_retail` | `hardware_general` |
   `hybrid_enterprise`) live in configuration + `store_identity`, NEVER in core
   math. A profile selects: default tiers (incl. future `Super-Gros` /
   quantity-breaks), terminology (FR/AR microcopy per trade), unit presets
   (`Douzaine/Série/Colis` vs `m/kg/L`), starter matrices, and print/label
   templates. Switching profile changes defaults and surfaces — never prices,
   never ledger entries, never migration history.
2. **Multi-level packaging (*Colisage*)** is a universal unit ladder on
   `product_units` (`Pièce → Boîte ×3 → Demi-Dz ×6 → Douzaine ×12 →
   Balle/Carton`), with automatic conversion and carton/dozen invoice totals.
   Hardware reuses the same ladder (`Pièce → Boîte → Carton`; `m/kg/L` stay
   fractional). One conversion engine, two vocabularies.
3. **Variant matrix** (`Parent Model/Ref × Size × Color/Finish`) is a universal
   parent-children catalog capability (new additive migrations `0020+`): 1-click
   variant/SKU/barcode generation, rapid 2D grid order entry in POS and
   Purchases, per-variant stock. Hardware colors/finishes reuse the same axis
   (`peinture RAL`, `visserie Ø/length`) with zero special-casing in math.
4. **Assorted packs (*Série / Colis Assorti*)** ride the existing
   `product_kit_items` semantics (e.g. `1 Série = 2M + 4L + 4XL + 2XXL`),
   deducting exact component stock atomically inside the single sale
   transaction. No new deduction path is invented.
5. **Wholesale credit-equation snapshot** (`Ancien Solde + Montant Document −
   Versement = Nouveau Solde`) is recorded atomically at finalization and
   printed on every surface (80/58mm, Graphique/TSPL, A4 `BL`/`Facture`) —
   Pillar 6. The ledger stays the single truth; the snapshot is its frozen
   printed witness.

## Equivalence & safety proof (required per wave)

- No existing column, trigger, or total changes meaning: all v2.0 schema is
  additive (`0020+`), proven by a data-retention upgrade test on a synthetic
  populated `v1.2.1` DB in scratch (Pillar 4).
- `price_for_tier`, `verify_cart_line`, the atomic sale transaction, and
  `ROUND_HALF_UP` are untouched by profile switching (profile is read at the
  IPC edge for defaults only, exactly like `terminal_id` per `ADR-028/045`).
- FTS5 recall must not regress: phonetic/trade-slang aliases
  (`dri ll`, `Série`, `Douzaine`) extend `product_aliases`, never the tokenizer.
- Every wave ships its regression + visual gates (Pillar 8) before commit.

## Consequences

- One engine, two storefronts, zero math forks: a bug fixed for underwear is
  fixed for hardware by construction.
- Profiles make the third vertical nearly free (same mechanism, new preset).
- Cost: new additive migrations + matrix/2D-entry UX are the largest v2.0 work
  items (Waves 2–4 of `ROADMAP-v2.0-DUAL-VERTICAL.md`); printing surfaces
  multiply (two label studios, equation on all tickets) and are pinned by
  `0019`-style template versioning.
- Rejected alternative: `shop_type` branching in `sales.rs`/`pricing.rs` —
  rejected for forking sacred invariants and doubling audit surface.
