-- Replaces the free-text transactions.account placeholder with a real account_id FK (#12).
-- Truncated rather than backfilled — still dev data, same move as the merchant_id migration
-- (20261006120200_add_merchant_id_to_transactions.sql). CASCADE also empties transaction_tags.
TRUNCATE TABLE transactions RESTART IDENTITY CASCADE;

ALTER TABLE transactions DROP COLUMN account;

ALTER TABLE transactions ADD COLUMN account_id INTEGER NOT NULL;

-- Composite, like the category FK: the account must belong to the transaction's own household.
-- No ON DELETE CASCADE — deleting an account is blocked while transactions still use it.
ALTER TABLE transactions ADD CONSTRAINT transactions_household_account_fkey
    FOREIGN KEY (household_id, account_id) REFERENCES accounts (household_id, id);

CREATE INDEX idx_transactions_household_id_account_id ON transactions (household_id, account_id);
