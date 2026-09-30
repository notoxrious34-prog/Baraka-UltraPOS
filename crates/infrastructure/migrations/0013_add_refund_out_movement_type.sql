-- Baraka UltraPOS migration 0013: return-refund cash movement (TASK-BRK-046).
--
-- Investigation findings driving this file:
-- 1. cash_movements.movement_type (recreated by 0008, widened by 0009)
--    carries a CHECK listing seven types; REFUND_OUT must be added for the
--    return engine's drawer payouts. SQLite cannot ALTER a CHECK, so
--    cash_movements is rebuilt with the non-destructive
--    create-copy-drop-rename pattern established by 0008/0009.
-- 2. The amount-sign CHECK groups REFUND_OUT with EXPENSE_OUT, MANUAL_OUT
--    and SUPPLIER_PAYMENT_OUT (strictly negative), mirroring the domain
--    MovementType::requires_negative contract (cash_movement.rs).
-- 3. Pre-existing cash_movements append-only triggers die with the DROP and
--    are re-created identically here (0009 precedent, same names/messages).
-- 4. No other table changes: documents.type / party columns carry NO CHECK
--    (AVOIR accepted today), stock_movements.movement_type carries NO CHECK
--    (RETURN_IN accepted today), customer_ledger.entry_type accepts CREDIT.
-- Never DROP TABLE without first copying live rows.

-- 1. Widened cash_movements (REFUND_OUT added) ---------------------------
CREATE TABLE IF NOT EXISTS cash_movements_new (
    id TEXT PRIMARY KEY,
    cash_session_id TEXT NOT NULL REFERENCES cash_sessions(id),
    movement_type TEXT NOT NULL CHECK(movement_type IN
        ('OPENING_FLOAT','SALE_CASH_IN','CREDIT_PAYMENT_IN','EXPENSE_OUT',
         'MANUAL_IN','MANUAL_OUT','SUPPLIER_PAYMENT_OUT','REFUND_OUT')),
    amount_centimes INTEGER NOT NULL CHECK(
        (movement_type = 'OPENING_FLOAT' AND amount_centimes >= 0)
        OR (movement_type IN ('SALE_CASH_IN','CREDIT_PAYMENT_IN','MANUAL_IN')
            AND amount_centimes > 0)
        OR (movement_type IN ('EXPENSE_OUT','MANUAL_OUT','SUPPLIER_PAYMENT_OUT','REFUND_OUT')
            AND amount_centimes < 0)
    ),
    reference_type TEXT NULL,
    reference_id TEXT NULL,
    description TEXT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_by TEXT NOT NULL REFERENCES users(id)
) STRICT;

-- 2. Copy pre-existing rows (non-destructive) ------------------------------
INSERT OR IGNORE INTO cash_movements_new
    (id, cash_session_id, movement_type, amount_centimes,
     reference_type, reference_id, description, created_at, created_by)
SELECT
    id, cash_session_id, movement_type, amount_centimes,
    reference_type, reference_id, description, created_at, created_by
FROM cash_movements
WHERE NOT EXISTS (SELECT 1 FROM cash_movements_new WHERE cash_movements_new.id = cash_movements.id);

-- 3. Replace legacy table -----------------------------------------------
DROP TABLE IF EXISTS cash_movements;
ALTER TABLE cash_movements_new RENAME TO cash_movements;

-- 4. Indices ---------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_cash_movements_session
    ON cash_movements(cash_session_id);

-- 5. Triggers (identical pattern, names, and abort messages) ---------------
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
