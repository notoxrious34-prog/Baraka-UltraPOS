-- Baraka UltraPOS canonical baseline schema (migration 0001).
-- 25 tables, STRICT mode throughout, append-only and finalized-document
-- triggers enforced at the database level. No ORM: this file is the schema.

-- 01 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    checksum TEXT NOT NULL,
    applied_at TEXT NOT NULL
) STRICT;

-- 02 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- 03 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    pin_hash TEXT,
    role TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- 04 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY,
    code TEXT UNIQUE NOT NULL,
    name_ar TEXT NOT NULL,
    name_fr TEXT NOT NULL,
    parent_id TEXT REFERENCES categories(id),
    is_active INTEGER NOT NULL DEFAULT 1
) STRICT;

-- 05 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS products (
    id TEXT PRIMARY KEY,
    sku TEXT UNIQUE NOT NULL,
    barcode TEXT,
    name_ar TEXT NOT NULL,
    name_fr TEXT NOT NULL,
    category_id TEXT REFERENCES categories(id),
    is_kit INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1,
    origin_device_id TEXT NOT NULL
) STRICT;

-- 06 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS product_units (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    unit_name_ar TEXT NOT NULL,
    unit_name_fr TEXT NOT NULL,
    factor_micro INTEGER NOT NULL,
    barcode TEXT UNIQUE,
    sale_price_centimes INTEGER NOT NULL,
    purchase_price_centimes INTEGER NOT NULL,
    is_base_unit INTEGER NOT NULL,
    is_default_selling INTEGER NOT NULL
) STRICT;

-- 07 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS product_kit_items (
    id TEXT PRIMARY KEY,
    kit_product_id TEXT NOT NULL REFERENCES products(id),
    component_product_id TEXT NOT NULL REFERENCES products(id),
    unit_id TEXT NOT NULL REFERENCES product_units(id),
    quantity_micro INTEGER NOT NULL
) STRICT;

-- 08 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS warehouses (
    id TEXT PRIMARY KEY,
    code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1
) STRICT;

-- 09 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS stock_levels (
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    quantity_micro INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (warehouse_id, product_id)
) STRICT;

-- 10 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS stock_movements (
    id TEXT PRIMARY KEY,
    sequence_no INTEGER NOT NULL,
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    movement_type TEXT NOT NULL,
    quantity_micro INTEGER NOT NULL,
    reference_document_type TEXT,
    reference_document_id TEXT,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL
) STRICT;

-- 11 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS customers (
    id TEXT PRIMARY KEY,
    code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    phone TEXT,
    address TEXT,
    credit_policy TEXT NOT NULL,
    credit_limit_centimes INTEGER NOT NULL DEFAULT 0,
    current_balance_centimes INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1,
    origin_device_id TEXT NOT NULL
) STRICT;

-- 12 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS customer_ledger (
    id TEXT PRIMARY KEY,
    sequence_no INTEGER NOT NULL,
    customer_id TEXT NOT NULL REFERENCES customers(id),
    entry_type TEXT NOT NULL,
    amount_centimes INTEGER NOT NULL,
    running_balance_centimes INTEGER NOT NULL,
    document_type TEXT,
    document_id TEXT,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL
) STRICT;

-- 13 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS suppliers (
    id TEXT PRIMARY KEY,
    code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    phone TEXT,
    address TEXT,
    current_balance_centimes INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- 14 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS supplier_ledger (
    id TEXT PRIMARY KEY,
    sequence_no INTEGER NOT NULL,
    supplier_id TEXT NOT NULL REFERENCES suppliers(id),
    entry_type TEXT NOT NULL,
    amount_centimes INTEGER NOT NULL,
    running_balance_centimes INTEGER NOT NULL,
    document_type TEXT,
    document_id TEXT,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL
) STRICT;

-- 15 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    number TEXT NOT NULL,
    fiscal_year INTEGER NOT NULL,
    type TEXT NOT NULL,
    party_type TEXT,
    party_id TEXT,
    source_document_id TEXT REFERENCES documents(id),
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    document_language TEXT NOT NULL,
    subtotal_ht_centimes INTEGER NOT NULL,
    vat_total_centimes INTEGER NOT NULL,
    global_discount_centimes INTEGER NOT NULL,
    timbre_centimes INTEGER NOT NULL,
    rounding_centimes INTEGER NOT NULL,
    ttc_centimes INTEGER NOT NULL,
    paid_amount_centimes INTEGER NOT NULL,
    change_amount_centimes INTEGER NOT NULL,
    payment_method TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(id),
    revision INTEGER NOT NULL DEFAULT 1,
    origin_device_id TEXT NOT NULL,
    UNIQUE(type, fiscal_year, number)
) STRICT;

