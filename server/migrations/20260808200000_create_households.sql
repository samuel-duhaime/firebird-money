-- A household groups the accounts/finances of a family. It carries no identity of its own
-- (no email, no login) — that lives on `users`, connected via `household_members`.
CREATE TABLE households (
    id SERIAL PRIMARY KEY,

    -- When the row was inserted into the database.
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
