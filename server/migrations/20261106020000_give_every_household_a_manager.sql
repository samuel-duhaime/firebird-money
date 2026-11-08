-- Before roles had rules, a household's last manager could be demoted. Now only a manager can
-- change roles, delete the household or step down, so a household without one would be stuck:
-- make its longest-standing member its manager.
UPDATE users
SET household_role = 'family_manager'
WHERE id IN (
    SELECT DISTINCT ON (household_id) id
    FROM users
    WHERE household_id IS NOT NULL
      AND household_id NOT IN (
          SELECT household_id FROM users
          WHERE household_role = 'family_manager' AND household_id IS NOT NULL
      )
    ORDER BY household_id, id
);
