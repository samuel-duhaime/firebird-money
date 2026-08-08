-- Connects a user to a household with a role. This is the join the old `users.household_id`
-- column couldn't express: a user can belong to more than one household (e.g. an adult child who
-- is a family_member in their parents' household and a family_manager of their own), each with
-- its own role.
CREATE TABLE household_members (
    id SERIAL PRIMARY KEY,

    household_id INTEGER NOT NULL REFERENCES households (id),
    user_id INTEGER NOT NULL REFERENCES users (id),

    -- Role this user plays in this specific household.
    type TEXT NOT NULL CHECK (type IN ('family_manager', 'family_member')),

    -- When the row was inserted into the database.
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- A user has exactly one role per household.
    UNIQUE (household_id, user_id)
);
