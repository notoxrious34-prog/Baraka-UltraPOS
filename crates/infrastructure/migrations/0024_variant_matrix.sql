-- Migration 0024: variant-matrix engine (BRK-114, Wave 4).
-- Additive tables, no backfill, zero impact on single products:
-- existing products keep working untouched; a variant model groups
-- generated product rows (each cell IS a product: SKU/barcode/stock
-- and tier prices live on products/product_units as before).
-- (`model_id`, `axis1_value`, `axis2_value`) is unique; display order
-- rides explicit sort keys (sizes never sort alphabetically wrong).
CREATE TABLE IF NOT EXISTS variant_models (
    id                  TEXT    NOT NULL PRIMARY KEY,
    name_ar             TEXT    NOT NULL,
    name_fr             TEXT    NOT NULL,
    axis1_name          TEXT    NOT NULL,
    axis2_name          TEXT    NOT NULL,
    category_id         TEXT    REFERENCES categories(id),
    is_active           INTEGER NOT NULL DEFAULT 1,
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS variant_cells (
    id                  TEXT    NOT NULL PRIMARY KEY,
    model_id            TEXT    NOT NULL REFERENCES variant_models(id) ON DELETE RESTRICT,
    product_id          TEXT    NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    axis1_value         TEXT    NOT NULL,
    axis2_value         TEXT    NOT NULL,
    axis1_sort          INTEGER NOT NULL,
    axis2_sort          INTEGER NOT NULL,
    UNIQUE (model_id, axis1_value, axis2_value)
) STRICT;

CREATE INDEX IF NOT EXISTS idx_variant_cells_model
    ON variant_cells(model_id);
CREATE INDEX IF NOT EXISTS idx_variant_cells_product
    ON variant_cells(product_id);
