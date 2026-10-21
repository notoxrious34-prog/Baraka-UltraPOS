-- Migration 0016: FTS5 search index apparatus (TASK-BRK-076, ADR-101).
--
-- SCOPE: schema + triggers + norm backfill ONLY. The repository query path
-- stays byte-identical LIKE until the BRK-077 cutover; nothing here changes
-- read behavior.
--
-- Tokenizer note: the workspace bundles SQLite 3.46.0, where
-- `unicode61 "remove_diacritics N"` (needs 3.51+) is a parse error (proven by
-- the BRK-076 step-0 probe). Plain `unicode61` is equivalent HERE because
-- every indexed token is pre-normalized (accents/diacritics stripped,
-- Arabic unified) by `baraka_search_norm` at write time — ADR-101 keeps the
-- FTS tokenizer deliberately dumb; unicode61 retains ASCII case-folding.
--
-- Identity note: `products.id` is TEXT (UUID), so the FTS table carries its
-- own `product_id UNINDEXED` key column instead of relying on rowid sync.
-- Content mode is the FTS default (fully stored): the table keeps its own
-- copies and the triggers below do plain DELETE + INSERT mirroring.
-- (`content=''` was tried first and rejected: in FTS5 it means contentless —
-- column values read back as NULL and DELETE ... WHERE product_id = ? cannot
-- match, so the mirror silently accumulates duplicates. Proven by probe.)
--
-- Covering barcode indexes already exist (0001: idx_products_barcode,
-- idx_product_units_barcode) — verified live, NOT duplicated here.

ALTER TABLE products ADD COLUMN search_norm_fr TEXT NOT NULL DEFAULT '';
ALTER TABLE products ADD COLUMN search_norm_ar TEXT NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS product_aliases (
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    alias_norm TEXT NOT NULL,
    UNIQUE (product_id, alias_norm)
) STRICT;

CREATE INDEX IF NOT EXISTS idx_product_aliases_product
    ON product_aliases(product_id);

CREATE VIRTUAL TABLE IF NOT EXISTS products_fts USING fts5(
    product_id UNINDEXED,
    norm_fr,
    norm_ar,
    norm_sku,
    norm_barcode,
    norm_aliases,
    tokenize = 'unicode61'
);

-- Refresh pattern: re-mirror one product's FTS row from relational truth.
-- Used by the update/unit/alias triggers via DELETE + re-INSERT so rows
-- missing before the backfill (or removed) converge instead of drifting.
-- Aggregate choice: norm_barcode covers the product barcode plus every unit
-- barcode; norm_aliases covers linked alias_norm values (space-joined; FTS
-- tokenizes the separators away). All normalization flows through the SAME
-- `baraka_search_norm` scalar the query path uses.

CREATE TRIGGER IF NOT EXISTS trg_products_fts_ai AFTER INSERT ON products
BEGIN
    INSERT INTO products_fts(
        product_id, norm_fr, norm_ar, norm_sku, norm_barcode, norm_aliases
    )
    VALUES (
        NEW.id,
        baraka_search_norm(NEW.name_fr),
        baraka_search_norm(NEW.name_ar),
        baraka_search_norm(NEW.sku),
        (
            SELECT COALESCE(group_concat(baraka_search_norm(b), ' '), '')
            FROM (
                SELECT NEW.barcode AS b
                UNION
                SELECT barcode FROM product_units WHERE product_id = NEW.id
            )
            WHERE b IS NOT NULL
        ),
        (
            SELECT COALESCE(group_concat(alias_norm, ' '), '')
            FROM product_aliases
            WHERE product_id = NEW.id
        )
    );
END;

CREATE TRIGGER IF NOT EXISTS trg_products_fts_ad AFTER DELETE ON products
BEGIN
    DELETE FROM products_fts WHERE product_id = OLD.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_products_fts_au AFTER UPDATE ON products
BEGIN
    DELETE FROM products_fts WHERE product_id = OLD.id;
    INSERT INTO products_fts(
        product_id, norm_fr, norm_ar, norm_sku, norm_barcode, norm_aliases
    )
    SELECT
        p.id,
        baraka_search_norm(p.name_fr),
        baraka_search_norm(p.name_ar),
        baraka_search_norm(p.sku),
        (
            SELECT COALESCE(group_concat(baraka_search_norm(b), ' '), '')
            FROM (
                SELECT p.barcode AS b
                UNION
                SELECT barcode FROM product_units WHERE product_id = p.id
            )
            WHERE b IS NOT NULL
        ),
        (
            SELECT COALESCE(group_concat(alias_norm, ' '), '')
            FROM product_aliases
            WHERE product_id = p.id
        )
    FROM products AS p
    WHERE p.id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_units_fts_ai AFTER INSERT ON product_units
