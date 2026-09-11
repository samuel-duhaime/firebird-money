-- Groups categories under a household-owned umbrella (e.g. "Food & Dining" containing "Groceries",
-- "Restaurants & Bars"), and moves `type` (income/expense/transfer) up onto the group — every
-- category in a group shares its type, so there's nothing left to keep in sync between the two.
--
-- A separate table (rather than a self-referential is_group_category/main_category_id on
-- `categories`) means neither "a leaf's group must actually be a group" nor "a group can't itself
-- have a group" needs a trigger to enforce — they're structurally impossible, since groups live in
-- a different table with its own id space. It also means a transaction can never reference a group
-- directly: `transactions.category_id` only ever points into `categories`.
CREATE TABLE category_groups (
    id SERIAL PRIMARY KEY,

    household_id INTEGER NOT NULL REFERENCES households (id),

    name_en TEXT NOT NULL,
    name_fr TEXT NOT NULL,

    type TEXT NOT NULL CHECK (type IN ('income', 'expense', 'transfer')),

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (household_id, name_en),
    UNIQUE (household_id, name_fr),
    -- Lets `categories (household_id, group_id)` reference this table compositely below.
    UNIQUE (household_id, id)
);

CREATE INDEX idx_category_groups_household_id ON category_groups (household_id);

-- `categories` is still empty at this point (truncated by an earlier migration, and seeding now
-- happens from application code on household creation, not from a migration), so these can be
-- applied directly with no backfill.
ALTER TABLE categories DROP COLUMN type;

ALTER TABLE categories ADD COLUMN group_id INTEGER NOT NULL REFERENCES category_groups (id);
ALTER TABLE categories ADD CONSTRAINT categories_household_id_group_id_fkey
    FOREIGN KEY (household_id, group_id) REFERENCES category_groups (household_id, id);

CREATE INDEX idx_categories_group_id ON categories (group_id);
