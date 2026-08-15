-- Short code a second person types to join an existing household during onboarding (#26).
-- Generated when the household is created, and never shown to anyone outside it.
ALTER TABLE households ADD COLUMN join_code TEXT;

-- Backfill existing rows (dev data only) so the NOT NULL below can be applied.
UPDATE households
SET join_code = upper(substr(md5(random()::text || id::text), 1, 8))
WHERE join_code IS NULL;

ALTER TABLE households ALTER COLUMN join_code SET NOT NULL;
ALTER TABLE households ADD CONSTRAINT households_join_code_key UNIQUE (join_code);
