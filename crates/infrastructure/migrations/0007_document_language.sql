-- Baraka UltraPOS migration 0007: explicit document language (additive only).
--
-- Section 17 mandates that document_language capture the sale-time language
-- selection. Pre-existing rows default to 'FR', which honestly preserves
-- history (every prior document was rendered French under the hardcoded
-- behavior) without fabricating uncaptured information. The CHECK repeats
-- the IPC contract ("AR" | "FR", case-sensitive).

ALTER TABLE documents ADD COLUMN language TEXT NOT NULL DEFAULT 'FR'
    CHECK (language IN ('AR', 'FR'));