BEGIN
    DELETE FROM products_fts WHERE product_id = NEW.product_id;
    INSERT INTO products_fts(
        product_id, norm_fr, norm_ar, norm_sku, norm_barcode, norm_aliases
    )
    SELECT
        p.id,
        baraka_search_norm(p.name_fr),
        baraka_search_norm(p.name_ar),
        baraka_search_norm(p.sku),
        (
            SELECT COALESCE(group_concat(baraka_search_norm(b), ' '), '')
            FROM (
                SELECT p.barcode AS b
                UNION
                SELECT barcode FROM product_units WHERE product_id = p.id
            )
            WHERE b IS NOT NULL
        ),
        (
            SELECT COALESCE(group_concat(alias_norm, ' '), '')
            FROM product_aliases
            WHERE product_id = p.id
        )
    FROM products AS p
    WHERE p.id = NEW.product_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_units_fts_au AFTER UPDATE ON product_units
BEGIN
    DELETE FROM products_fts WHERE product_id = OLD.product_id;
    DELETE FROM products_fts WHERE product_id = NEW.product_id;
    INSERT INTO products_fts(
        product_id, norm_fr, norm_ar, norm_sku, norm_barcode, norm_aliases
    )
    SELECT
        p.id,
        baraka_search_norm(p.name_fr),
        baraka_search_norm(p.name_ar),
        baraka_search_norm(p.sku),
        (
            SELECT COALESCE(group_concat(baraka_search_norm(b), ' '), '')
            FROM (
                SELECT p.barcode AS b
                UNION
                SELECT barcode FROM product_units WHERE product_id = p.id
            )
            WHERE b IS NOT NULL
        ),
        (
            SELECT COALESCE(group_concat(alias_norm, ' '), '')
            FROM product_aliases
            WHERE product_id = p.id
        )
    FROM products AS p
    WHERE p.id = NEW.product_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_units_fts_ad AFTER DELETE ON product_units
BEGIN
    DELETE FROM products_fts WHERE product_id = OLD.product_id;
    INSERT INTO products_fts(
        product_id, norm_fr, norm_ar, norm_sku, norm_barcode, norm_aliases
    )
    SELECT
        p.id,
        baraka_search_norm(p.name_fr),
        baraka_search_norm(p.name_ar),
        baraka_search_norm(p.sku),
        (
            SELECT COALESCE(group_concat(baraka_search_norm(b), ' '), '')
            FROM (
                SELECT p.barcode AS b
                UNION
                SELECT barcode FROM product_units WHERE product_id = p.id
            )
            WHERE b IS NOT NULL
        ),
        (
            SELECT COALESCE(group_concat(alias_norm, ' '), '')
            FROM product_aliases
            WHERE product_id = p.id
        )
    FROM products AS p
    WHERE p.id = OLD.product_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_aliases_fts_ai AFTER INSERT ON product_aliases
BEGIN
    DELETE FROM products_fts WHERE product_id = NEW.product_id;
    INSERT INTO products_fts(
        product_id, norm_fr, norm_ar, norm_sku, norm_barcode, norm_aliases
    )
    SELECT
        p.id,
        baraka_search_norm(p.name_fr),
        baraka_search_norm(p.name_ar),
        baraka_search_norm(p.sku),
        (
            SELECT COALESCE(group_concat(baraka_search_norm(b), ' '), '')
            FROM (
                SELECT p.barcode AS b
                UNION
                SELECT barcode FROM product_units WHERE product_id = p.id
            )
            WHERE b IS NOT NULL
        ),
        (
            SELECT COALESCE(group_concat(alias_norm, ' '), '')
            FROM product_aliases
            WHERE product_id = p.id
        )
    FROM products AS p
    WHERE p.id = NEW.product_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_aliases_fts_ad AFTER DELETE ON product_aliases
BEGIN
    DELETE FROM products_fts WHERE product_id = OLD.product_id;
    INSERT INTO products_fts(
        product_id, norm_fr, norm_ar, norm_sku, norm_barcode, norm_aliases
    )
    SELECT
        p.id,
        baraka_search_norm(p.name_fr),
        baraka_search_norm(p.name_ar),
        baraka_search_norm(p.sku),
        (
            SELECT COALESCE(group_concat(baraka_search_norm(b), ' '), '')
            FROM (
                SELECT p.barcode AS b
                UNION
                SELECT barcode FROM product_units WHERE product_id = p.id
            )
            WHERE b IS NOT NULL
        ),
        (
            SELECT COALESCE(group_concat(alias_norm, ' '), '')
            FROM product_aliases
            WHERE product_id = p.id
        )
    FROM products AS p
    WHERE p.id = OLD.product_id;
END;

-- Backfill (same transaction as the DDL via the migration runner): fill the
-- new norm columns from the SAME scalar the triggers and query path use.
-- Each row UPDATE fires trg_products_fts_au, which inserts the FTS row, so
-- relational columns and the FTS mirror converge in one pass. Alias rows are
-- seeded by the runner in Rust (domain dictionary) right after this batch;
-- each alias INSERT fires trg_aliases_fts_ai and refreshes the FTS row.
UPDATE products
SET search_norm_fr = baraka_search_norm(name_fr),
    search_norm_ar = baraka_search_norm(name_ar);
