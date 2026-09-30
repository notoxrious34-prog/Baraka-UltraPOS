-- Baraka UltraPOS migration 0005: customer module indexes (additive only).
--
-- Reconciliation against 0001 (see completion report):
-- - customers: ALREADY contains name, phone, address, is_active,
--   created_at, updated_at, credit_policy, credit_limit_centimes,
--   current_balance_centimes. No column to add.
-- - customer_ledger: append-only UPDATE/DELETE triggers exist in 0001 and
--   cover DEBIT and CREDIT identically (no entry-type distinction at the
--   trigger level). No change.
-- - Genuinely missing: a UNIQUE phone index and a name search index.
-- Never DROP TABLE or rebuild: existing data is preserved.

CREATE UNIQUE INDEX IF NOT EXISTS idx_customers_phone_unique
    ON customers(phone) WHERE phone IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_customers_name ON customers(name);
