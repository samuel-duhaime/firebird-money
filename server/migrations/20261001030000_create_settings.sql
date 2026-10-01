-- Per-user display preferences, e.g. which optional columns show on the transactions list. One
-- row per user, created (at defaults) the first time they change anything — see the `settings`
-- repository's upsert.
CREATE TABLE settings (
    user_id INTEGER PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,

    -- Transactions list: optional columns the user can hide.
    show_category_column BOOLEAN NOT NULL DEFAULT true,
    show_tags_column BOOLEAN NOT NULL DEFAULT true,
    show_account_column BOOLEAN NOT NULL DEFAULT true
);
