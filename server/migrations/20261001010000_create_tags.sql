-- Tags: free-form labels the user attaches to transactions for their own organization (e.g.
-- "Vacation 2026", "Reimbursable"), independent of category or account. A transaction can have
-- several tags, and a tag can apply to many transactions — see `transaction_tags` below.
CREATE TABLE tags (
    id SERIAL PRIMARY KEY,
    household_id INTEGER NOT NULL REFERENCES households (id),

    -- Single display name — unlike categories, tags are free-form and user-authored, so there's
    -- no name_en/name_fr pair to keep in sync.
    name TEXT NOT NULL,

    -- Hex color (e.g. "#2F80ED") shown as the tag's dot in the UI.
    color TEXT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (household_id, name)
);

CREATE INDEX idx_tags_household_id ON tags (household_id);

-- Join table connecting transactions to the tags attached to them. `tag_id` has no ON DELETE
-- CASCADE: deleting a tag still attached to a transaction should fail (like deleting an in-use
-- category), not silently strip it off every transaction. `transaction_id` cascades, since
-- deleting the transaction itself should take its tag links with it.
CREATE TABLE transaction_tags (
    transaction_id BIGINT NOT NULL REFERENCES transactions (id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags (id),
    PRIMARY KEY (transaction_id, tag_id)
);

CREATE INDEX idx_transaction_tags_tag_id ON transaction_tags (tag_id);
