-- Where a transaction came from: typed in by hand (`manual`) or added from a bank statement
-- the user imported and reviewed (`import`). Set once on insert and never changed afterward.
-- Every existing row predates statement import's rebuild, so they all default to `manual`.
ALTER TABLE transactions ADD COLUMN source TEXT NOT NULL DEFAULT 'manual'
    CONSTRAINT transactions_source_valid CHECK (source IN ('manual', 'import'));
