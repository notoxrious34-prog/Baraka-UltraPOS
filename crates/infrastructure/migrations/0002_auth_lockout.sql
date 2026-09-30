-- Baraka UltraPOS migration 0002: lockout-tracking columns on users.
-- Applied through the embedded checksum-tracked runner; 0001 is untouched.

ALTER TABLE users ADD COLUMN failed_login_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN locked_until TEXT;
