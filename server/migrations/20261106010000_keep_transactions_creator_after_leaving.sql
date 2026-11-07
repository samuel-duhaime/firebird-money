-- Who created a transaction becomes history, like `reviewed_by_user_id`: only the plain
-- `transactions_user_id_fkey` stays, so it keeps pointing at that user after they leave or are
-- removed from the household, instead of blocking it. The transactions themselves stay with the
-- household.
ALTER TABLE transactions DROP CONSTRAINT transactions_household_user_fkey;
