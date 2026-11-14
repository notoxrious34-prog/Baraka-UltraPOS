-- Migration 0021: Super-Gros bulk price per selling unit (BRK-117).
-- Additive: one NULL column, no backfill (absent = fall back to wholesale
-- then sale per `price_for_tier`). Existing rows read as `None`.
ALTER TABLE product_units ADD COLUMN super_gros_price_centimes INTEGER;
