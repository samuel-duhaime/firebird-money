-- Replaces free-text transactions.merchant with a real merchant_id FK (#14). Truncated rather than
-- backfilled — confirmed this is still dev data, same move the earlier household_id migration made
-- (20260910020000_add_household_id_to_transactions.sql). CASCADE also empties transaction_tags,
-- which references transactions and didn't exist yet when that earlier migration was written.
TRUNCATE TABLE transactions RESTART IDENTITY CASCADE;

-- Kept forever as the immutable historical record of what the bank/import actually said; never
-- user-editable after creation (unlike merchant_id, which the user can repoint at any time).
ALTER TABLE transactions RENAME COLUMN merchant TO original_statement;

ALTER TABLE transactions ADD COLUMN merchant_id INTEGER NOT NULL REFERENCES merchants (id);

CREATE INDEX idx_transactions_merchant_id ON transactions (merchant_id);

-- Composite index for the per-merchant transaction_count query (see merchants::repository::list) —
-- merchant_id alone can't be used the way category_id is for categories' count, because a common
-- merchant's id isn't unique to one household, so every count must also filter on household_id.
CREATE INDEX idx_transactions_household_id_merchant_id ON transactions (household_id, merchant_id);
