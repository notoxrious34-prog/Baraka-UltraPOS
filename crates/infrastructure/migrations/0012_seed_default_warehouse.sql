-- Baraka UltraPOS migration 0012: seed canonical default warehouse (TASK-BRK-025).
--
-- Context (TASK-BRK-024 audit): table `warehouses` is empty on fresh databases
-- because no migration seeds it and no UI can invoke `warehouses_create` yet.
-- An active warehouse is an invariant prerequisite for cash sessions and sales,
-- so a fresh install is inoperable without one. This migration inserts the
-- canonical primary warehouse exactly once, and only when the table is empty:
-- - Fresh databases: one row, DEP-01 / Dépôt Principal, active + default.
-- - Existing databases (any rows present): strict no-op via WHERE NOT EXISTS.
-- - The fixed id is a valid RFC 4122 UUID string (v7 variant bits), so any
--   downstream UUID parsing succeeds without panic.
-- - Omitted columns (`revision`, `origin_device_id`) fall back to their
--   schema defaults (0 / NULL) per 0003_stock_engine_schema.sql.

INSERT INTO warehouses (
    id,
    code,
    name,
    is_active,
    is_default,
    created_at,
    updated_at
)
SELECT
    '01950000-0000-7000-8000-000000000001',
    'DEP-01',
    'Dépôt Principal',
    1,
    1,
    CURRENT_TIMESTAMP,
    CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM warehouses);
