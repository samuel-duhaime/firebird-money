-- Onboarding steps this user has finished, e.g. `{household,name}`. The client keeps sending them
-- back to `/onboarding` while any step is missing, so a step added later (extend the CHECK below
-- and the server's `ONBOARDING_STEPS`) is asked of everyone who hasn't done it — even people who
-- finished onboarding before it existed.
ALTER TABLE users ADD COLUMN onboarding_steps TEXT[] NOT NULL DEFAULT '{}'
    CHECK (onboarding_steps <@ ARRAY['name', 'household']);

-- Credit existing users with the steps they've effectively already done.
UPDATE users SET onboarding_steps = array_append(onboarding_steps, 'household')
WHERE EXISTS (SELECT 1 FROM household_members WHERE household_members.user_id = users.id);

UPDATE users SET onboarding_steps = array_append(onboarding_steps, 'name')
WHERE first_name IS NOT NULL;
