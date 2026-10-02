import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useReorderCategoryGroups } from '../hooks/use-reorder-category-groups';
import { useDragReorder } from '../../../lib/use-drag-reorder';
import { reorderCategoryGroupsFailedToast } from '../../../lib/toast';
import { CategoryGroupCard } from './CategoryGroupCard';
import type { Category, CategoryGroup } from '../utils/types';

/** A stable empty-array reference for a group with no categories — `?? []` would otherwise hand
 * `useDragReorder` a newly allocated array every render, defeating its memoized-input check. */
const NO_CATEGORIES: Category[] = [];

type CategoryTypeSectionProps = {
  type: CategoryGroup['type'];
  groups: CategoryGroup[];
  categories: Category[];
  language: string;
  onNewGroup: (type: CategoryGroup['type']) => void;
  onEditGroup: (group: CategoryGroup) => void;
  onEditCategory: (category: Category) => void;
  onNewCategory: (groupId: number) => void;
};

export const CategoryTypeSection = ({
  type,
  groups,
  categories,
  language,
  onNewGroup,
  onEditGroup,
  onEditCategory,
  onNewCategory,
}: CategoryTypeSectionProps) => {
  const { t } = useTranslation();
  const reorderCategoryGroupsMutation = useReorderCategoryGroups();

  const {
    items,
    handleDragStart,
    handleDragOver,
    handleDrop,
    handleDragEnd,
    moveUp,
    moveDown,
  } = useDragReorder(
    groups,
    (group) => group.id,
    reorderCategoryGroupsMutation,
    reorderCategoryGroupsFailedToast,
  );

  // Grouped once per `categories` change, not re-filtered for every group on every render — see
  // the `useDragReorder` fix this accompanies: a fresh array identity every render looks like a
  // real change to its `sourceItems` effect, which would reset an in-flight drag or save.
  const categoriesByGroupId = useMemo(() => {
    const map = new Map<number, Category[]>();
    for (const category of categories) {
      const list = map.get(category.group_id);
      if (list) list.push(category);
      else map.set(category.group_id, [category]);
    }
    return map;
  }, [categories]);

  return (
    <section className="category-type-section">
      <div className="category-type-section-header">
        <h3>{t(`settings.categories.types.${type}`)}</h3>
        <button
          type="button"
          className="settings-panel-primary-button"
          onClick={() => onNewGroup(type)}
        >
          {t('settings.categories.newGroup')}
        </button>
      </div>

      {items.length === 0 ? (
        <p>{t('settings.categories.emptySection')}</p>
      ) : (
        <ul className="category-group-list">
          {items.map((group, index) => (
            <CategoryGroupCard
              key={group.id}
              group={group}
              categories={categoriesByGroupId.get(group.id) ?? NO_CATEGORIES}
              language={language}
              onEditGroup={() => onEditGroup(group)}
              onEditCategory={onEditCategory}
              onNewCategory={onNewCategory}
              onRowDragOver={handleDragOver(index)}
              onRowDrop={handleDrop}
              onHandleDragStart={handleDragStart(index)}
              onHandleDragEnd={handleDragEnd}
              onMoveUp={() => moveUp(index)}
              onMoveDown={() => moveDown(index)}
              canMoveUp={index > 0}
              canMoveDown={index < items.length - 1}
            />
          ))}
        </ul>
      )}
    </section>
  );
};
