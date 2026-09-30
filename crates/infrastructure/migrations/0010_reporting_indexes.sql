-- Baraka UltraPOS migration 0010: reporting read-path indexes (TASK-BRK-012A).
--
-- Covering indexes for the eight read-only report queries. No tables,
-- columns, triggers, or data change here: indexes only. Purely additive
-- and re-runnable via IF NOT EXISTS.
--
-- NOTE (supersedes the interim 0010 draft from the prior turn, which was
-- never applied to any persistent database — only throwaway test files):
-- the index set below is exactly the four covering indexes mandated by
-- the TASK-BRK-012A specification.

CREATE INDEX IF NOT EXISTS idx_documents_reporting
    ON documents(status, type, created_at);

CREATE INDEX IF NOT EXISTS idx_document_items_reporting
    ON document_items(document_id, product_id);

CREATE INDEX IF NOT EXISTS idx_stock_movements_reporting
    ON stock_movements(created_at, movement_type, warehouse_id, product_id);

CREATE INDEX IF NOT EXISTS idx_audit_log_reporting
    ON audit_log(timestamp, action, user_id);
