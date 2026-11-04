-- Who an account belongs to: one member of the household, or NULL for Shared with all of it. Only a
-- label: everyone in the household still sees every account.
ALTER TABLE accounts ADD COLUMN owner_user_id INTEGER;

-- The composite FK keeps the owner in the account's own household; a NULL owner skips the check.
-- Deleting the user makes the account Shared. Removing them from the household is handled in
-- members::repository::remove, which makes their accounts Shared in the same statement.
ALTER TABLE accounts ADD CONSTRAINT accounts_household_owner_fkey
    FOREIGN KEY (household_id, owner_user_id) REFERENCES users (household_id, id)
    ON DELETE SET NULL (owner_user_id);

CREATE INDEX idx_accounts_owner_user_id ON accounts (owner_user_id);

-- Existing accounts stay Shared (NULL): nothing records who created them, and each one can be given
-- an owner from its edit form.
