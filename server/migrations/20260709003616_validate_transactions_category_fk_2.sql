-- Runs in its own transaction, separate from the NOT VALID add in
-- 20260708132733, so the ACCESS EXCLUSIVE lock from that ADD CONSTRAINT is
-- released before this potentially slow validation scan starts. VALIDATE
-- CONSTRAINT only needs SHARE UPDATE EXCLUSIVE, which doesn't block writes.
ALTER TABLE transactions
    VALIDATE CONSTRAINT transactions_category_id_fkey;
