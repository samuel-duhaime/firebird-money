-- The `email` UNIQUE constraint on `users` is case-sensitive, so "Sam@x.com" and "sam@x.com"
-- could otherwise become two separate accounts. Every write path already normalizes to lowercase
-- (`normalize_email` on sign-in) except direct `POST /users`; this closes the gap at the database
-- level regardless of caller, and gives `get_by_email`'s `lower(email) = lower($1)` lookup an
-- index to use instead of a sequential scan.

-- Reconciles any case-variant duplicates that already exist before the index below is created —
-- the index creation fails outright if it isn't. Keeps the lowest id per lower(email) as
-- canonical. `sessions`/`login_tokens` cascade-delete with their user; `household_members` does
-- not, so it's moved to the canonical account first (dropping a moved row only if the canonical
-- account is already a member of that same household).
DO $$
DECLARE
    dup RECORD;
BEGIN
    FOR dup IN
        SELECT lower(email) AS email_lower, min(id) AS canonical_id
        FROM users
        GROUP BY lower(email)
        HAVING count(*) > 1
    LOOP
        UPDATE household_members
        SET user_id = dup.canonical_id
        WHERE user_id IN (
                  SELECT id FROM users
                  WHERE lower(email) = dup.email_lower AND id <> dup.canonical_id
              )
          AND NOT EXISTS (
                  SELECT 1 FROM household_members existing
                  WHERE existing.household_id = household_members.household_id
                    AND existing.user_id = dup.canonical_id
              );

        -- Whatever's left on a duplicate is a membership that collided with one the canonical
        -- account already had; there's nothing to move it to.
        DELETE FROM household_members
        WHERE user_id IN (
                  SELECT id FROM users
                  WHERE lower(email) = dup.email_lower AND id <> dup.canonical_id
              );

        DELETE FROM users WHERE lower(email) = dup.email_lower AND id <> dup.canonical_id;
    END LOOP;
END $$;

CREATE UNIQUE INDEX users_email_lower_key ON users (lower(email));
