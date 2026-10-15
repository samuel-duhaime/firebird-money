-- Households used to be seeded with a placeholder "Member #1" tag. Every member now gets a tag
-- named after them when they join instead (first name plus last initial, e.g. "John S." — see
-- `tags::defaults::member_tag_name`), so bring existing households in line.

-- First, rename the placeholder after the household's family manager (the earliest one, in case
-- there are several), unless the manager has no name yet or the household already has that tag.
UPDATE tags
SET name = renamed.name
FROM (
    SELECT DISTINCT ON (household_id)
           household_id,
           first_name || COALESCE(' ' || upper(left(NULLIF(last_name, ''), 1)) || '.', '') AS name
    FROM users
    WHERE household_role = 'family_manager'
      AND first_name IS NOT NULL
      AND first_name <> ''
    ORDER BY household_id, id
) AS renamed
WHERE tags.household_id = renamed.household_id
  AND tags.name = 'Member #1'
  AND NOT EXISTS (
      SELECT 1 FROM tags existing
      WHERE existing.household_id = renamed.household_id AND existing.name = renamed.name
  );

-- Then give every other named member their own tag, appended to the household's order. Members
-- whose tag already exists (including the manager just renamed above, or a second "John S.") are
-- left alone.
INSERT INTO tags (household_id, name, color, sort_order)
SELECT member.household_id,
       member.name,
       '#9B51E0',
       (SELECT COALESCE(MAX(sort_order), -1) FROM tags WHERE household_id = member.household_id)
           + ROW_NUMBER() OVER (PARTITION BY member.household_id ORDER BY member.id)
FROM (
    SELECT DISTINCT ON (household_id, name) id, household_id, name
    FROM (
        SELECT id,
               household_id,
               first_name || COALESCE(' ' || upper(left(NULLIF(last_name, ''), 1)) || '.', '') AS name
        FROM users
        WHERE household_id IS NOT NULL
          AND first_name IS NOT NULL
          AND first_name <> ''
    ) AS named
    WHERE NOT EXISTS (
        SELECT 1 FROM tags existing
        WHERE existing.household_id = named.household_id AND existing.name = named.name
    )
    ORDER BY household_id, name, id
) AS member;
