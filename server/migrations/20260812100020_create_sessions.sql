-- Logged-in sessions. The session cookie carries the raw token; only its SHA-256 hash is stored
-- here, same reasoning as `login_tokens`.
CREATE TABLE sessions (
    id SERIAL PRIMARY KEY,

    -- Cascades so deleting a user (e.g. account deletion) doesn't get blocked by their sessions.
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,

    -- SHA-256 of the raw session token, hex encoded.
    token_hash TEXT NOT NULL UNIQUE,

    -- Sessions are absolute: past this the cookie stops working and the user signs in again.
    expires_at TIMESTAMPTZ NOT NULL,

    -- When the row was inserted into the database.
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX sessions_user_id_idx ON sessions (user_id);
