-- Core transactions table.
--
-- `category_id` is a plain integer for now (no foreign key): the `categories`
-- lookup table is being designed in a separate PR. Once it lands, a
-- `REFERENCES categories(id)` constraint should be added here via a follow-up
-- migration.
CREATE TABLE transactions (
    -- Row id. Also what the `/transactions/{id}` endpoint will look up by.
    id BIGSERIAL PRIMARY KEY,

    -- Day the transaction occurred (no time component needed).
    date DATE NOT NULL,

    -- Merchant / payee, e.g. "STARBUCKS STORE #4521".
    merchant TEXT NOT NULL,

    -- Money as NUMERIC, never FLOAT, to avoid rounding errors.
    -- Convention (see budget-file-to-transaction skill): expenses are positive.
    amount NUMERIC(12, 2) NOT NULL,

    -- Points at a category (no FK yet, see note above).
    category_id INTEGER NOT NULL,

    -- Which account/user this belongs to, e.g. "User 1". Plain text for now;
    -- may become its own table if accounts need more structure later.
    account TEXT NOT NULL,

    -- When the row was inserted into the database (not the transaction date).
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Speeds up date-range queries and "order by date", e.g. a monthly view.
CREATE INDEX idx_transactions_date ON transactions (date);

-- Speeds up per-category filters/aggregates, e.g. "total spent on Groceries".
CREATE INDEX idx_transactions_category_id ON transactions (category_id);