-- 16 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS document_items (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    unit_id TEXT NOT NULL REFERENCES product_units(id),
    source_item_id TEXT,
    quantity_micro INTEGER NOT NULL,
    quantity_base_micro INTEGER NOT NULL,
    unit_price_centimes INTEGER NOT NULL,
    discount_centimes INTEGER NOT NULL,
    tva_rate_micro INTEGER NOT NULL,
    tax_exemption_code TEXT,
    line_ht_centimes INTEGER NOT NULL,
    line_tva_centimes INTEGER NOT NULL,
    line_ttc_centimes INTEGER NOT NULL,
    cost_price_centimes INTEGER NOT NULL,
    sort_order INTEGER NOT NULL
) STRICT;

-- 17 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS payments (
    id TEXT PRIMARY KEY,
    sequence_no INTEGER NOT NULL,
    document_id TEXT REFERENCES documents(id),
    customer_id TEXT REFERENCES customers(id),
    amount_centimes INTEGER NOT NULL,
    payment_method TEXT NOT NULL,
    cash_session_id TEXT,
    notes TEXT,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL
) STRICT;

-- 18 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS cash_sessions (
    id TEXT PRIMARY KEY,
    terminal_id TEXT NOT NULL,
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    status TEXT NOT NULL,
    opened_at TEXT NOT NULL,
    closed_at TEXT,
    opening_float_centimes INTEGER NOT NULL,
    system_expected_centimes INTEGER NOT NULL DEFAULT 0,
    actual_counted_centimes INTEGER,
    variance_centimes INTEGER,
    notes TEXT
) STRICT;

-- 19 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS cash_movements (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES cash_sessions(id),
    movement_type TEXT NOT NULL,
    amount_centimes INTEGER NOT NULL,
    reason TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL
) STRICT;

-- 20 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tax_rules (
    id TEXT PRIMARY KEY,
    rule_code TEXT NOT NULL,
    name TEXT NOT NULL,
    rate_micro INTEGER NOT NULL,
    effective_from TEXT NOT NULL,
    effective_to TEXT,
    is_active INTEGER NOT NULL DEFAULT 1
) STRICT;

-- 21 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS document_sequences (
    type TEXT NOT NULL,
    fiscal_year INTEGER NOT NULL,
    last_sequence INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(type, fiscal_year)
) STRICT;

