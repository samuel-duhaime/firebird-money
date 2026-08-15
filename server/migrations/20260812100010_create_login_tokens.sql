-- One-time magic-link tokens issued by `POST /auth/request-login` and spent by `GET /auth/verify`.
--
-- Only the SHA-256 hash of the token is stored: the raw value lives in the emailed link and
-- nowhere else, so a leaked database dump can't be replayed as a login.
CREATE TABLE login_tokens (
    id SERIAL PRIMARY KEY,

    -- Cascades so deleting a user (e.g. account deletion) doesn't get blocked by their old
    -- sign-in attempts.
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,

    -- SHA-256 of the raw token, hex encoded.
    token_hash TEXT NOT NULL UNIQUE,

    -- Short lived by design; `GET /auth/verify` refuses anything past this.
    expires_at TIMESTAMPTZ NOT NULL,

    -- Set the moment the token is spent, so the same link can't be used twice.
    used_at TIMESTAMPTZ,

    -- When the row was inserted into the database.
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Verify looks tokens up by hash only; the index comes free with the UNIQUE above.
CREATE INDEX login_tokens_user_id_idx ON login_tokens (user_id);
