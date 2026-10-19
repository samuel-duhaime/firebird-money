-- The user's UI language, so it follows them across devices instead of living in one browser's
-- storage. NULL means they've never picked one: the client keeps whatever language it's showing.
ALTER TABLE settings ADD COLUMN language TEXT
    CONSTRAINT settings_language_valid CHECK (language IN ('en', 'fr'));
