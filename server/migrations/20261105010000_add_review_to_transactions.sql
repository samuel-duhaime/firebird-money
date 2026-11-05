-- Who a transaction waits on for a review: one member of the household, or NULL for Anyone. Only
-- meaningful while `reviewed` is false.
ALTER TABLE transactions ADD COLUMN review_assignee_user_id INTEGER;

-- Same composite FK as `transactions_household_owner_fkey`: the assignee stays in the transaction's
-- own household, and deleting the user hands the review to Anyone. Removing them from the
-- household is handled in members::repository::remove.
ALTER TABLE transactions ADD CONSTRAINT transactions_household_review_assignee_fkey
    FOREIGN KEY (household_id, review_assignee_user_id) REFERENCES users (household_id, id)
    ON DELETE SET NULL (review_assignee_user_id);

-- Who marked it reviewed, and when. NULL on a transaction that was never reviewed by anyone, e.g.
-- one added by hand or imported, which start out reviewed. A plain FK, not a same-household one:
-- it records history, so it stays when that member later leaves the household.
ALTER TABLE transactions ADD COLUMN reviewed_by_user_id INTEGER
    REFERENCES users (id) ON DELETE SET NULL;
ALTER TABLE transactions ADD COLUMN reviewed_at TIMESTAMPTZ;

-- The "Needs review" views only ever read unreviewed rows.
CREATE INDEX idx_transactions_needs_review ON transactions (household_id)
    WHERE NOT reviewed;

-- Every transaction so far was added by hand or reviewed during a statement import, so none needs a
-- review: they all start as "No one". Nobody marked them reviewed in the app, so who and when stay
-- empty.
UPDATE transactions SET reviewed = true WHERE NOT reviewed;
