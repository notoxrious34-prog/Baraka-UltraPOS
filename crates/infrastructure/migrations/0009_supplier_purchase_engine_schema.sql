-- Baraka UltraPOS migration 0009: supplier + purchase engine (TASK-BRK-011).
--
-- Investigation findings driving this file (mandatory pre-implementation audit):
-- 1. documents.type / documents.party_type carry NO database CHECK constraints in
--    migrations 0001-0008; allowed values are enforced in code only, and the code
--    DocumentType enum ALREADY includes BC, BR, FF. Therefore NO documents rebuild
--    is required and NONE is performed (the task's table-rebuild assumption does
--    not apply to the actual schema). party_type accepts 'SUPPLIER' unchanged.
-- 2. stock_movements.movement_type has NO CHECK either; 'PURCHASE_IN' already
--    exists as a StockMovementType domain code and is accepted today. No change.
-- 3. cash_movements.movement_type (recreated by 0008) DOES have a CHECK listing
--    six types; SUPPLIER_PAYMENT_OUT must be added. SQLite cannot ALTER a CHECK,
--    so cash_movements is rebuilt with the non-destructive create-copy-drop-rename
--    pattern. The amount-sign CHECK groups SUPPLIER_PAYMENT_OUT with EXPENSE_OUT
--    and MANUAL_OUT (strictly negative). Pre-existing cash_movements append-only
--    triggers die with the DROP and are re-created identically here.
-- 4. supplier_ledger already has append-only triggers from 0001 using the exact
--    customer_ledger pattern; the supplier_ledger rebuild below re-creates those
--    triggers with IDENTICAL names and abort messages (no reinvention).
-- 5. document_sequences is keyed generically by PRIMARY KEY(type, fiscal_year);
--    BC/BR/FF insert new keys with zero schema change.
-- 6. Historical-data note: pre-0009 suppliers rows (none exist in any known
--    database today; the module had no write path) are migrated with origin
--    'UNKNOWN'; suppliers.code is dropped because the authoritative shape defines
--    no code column and no code path consumed it.
-- Never DROP TABLE without first copying live rows.

