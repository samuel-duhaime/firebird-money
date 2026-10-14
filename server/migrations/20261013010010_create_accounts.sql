-- Accounts: a household's bank accounts, credit cards, investment accounts, ... (#12). Every
-- account is held at exactly one institution (common or the household's own custom one) and always
-- belongs to exactly one household. No balance or sync state yet — just what's needed to attach
-- transactions to a real account instead of free text.
CREATE TABLE accounts (
    id SERIAL PRIMARY KEY,

    household_id INTEGER NOT NULL REFERENCES households (id),

    -- A plain FK: a common institution's household_id is NULL, so a composite FK can't express
    -- "common or this household's own". Checked in accounts::repository instead, the same way
    -- transactions.merchant_id is.
    institution_id INTEGER NOT NULL REFERENCES institutions (id),

    -- What the household calls it, e.g. "Compte conjoint".
    name TEXT NOT NULL,

    type TEXT NOT NULL,

    -- The last few characters of the account number, shown as "(…5012)". Optional.
    last_four TEXT,

    -- ISO 4217 code.
    currency TEXT NOT NULL DEFAULT 'CAD',

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT accounts_name_not_blank CHECK (name <> ''),
    CONSTRAINT accounts_type_valid CHECK (
        type IN ('checking', 'savings', 'credit_card', 'investment', 'loan', 'mortgage', 'cash', 'other')
    ),
    CONSTRAINT accounts_last_four_valid CHECK (last_four ~ '^[0-9A-Za-z]{1,4}$'),
    CONSTRAINT accounts_currency_valid CHECK (currency ~ '^[A-Z]{3}$'),

    -- Target of transactions' composite FK, so a transaction can't point at another household's
    -- account — same reasoning as categories_household_id_id_key.
    CONSTRAINT accounts_household_id_id_key UNIQUE (household_id, id)
);

CREATE INDEX idx_accounts_institution_id ON accounts (institution_id);
