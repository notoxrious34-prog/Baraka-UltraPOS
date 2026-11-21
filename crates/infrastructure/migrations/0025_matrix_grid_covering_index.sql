-- Migration 0025: covering index for the variant-matrix grid (BRK-129, AUD-060 F-1).
-- The DSN-108/109 grid query sums stock per product cell
-- (WHERE s.product_id = p.id) but the stock_levels PRIMARY KEY leads with
-- warehouse_id, so every cell triggered a full table SCAN (~537ms for a
-- 500-cell model at 10k SKUs). This additive covering index turns the
-- correlated subquery into an index SEARCH with no heap visit.
-- Pure addition: no table touched, no backfill, fully online.
CREATE INDEX IF NOT EXISTS idx_stock_levels_product_covering
    ON stock_levels(product_id, quantity_base_micro);
