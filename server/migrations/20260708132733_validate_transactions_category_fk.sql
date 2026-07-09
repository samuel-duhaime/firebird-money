-- The FK added in 20260707021117 was validated inline, which takes a
-- SHARE ROW EXCLUSIVE lock and scans the whole table, blocking writes for the
-- duration (flagged by review). Re-add it as NOT VALID here; a later
-- migration runs VALIDATE CONSTRAINT in its own transaction, since sqlx
-- wraps each migration file in a transaction and doing both steps in one
-- file would hold the lock for the validation scan too, defeating the point.
-- (Editing the original 20260707021117 migration isn't an option: it's
-- already applied.)
ALTER TABLE transactions DROP CONSTRAINT transactions_category_id_fkey;

ALTER TABLE transactions
    ADD CONSTRAINT transactions_category_id_fkey
    FOREIGN KEY (category_id) REFERENCES categories (id) NOT VALID;
