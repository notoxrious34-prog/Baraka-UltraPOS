-- Migration 0023: quantity-break price ladders per product x unit (BRK-121).
-- Additive table, no backfill (absent = pure tier pricing per
-- `price_for_tier`). One row per (product, unit, minimum quantity):
-- the greatest min_qty_micro at or below the line quantity wins; a break
-- never raises the tier price (min() rule in the pricing service).
CREATE TABLE IF NOT EXISTS price_breaks (
    id                  TEXT    NOT NULL PRIMARY KEY,
    product_id          TEXT    NOT NULL,
    unit_id             TEXT    NOT NULL,
    min_qty_micro       INTEGER NOT NULL CHECK (min_qty_micro > 0),
    price_centimes      INTEGER NOT NULL CHECK (price_centimes >= 0),
    UNIQUE (product_id, unit_id, min_qty_micro)
);
CREATE INDEX IF NOT EXISTS idx_price_breaks_product_unit
    ON price_breaks(product_id, unit_id);
