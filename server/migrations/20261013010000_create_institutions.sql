-- Institutions: the bank, credit union, or brokerage an account is held at, e.g. "Desjardins",
-- "Wealthsimple" (#16). Same shape as merchants: an institution can be *common* — shared by every
-- household, so the usual Canadian banks don't need to be re-typed by everyone — or a household's
-- own custom one for anything not on that list. `household_id IS NULL` marks a common institution.
CREATE TABLE institutions (
    id SERIAL PRIMARY KEY,

    -- NULL = common institution, shared by every household. Non-null = this household's own.
    household_id INTEGER REFERENCES households (id),

    name TEXT NOT NULL,

    -- Other names worth matching on in the "Add account" search, without changing what's shown —
    -- e.g. "National Bank" is also found by typing "Banque Nationale". Empty for custom ones.
    aliases TEXT[] NOT NULL DEFAULT '{}',

    -- Shown under the name in the "Add account" picker. Optional for custom institutions.
    website_url TEXT,

    -- Path/URL to a logo image. NULL for now, same as merchants.
    logo_url TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT institutions_name_not_blank CHECK (name <> '')
);

-- Same pair of partial unique indexes as merchants: custom names unique per household, common names
-- unique globally (a plain UNIQUE(household_id, name) would never catch two NULL household_ids).
CREATE UNIQUE INDEX idx_institutions_household_id_name ON institutions (household_id, name)
    WHERE household_id IS NOT NULL;
CREATE UNIQUE INDEX idx_institutions_common_name ON institutions (name) WHERE household_id IS NULL;

CREATE INDEX idx_institutions_household_id ON institutions (household_id);

-- The common institutions are seeded at server startup from institutions::defaults, not here — same
-- reasoning as the common merchants (see 20261006120000_create_merchants.sql).
