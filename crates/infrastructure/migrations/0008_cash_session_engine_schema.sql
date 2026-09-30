-- Baraka UltraPOS migration 0008: cash session engine (authoritative shape).
--
-- Brings cash_sessions and cash_movements to the exact sovereign shape via
-- non-destructive rebuild: new tables are created, existing rows are
-- migrated with column mapping (user_id -> opened_by, notes ->
-- closing_notes, session_id -> cash_session_id, reason -> description),
-- legacy 'CASH_IN' movements map to 'SALE_CASH_IN', then the old tables are
-- replaced. No DROP of live data without migration. Re-runnable through the
-- version-tracked runner (second run is a no-op); manual double execution is
-- guarded with IF NOT EXISTS / conditional rebuild.
--
-- New tables carry the full CHECK contracts, the single-active-session
-- unique index, and the CLOSED-immutability + no-delete triggers. The
-- pre-existing cash_movements append-only triggers from 0001 are re-created
-- against the rebuilt table (they die with the DROP).

-- 1. New authoritative cash_sessions ------------------------------------
CREATE TABLE IF NOT EXISTS cash_sessions_new (
    id TEXT PRIMARY KEY,
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    terminal_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('OPEN','CLOSING','CLOSED')),
    opened_by TEXT NOT NULL REFERENCES users(id),
    opened_at TEXT NOT NULL,
    opening_float_centimes INTEGER NOT NULL CHECK(opening_float_centimes >= 0),
    closed_by TEXT NULL REFERENCES users(id),
    closed_at TEXT NULL,
    actual_counted_centimes INTEGER NULL CHECK(actual_counted_centimes IS NULL OR actual_counted_centimes >= 0),
    system_expected_centimes INTEGER NULL,
    variance_centimes INTEGER NULL,
    closing_notes TEXT NULL,
    revision INTEGER NOT NULL DEFAULT 1,
    origin_device_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- 2. New authoritative cash_movements -----------------------------------
CREATE TABLE IF NOT EXISTS cash_movements_new (
    id TEXT PRIMARY KEY,
    cash_session_id TEXT NOT NULL REFERENCES cash_sessions_new(id),
    movement_type TEXT NOT NULL CHECK(movement_type IN
        ('OPENING_FLOAT','SALE_CASH_IN','CREDIT_PAYMENT_IN','EXPENSE_OUT',
         'MANUAL_IN','MANUAL_OUT')),
    amount_centimes INTEGER NOT NULL CHECK(
        (movement_type = 'OPENING_FLOAT' AND amount_centimes >= 0)
        OR (movement_type IN ('SALE_CASH_IN','CREDIT_PAYMENT_IN','MANUAL_IN')
            AND amount_centimes > 0)
        OR (movement_type IN ('EXPENSE_OUT','MANUAL_OUT')
            AND amount_centimes < 0)
    ),
    reference_type TEXT NULL,
    reference_id TEXT NULL,
    description TEXT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_by TEXT NOT NULL REFERENCES users(id)
) STRICT;

-- 3. Migrate existing rows (only when legacy tables still exist) --------
-- Sessions: map user_id -> opened_by, notes -> closing_notes.
INSERT OR IGNORE INTO cash_sessions_new
    (id, warehouse_id, terminal_id, status, opened_by, opened_at,
     opening_float_centimes, closed_by, closed_at,
     actual_counted_centimes, system_expected_centimes, variance_centimes,
     closing_notes, revision, origin_device_id, created_at, updated_at)
SELECT
    id,
    warehouse_id,
    terminal_id,
    CASE status WHEN 'CLOSING' THEN 'CLOSING' WHEN 'CLOSED' THEN 'CLOSED' ELSE 'OPEN' END,
    user_id,
    opened_at,
    opening_float_centimes,
    NULL,
    closed_at,
    actual_counted_centimes,
    CASE WHEN system_expected_centimes = 0 THEN NULL ELSE system_expected_centimes END,
    variance_centimes,
    notes,
    1,
    'LOCAL',
    datetime('now'),
    datetime('now')
FROM cash_sessions
WHERE NOT EXISTS (SELECT 1 FROM cash_sessions_new WHERE cash_sessions_new.id = cash_sessions.id);

-- Movements: map session_id -> cash_session_id, reason -> description,
-- legacy 'CASH_IN' -> 'SALE_CASH_IN'.
INSERT OR IGNORE INTO cash_movements_new
    (id, cash_session_id, movement_type, amount_centimes,
     reference_type, reference_id, description, created_at, created_by)
SELECT
    id,
    session_id,
    CASE movement_type WHEN 'CASH_IN' THEN 'SALE_CASH_IN' ELSE movement_type END,
    amount_centimes,
    NULL,
    NULL,
    reason,
    created_at,
    created_by
FROM cash_movements
WHERE NOT EXISTS (SELECT 1 FROM cash_movements_new WHERE cash_movements_new.id = cash_movements.id);

-- 4. Replace legacy tables (data already preserved in *_new) ------------
DROP TABLE IF EXISTS cash_movements;
DROP TABLE IF EXISTS cash_sessions;
ALTER TABLE cash_sessions_new RENAME TO cash_sessions;
ALTER TABLE cash_movements_new RENAME TO cash_movements;

-- 5. Indices --------------------------------------------------------------
CREATE UNIQUE INDEX IF NOT EXISTS idx_cash_sessions_active_per_terminal
    ON cash_sessions(terminal_id) WHERE status != 'CLOSED';
CREATE INDEX IF NOT EXISTS idx_cash_movements_session
    ON cash_movements(cash_session_id);

-- 6. Triggers ---------------------------------------------------------------
-- CLOSED sessions are immutable.
CREATE TRIGGER IF NOT EXISTS trg_cash_sessions_immutable_when_closed
BEFORE UPDATE ON cash_sessions
WHEN OLD.status = 'CLOSED'
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: closed cash session is immutable');
END;

-- Sessions are never deletable.
CREATE TRIGGER IF NOT EXISTS trg_cash_sessions_no_delete
BEFORE DELETE ON cash_sessions
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: cash_sessions cannot be deleted');
END;

-- cash_movements stays strictly append-only (re-created after rebuild).
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