-- 1. suppliers: authoritative shape --------------------------------------
CREATE TABLE IF NOT EXISTS suppliers_new (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(trim(name) != ''),
    phone TEXT NULL,
    address TEXT NULL,
    nif TEXT NULL,
    rc TEXT NULL,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
    current_balance_centimes INTEGER NOT NULL DEFAULT 0,
    notes TEXT NULL,
    origin_device_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- 2. supplier_ledger: authoritative shape, append-only -------------------
CREATE TABLE IF NOT EXISTS supplier_ledger_new (
    id TEXT PRIMARY KEY,
    supplier_id TEXT NOT NULL REFERENCES suppliers_new(id),
    entry_type TEXT NOT NULL CHECK(entry_type IN ('DEBIT','CREDIT')),
    amount_centimes INTEGER NOT NULL CHECK(amount_centimes > 0),
    reference_type TEXT NOT NULL CHECK(reference_type IN ('PURCHASE_INVOICE','PAYMENT')),
    reference_id TEXT NOT NULL,
    description TEXT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_by TEXT NOT NULL REFERENCES users(id)
) STRICT;

-- 3. Cash movements: widen movement_type CHECK (SUPPLIER_PAYMENT_OUT). ---
CREATE TABLE IF NOT EXISTS cash_movements_new (
    id TEXT PRIMARY KEY,
    cash_session_id TEXT NOT NULL REFERENCES cash_sessions(id),
    movement_type TEXT NOT NULL CHECK(movement_type IN
        ('OPENING_FLOAT','SALE_CASH_IN','CREDIT_PAYMENT_IN','EXPENSE_OUT',
         'MANUAL_IN','MANUAL_OUT','SUPPLIER_PAYMENT_OUT')),
    amount_centimes INTEGER NOT NULL CHECK(
        (movement_type = 'OPENING_FLOAT' AND amount_centimes >= 0)
        OR (movement_type IN ('SALE_CASH_IN','CREDIT_PAYMENT_IN','MANUAL_IN')
            AND amount_centimes > 0)
        OR (movement_type IN ('EXPENSE_OUT','MANUAL_OUT','SUPPLIER_PAYMENT_OUT')
            AND amount_centimes < 0)
    ),
    reference_type TEXT NULL,
    reference_id TEXT NULL,
    description TEXT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_by TEXT NOT NULL REFERENCES users(id)
) STRICT;

-- 4. Copy pre-existing rows (non-destructive) ------------------------------
INSERT OR IGNORE INTO suppliers_new
    (id, name, phone, address, nif, rc, is_active,
     current_balance_centimes, notes, origin_device_id, created_at, updated_at)
SELECT
    id, name, phone, address, NULL, NULL, is_active,
    current_balance_centimes, NULL, 'UNKNOWN', created_at, updated_at
FROM suppliers
WHERE NOT EXISTS (SELECT 1 FROM suppliers_new WHERE suppliers_new.id = suppliers.id);

INSERT OR IGNORE INTO supplier_ledger_new
    (id, supplier_id, entry_type, amount_centimes,
     reference_type, reference_id, description, created_at, created_by)
SELECT
    id,
    supplier_id,
    CASE WHEN entry_type = 'CREDIT' THEN 'CREDIT' ELSE 'DEBIT' END,
    amount_centimes,
    CASE WHEN document_type = 'PURCHASE_INVOICE'
         THEN 'PURCHASE_INVOICE' ELSE 'PAYMENT' END,
    COALESCE(document_id, 'LEGACY-MIGRATED'),
    NULL,
    created_at,
    created_by
FROM supplier_ledger
WHERE NOT EXISTS (SELECT 1 FROM supplier_ledger_new WHERE supplier_ledger_new.id = supplier_ledger.id);

INSERT OR IGNORE INTO cash_movements_new
    (id, cash_session_id, movement_type, amount_centimes,
     reference_type, reference_id, description, created_at, created_by)
SELECT
    id, cash_session_id, movement_type, amount_centimes,
    reference_type, reference_id, description, created_at, created_by
FROM cash_movements
WHERE NOT EXISTS (SELECT 1 FROM cash_movements_new WHERE cash_movements_new.id = cash_movements.id);

-- 5. Replace legacy tables (children before parents) ------------------------
DROP TABLE IF EXISTS cash_movements;
ALTER TABLE cash_movements_new RENAME TO cash_movements;

DROP TABLE IF EXISTS supplier_ledger;
DROP TABLE IF EXISTS suppliers;
ALTER TABLE suppliers_new RENAME TO suppliers;
ALTER TABLE supplier_ledger_new RENAME TO supplier_ledger;

-- 6. Indices -----------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_suppliers_name ON suppliers(name);
CREATE INDEX IF NOT EXISTS idx_supplier_ledger_supplier
    ON supplier_ledger(supplier_id, created_at);
CREATE INDEX IF NOT EXISTS idx_cash_movements_session
    ON cash_movements(cash_session_id);

-- 7. Triggers ------------------------------------------------------------------
-- supplier_ledger append-only: IDENTICAL pattern, names, and abort messages as
-- the customer_ledger triggers from migration 0001 (investigation point 4).
CREATE TRIGGER IF NOT EXISTS prevent_supplier_ledger_update
BEFORE UPDATE ON supplier_ledger
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: supplier_ledger is strictly append-only');
END;

CREATE TRIGGER IF NOT EXISTS prevent_supplier_ledger_delete
BEFORE DELETE ON supplier_ledger
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: supplier_ledger is strictly append-only');
END;

-- cash_movements append-only: re-created after the CHECK-widening rebuild
-- (identical pattern to 0001/0008).
CREATE TRIGGER IF NOT EXISTS prevent_cash_movements_update
BEFORE UPDATE ON cash_movements
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: cash_movements is strictly append-only');
END;

CREATE TRIGGER IF NOT EXISTS prevent_cash_movements_delete
BEFORE DELETE ON cash_movements
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: cash_movements is strictly append-only');
END;

-- documents / stock_movements: NO changes (investigation points 1 and 2).
