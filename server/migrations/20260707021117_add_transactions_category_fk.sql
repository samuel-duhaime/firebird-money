-- Now that categories exist and are seeded, enforce that every transaction points at a real one.
ALTER TABLE transactions
    ADD CONSTRAINT transactions_category_id_fkey
    FOREIGN KEY (category_id) REFERENCES categories (id);
