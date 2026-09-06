-- A user belongs to exactly one household, period — not "one household per household_id", which
-- is all the original UNIQUE (household_id, user_id) enforced (it still allowed the same user to
-- join any number of *different* households). Tightening this to a single UNIQUE (user_id) is
-- what #65 relies on to treat "the caller's household" as a single value instead of a list.
ALTER TABLE household_members DROP CONSTRAINT household_members_household_id_user_id_key;
ALTER TABLE household_members ADD CONSTRAINT household_members_user_id_key UNIQUE (user_id);
