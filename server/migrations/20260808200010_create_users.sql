-- A person with their own login identity. How a user relates to a household (manager or family
-- member) is recorded in `household_members`, not here, since the same person could be connected
-- to more than one household.
CREATE TABLE users (
    id SERIAL PRIMARY KEY,

    -- Login identity. Required even for Google-only signups, since it's how we identify the
    -- account everywhere else (invites, support, etc).
    email TEXT NOT NULL UNIQUE,

    -- Google OAuth subject id, when the account is linked to Google. Absent for accounts created
    -- another way.
    google_id TEXT UNIQUE,

    -- Verification state of this account.
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('verified', 'pending', 'suspended')),

    first_name TEXT,
    last_name TEXT,
    avatar_url TEXT,

    -- When the row was inserted into the database.
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
