-- Lets a household drag-and-drop reorder its category groups (see `PATCH /category-groups/reorder`)
-- and the categories within each group (see `PATCH /categories/reorder`), instead of always
-- showing them in creation order.
ALTER TABLE category_groups ADD COLUMN sort_order INTEGER;
ALTER TABLE categories ADD COLUMN sort_order INTEGER;

-- Backfill existing rows with their current (creation-order) position, per household.
UPDATE category_groups
SET sort_order = ranked.rn - 1
FROM (
    SELECT id, ROW_NUMBER() OVER (PARTITION BY household_id ORDER BY id) AS rn
    FROM category_groups
) AS ranked
WHERE category_groups.id = ranked.id;

-- Backfilled per group (not per household): a category's position only ever needs to be
-- meaningful relative to the other categories in its own group, never across groups.
UPDATE categories
SET sort_order = ranked.rn - 1
FROM (
    SELECT id, ROW_NUMBER() OVER (PARTITION BY group_id ORDER BY id) AS rn
    FROM categories
) AS ranked
WHERE categories.id = ranked.id;

ALTER TABLE category_groups ALTER COLUMN sort_order SET NOT NULL;
ALTER TABLE categories ALTER COLUMN sort_order SET NOT NULL;
