-- Who a transaction belongs to: one member of the household, or NULL for Shared, as on accounts. It
-- starts as its account's owner and can be changed on its own, e.g. a shared purchase on a personal
-- card. Not `user_id`, which records who created the transaction.
ALTER TABLE transactions ADD COLUMN owner_user_id INTEGER;

-- Same composite FK as `accounts_household_owner_fkey`: the owner stays in the transaction's own
-- household, and deleting the user makes the transaction Shared. Removing them from the household
-- is handled in members::repository::remove.
ALTER TABLE transactions ADD CONSTRAINT transactions_household_owner_fkey
    FOREIGN KEY (household_id, owner_user_id) REFERENCES users (household_id, id)
    ON DELETE SET NULL (owner_user_id);

CREATE INDEX idx_transactions_owner_user_id ON transactions (owner_user_id);

-- Every existing transaction takes its account's owner.
UPDATE transactions t
SET owner_user_id = a.owner_user_id
FROM accounts a
WHERE a.id = t.account_id;
