-- Lets a household drag-and-drop reorder its tags (see `PATCH /tags/reorder`), instead of always
-- showing them in creation order.
ALTER TABLE tags ADD COLUMN sort_order INTEGER;

-- Backfill existing rows with their current (creation-order) position, per household.
UPDATE tags
SET sort_order = ranked.rn - 1
FROM (
    SELECT id, ROW_NUMBER() OVER (PARTITION BY household_id ORDER BY id) AS rn
    FROM tags
) AS ranked
WHERE tags.id = ranked.id;

ALTER TABLE tags ALTER COLUMN sort_order SET NOT NULL;
