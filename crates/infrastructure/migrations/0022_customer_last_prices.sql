-- TASK-BRK-111: last transacted unit price per customer x product x unit.
-- Additive table (no backfill): empty = unknown, never zero. Both price
-- legs are stored so recall can show the effective price exactly
-- (unit_price - discount_per_unit); the engine never auto-applies it.
CREATE TABLE IF NOT EXISTS customer_last_prices (
    customer_id               TEXT    NOT NULL,
    product_id                TEXT    NOT NULL,
    unit_id                   TEXT    NOT NULL,
    price_centimes            INTEGER NOT NULL,
    discount_per_unit_centimes INTEGER NOT NULL DEFAULT 0,
    document_id               TEXT    NOT NULL,
    sold_at                   TEXT    NOT NULL,
    PRIMARY KEY (customer_id, product_id, unit_id)
);
CREATE INDEX IF NOT EXISTS idx_customer_last_prices_doc
    ON customer_last_prices(document_id);
