-- A member's picture. In its own table so the image bytes stay out of every query on `users`, and
-- in the database so backups include it without any file storage to set up.
CREATE TABLE user_pictures (
    user_id INTEGER PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    content_type TEXT NOT NULL,
    data BYTEA NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT user_pictures_content_type_valid CHECK (
        content_type IN ('image/png', 'image/jpeg', 'image/webp')
    )
);

-- data-loss-reviewed: no screen ever set avatar_url or showed it. Only a direct PATCH /users/{id}
-- could have filled it, with a link to an image hosted elsewhere, which pictures now replace.
ALTER TABLE users DROP COLUMN avatar_url;
