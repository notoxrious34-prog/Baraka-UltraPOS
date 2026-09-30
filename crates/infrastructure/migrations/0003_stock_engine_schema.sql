-- Baraka UltraPOS migration 0003: stock engine alignment (additive only).
--
-- Verification against 0001 (see completion report):
-- - warehouses: id/code/name/is_active kept (TEXT UUID sovereign scheme; the
--   INTEGER AUTOINCREMENT target was adapted, not adopted, to preserve FK
--   coherence with all TEXT-keyed tables). Missing columns added below.
-- - stock_levels: kept PK(warehouse_id, product_id); quantity_base_micro
--   added with explicit base-micros semantics (legacy quantity_micro kept).
-- - stock_movements: kept TEXT ids; engine columns added; CHECK constraints
--   are enforced in application code (SQLite cannot ALTER in a CHECK).
-- - Append-only triggers for stock_movements already exist in 0001; the
--   product/warehouse index already exists. Only the transfer-batch partial
--   index is genuinely missing and is added here.
-- Never DROP TABLE or rebuild: existing data is preserved.

ALTER TABLE warehouses ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE warehouses ADD COLUMN origin_device_id TEXT;
ALTER TABLE warehouses ADD COLUMN created_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z';
ALTER TABLE warehouses ADD COLUMN updated_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z';

ALTER TABLE stock_levels ADD COLUMN quantity_base_micro INTEGER NOT NULL DEFAULT 0;
ALTER TABLE stock_levels ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE stock_levels ADD COLUMN updated_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z';

ALTER TABLE stock_movements ADD COLUMN unit_id TEXT;
ALTER TABLE stock_movements ADD COLUMN quantity_base_micro INTEGER NOT NULL DEFAULT 0;
ALTER TABLE stock_movements ADD COLUMN reference_type TEXT;
ALTER TABLE stock_movements ADD COLUMN reference_id TEXT;
ALTER TABLE stock_movements ADD COLUMN transfer_batch_id TEXT;
ALTER TABLE stock_movements ADD COLUMN authorized_negative_stock INTEGER NOT NULL DEFAULT 0;
ALTER TABLE stock_movements ADD COLUMN reason TEXT;

CREATE INDEX IF NOT EXISTS idx_stock_movements_transfer_batch
    ON stock_movements(transfer_batch_id) WHERE transfer_batch_id IS NOT NULL;
