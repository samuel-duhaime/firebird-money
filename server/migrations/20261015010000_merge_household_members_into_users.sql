-- Merges `household_members` into `users` (#111). It was split out so a user could belong to more
-- than one household, but that was dropped (one household per user, since #65), which left it a
-- 1:1 extension of `users`.

-- The household this user belongs to and the role they hold in it. Both NULL until onboarding.
ALTER TABLE users ADD COLUMN household_id INTEGER REFERENCES households (id);
ALTER TABLE users ADD COLUMN household_role TEXT
    CHECK (household_role IN ('family_manager', 'family_member'));
ALTER TABLE users ADD CONSTRAINT users_household_role_set_with_household
    CHECK ((household_id IS NULL) = (household_role IS NULL));

-- Target of the transactions composite FK below, same reasoning as the one it replaces on
-- household_members.
ALTER TABLE users ADD CONSTRAINT users_household_id_id_key UNIQUE (household_id, id);

UPDATE users
SET household_id = household_members.household_id,
    household_role = household_members.type
FROM household_members
WHERE household_members.user_id = users.id;

-- Which user a transaction is attributed to, replacing `household_member_id`. The composite FK
-- keeps the guarantee that it's someone in the transaction's own household, and also blocks
-- removing a user from their household while they still have transactions there.
ALTER TABLE transactions ADD COLUMN user_id INTEGER REFERENCES users (id);

UPDATE transactions
SET user_id = household_members.user_id
FROM household_members
WHERE household_members.id = transactions.household_member_id;

ALTER TABLE transactions ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE transactions ADD CONSTRAINT transactions_household_user_fkey
    FOREIGN KEY (household_id, user_id) REFERENCES users (household_id, id);

-- Also drops `transactions_household_member_fkey`.
ALTER TABLE transactions DROP COLUMN household_member_id;

DROP TABLE household_members;
