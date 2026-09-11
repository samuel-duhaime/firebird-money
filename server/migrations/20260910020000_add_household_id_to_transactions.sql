-- Scope transactions to a household, and attribute each one to the specific member who created it
-- (#65). `transactions` was truncated by the previous migration, so these NOT NULL columns need no
-- backfill.

-- Needed as the target of the composite FK below, same reasoning as
-- categories_household_id_id_key added in the previous migration.
ALTER TABLE household_members ADD CONSTRAINT household_members_household_id_id_key UNIQUE (household_id, id);

ALTER TABLE transactions ADD COLUMN household_id INTEGER NOT NULL REFERENCES households (id);

-- Which household member this transaction is attributed to — not the free-text `account` column,
-- which is a placeholder for a future *bank* account concept (see AddTransactionModal.tsx).
ALTER TABLE transactions ADD COLUMN household_member_id INTEGER NOT NULL REFERENCES household_members (id);

-- Replace the plain category_id FK with a composite one: a transaction can no longer reference a
-- category belonging to a different household.
ALTER TABLE transactions DROP CONSTRAINT transactions_category_id_fkey;
ALTER TABLE transactions ADD CONSTRAINT transactions_household_category_fkey
    FOREIGN KEY (household_id, category_id) REFERENCES categories (household_id, id);

-- Same guarantee for household_member_id: it must belong to the same household_id the transaction
-- itself carries, not some other household's member.
ALTER TABLE transactions ADD CONSTRAINT transactions_household_member_fkey
    FOREIGN KEY (household_id, household_member_id) REFERENCES household_members (household_id, id);

CREATE INDEX idx_transactions_household_id ON transactions (household_id);
