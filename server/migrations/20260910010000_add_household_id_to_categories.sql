-- Scope categories to a household (#65). Categories are currently global — every household would
-- see and could edit everyone else's. There's no real dev data worth preserving through this
-- reshape, so wipe both tables (transactions first, since it references categories) rather than
-- trying to backfill a household_id that doesn't exist yet for existing rows.
TRUNCATE TABLE transactions, categories RESTART IDENTITY CASCADE;

-- Uniqueness on name_en/name_fr becomes per-household instead of global — two households can each
-- have their own "Groceries".
ALTER TABLE categories DROP CONSTRAINT categories_name_en_key;
ALTER TABLE categories DROP CONSTRAINT categories_name_fr_key;

ALTER TABLE categories ADD COLUMN household_id INTEGER NOT NULL REFERENCES households (id);

ALTER TABLE categories ADD CONSTRAINT categories_household_id_name_en_key UNIQUE (household_id, name_en);
ALTER TABLE categories ADD CONSTRAINT categories_household_id_name_fr_key UNIQUE (household_id, name_fr);

-- Lets `transactions (household_id, category_id)` reference this table compositely in a follow-up
-- migration, so a transaction can't be pointed at another household's category.
ALTER TABLE categories ADD CONSTRAINT categories_household_id_id_key UNIQUE (household_id, id);

CREATE INDEX idx_categories_household_id ON categories (household_id);
