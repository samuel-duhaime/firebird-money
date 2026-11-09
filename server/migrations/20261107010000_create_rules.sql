-- Rules: "if a transaction matches these conditions, then apply these actions", run top to bottom
-- (lowest `sort_order` first) on statement import, and on a household's existing transactions when
-- the user asks. See `server/src/features/rules`.
CREATE TABLE rules (
    id SERIAL PRIMARY KEY,
    household_id INTEGER NOT NULL REFERENCES households (id),

    -- `rules::model::RuleConditions` and `RuleActions`, always read and written whole. Stored as
    -- JSON rather than in child tables with foreign keys: a merchant, category, tag or account a rule
    -- names can still be deleted, and the rule then just stops matching it (a condition) or skips it
    -- (an action), instead of a cascade emptying a condition and making the rule match everything.
    conditions JSONB NOT NULL,
    actions JSONB NOT NULL,

    -- This household's chosen order for its rules — lower runs first.
    sort_order INTEGER NOT NULL,

    -- When the rule was last applied to existing transactions, and how many it changed then.
    last_applied_at TIMESTAMPTZ,
    last_applied_count INTEGER NOT NULL DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_rules_household_id ON rules (household_id);
