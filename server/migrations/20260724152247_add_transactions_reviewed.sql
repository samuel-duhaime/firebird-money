-- Transactions created interactively (or manually) default to reviewed; automated
-- imports explicitly set this to false so they can be found and reviewed later.
ALTER TABLE transactions ADD COLUMN reviewed BOOLEAN NOT NULL DEFAULT true;
