-- Transactions list: the optional Owner column, visible like the others until the user hides it.
ALTER TABLE settings ADD COLUMN show_owner_column BOOLEAN NOT NULL DEFAULT true;
