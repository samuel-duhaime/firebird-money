-- The FK added in 20260707021117 was validated inline, which takes a
-- SHARE ROW EXCLUSIVE lock and scans the whole table, blocking writes for the
-- duration (flagged by review). Re-add it as NOT VALID + a separate
-- VALIDATE CONSTRAINT, which only needs SHARE UPDATE EXCLUSIVE while scanning.
-- (Editing the original migration isn't an option: it's already applied.)
ALTER TABLE transactions DROP CONSTRAINT transactions_category_id_fkey;

ALTER TABLE transactions
    ADD CONSTRAINT transactions_category_id_fkey
    FOREIGN KEY (category_id) REFERENCES categories (id) NOT VALID;

ALTER TABLE transactions
    VALIDATE CONSTRAINT transactions_category_id_fkey;
