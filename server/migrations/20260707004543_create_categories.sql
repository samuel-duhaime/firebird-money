-- Categories used to classify transactions, e.g. "Groceries", "Paycheck".
--
-- `transactions.category_id` is a plain integer for now (no foreign key yet).
-- Once this table is populated, a `REFERENCES categories(id)` constraint should
-- be added to `transactions` via a follow-up migration.
CREATE TABLE categories (
    -- Row id. Matches the type of `transactions.category_id` (INTEGER).
    id SERIAL PRIMARY KEY,

    -- Display name, English and French. Both required so the UI can switch
    -- language without a fallback lookup.
    name_en TEXT NOT NULL UNIQUE,
    name_fr TEXT NOT NULL UNIQUE,

    -- Whether this category represents money coming in, going out, or moving
    -- between accounts. Transaction amounts are always stored positive;
    -- direction comes from here.
    type TEXT NOT NULL CHECK (type IN ('income', 'expense', 'transfer')),

    -- When the row was inserted into the database.
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