-- 22 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS print_jobs (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id),
    status TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    is_duplicate INTEGER NOT NULL DEFAULT 0,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- 23 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS command_log (
    command_id TEXT PRIMARY KEY,
    command_type TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    response_payload TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

-- 24 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS audit_log (
    sequence_no INTEGER PRIMARY KEY AUTOINCREMENT,
    prev_hash TEXT NOT NULL,
    record_hash TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    user_id TEXT NOT NULL,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    old_value_json TEXT,
    new_value_json TEXT
) STRICT;

-- 25 ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS backups_log (
    id TEXT PRIMARY KEY,
    backup_id TEXT UNIQUE NOT NULL,
    app_version TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    source_device_id TEXT NOT NULL,
    database_hash TEXT NOT NULL,
    cipher_version TEXT NOT NULL,
    status TEXT NOT NULL
) STRICT;

-- Indexes ---------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_products_sku ON products(sku);
CREATE INDEX IF NOT EXISTS idx_products_barcode ON products(barcode);
CREATE INDEX IF NOT EXISTS idx_product_units_product ON product_units(product_id);
CREATE INDEX IF NOT EXISTS idx_product_units_barcode ON product_units(barcode);
CREATE INDEX IF NOT EXISTS idx_stock_movements_product_warehouse
    ON stock_movements(product_id, warehouse_id);
CREATE INDEX IF NOT EXISTS idx_customer_ledger_customer
    ON customer_ledger(customer_id, sequence_no);
CREATE INDEX IF NOT EXISTS idx_supplier_ledger_supplier
    ON supplier_ledger(supplier_id, sequence_no);
CREATE INDEX IF NOT EXISTS idx_document_items_document ON document_items(document_id);
CREATE INDEX IF NOT EXISTS idx_payments_document ON payments(document_id);
CREATE INDEX IF NOT EXISTS idx_payments_customer ON payments(customer_id);
CREATE INDEX IF NOT EXISTS idx_cash_movements_session ON cash_movements(session_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_entity ON audit_log(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_print_jobs_document ON print_jobs(document_id);

-- Append-only ledger triggers (UPDATE + DELETE forbidden) ----------------
CREATE TRIGGER prevent_customer_ledger_update
BEFORE UPDATE ON customer_ledger
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: customer_ledger is strictly append-only');
END;

CREATE TRIGGER prevent_customer_ledger_delete
BEFORE DELETE ON customer_ledger
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: customer_ledger is strictly append-only');
END;

CREATE TRIGGER prevent_supplier_ledger_update
BEFORE UPDATE ON supplier_ledger
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: supplier_ledger is strictly append-only');
END;

CREATE TRIGGER prevent_supplier_ledger_delete
BEFORE DELETE ON supplier_ledger
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: supplier_ledger is strictly append-only');
END;

CREATE TRIGGER prevent_stock_movements_update
BEFORE UPDATE ON stock_movements
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: stock_movements is strictly append-only');
END;

CREATE TRIGGER prevent_stock_movements_delete
BEFORE DELETE ON stock_movements
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: stock_movements is strictly append-only');
END;

CREATE TRIGGER prevent_cash_movements_update
BEFORE UPDATE ON cash_movements
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: cash_movements is strictly append-only');
END;

CREATE TRIGGER prevent_cash_movements_delete
BEFORE DELETE ON cash_movements
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: cash_movements is strictly append-only');
END;

CREATE TRIGGER prevent_payments_update
BEFORE UPDATE ON payments
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: payments is strictly append-only');
END;

CREATE TRIGGER prevent_payments_delete
BEFORE DELETE ON payments
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: payments is strictly append-only');
END;

CREATE TRIGGER prevent_audit_log_update
BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: audit_log is strictly append-only');
END;

CREATE TRIGGER prevent_audit_log_delete
BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: audit_log is strictly append-only');
END;

CREATE TRIGGER prevent_command_log_update
BEFORE UPDATE ON command_log
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: command_log is strictly append-only');
END;

CREATE TRIGGER prevent_command_log_delete
BEFORE DELETE ON command_log
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: command_log is strictly append-only');
END;

-- Finalized document immutability triggers -------------------------------
CREATE TRIGGER prevent_documents_update_finalized
BEFORE UPDATE ON documents
BEGIN
    SELECT CASE
        WHEN OLD.status IN ('COMPLETED', 'FINALIZED', 'POSTED')
        THEN RAISE(ABORT, 'MUTATION_FORBIDDEN: finalized document is immutable')
    END;
END;

CREATE TRIGGER prevent_document_items_update_finalized
BEFORE UPDATE ON document_items
BEGIN
    SELECT CASE
        WHEN (SELECT status FROM documents WHERE id = OLD.document_id)
             IN ('COMPLETED', 'FINALIZED', 'POSTED')
        THEN RAISE(ABORT, 'MUTATION_FORBIDDEN: document_items of finalized document is immutable')
    END;
END;

CREATE TRIGGER prevent_document_items_delete_finalized
BEFORE DELETE ON document_items
BEGIN
    SELECT CASE
        WHEN (SELECT status FROM documents WHERE id = OLD.document_id)
             IN ('COMPLETED', 'FINALIZED', 'POSTED')
        THEN RAISE(ABORT, 'MUTATION_FORBIDDEN: document_items of finalized document is immutable')
    END;
END;
