-- Merchants: who a transaction was paid to/by, e.g. "Starbucks", "IGA" (#14). Unlike categories or
-- tags, a merchant can be *common* — shared by every household, not owned by one — so a handful of
-- everyday merchants (Amazon, Netflix, Starbucks, ...) don't need to be re-typed by every household
-- that uses them. `household_id IS NULL` marks a common merchant; everything else belongs to
-- exactly one household, same as categories/tags.
CREATE TABLE merchants (
    id SERIAL PRIMARY KEY,

    -- NULL = common merchant, shared by every household. Non-null = this household's own.
    household_id INTEGER REFERENCES households (id),

    name TEXT NOT NULL,

    -- Alternate spellings worth matching on (typed search, and raw bank-statement text) without
    -- changing what's actually shown anywhere — e.g. "McDonald's" stays the real name, but
    -- "McDonalds" (no apostrophe, how most people type it) still finds it. See
    -- merchants::defaults::DefaultMerchant and repository::best_match. Empty for most merchants,
    -- including every custom one (not user-settable in this PR).
    aliases TEXT[] NOT NULL DEFAULT '{}',

    -- Path/URL to a logo image. Left NULL for every merchant in this migration (including the
    -- common ones below) — sourcing real logo assets is future work, not part of #14.
    logo_url TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT merchants_name_not_blank CHECK (name <> '')
);
-- No default-category column here: categories are per-household, so a shared merchant row can't
-- hold one category_id valid for every household. Instead, `GET /merchants` computes a
-- recommended_category_id per request from each household's own transaction history — see
-- repository::RECOMMENDED_CATEGORY_SUBQUERY.

-- Custom merchant names are unique per household; common merchant names are unique globally. Two
-- separate partial indexes, because a plain UNIQUE(household_id, name) would let every household
-- "re-add" Amazon — Postgres never treats two NULLs as equal, so that constraint wouldn't catch it.
CREATE UNIQUE INDEX idx_merchants_household_id_name ON merchants (household_id, name)
    WHERE household_id IS NOT NULL;
CREATE UNIQUE INDEX idx_merchants_common_name ON merchants (name) WHERE household_id IS NULL;

CREATE INDEX idx_merchants_household_id ON merchants (household_id);

-- The common merchants themselves are NOT seeded here — unlike this repo's other migrations, which
-- are pure schema. They're defined in merchants::defaults::DEFAULT_MERCHANTS and inserted by
-- merchants::repository::seed_defaults, called once at server startup (idempotent — safe on every
-- restart), the same way category_groups/tags define their starter data in Rust rather than SQL.
-- Not seeded at household creation like those, since common merchants are global singletons, not
-- per-household copies.
