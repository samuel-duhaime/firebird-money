import { useTranslation } from 'react-i18next';
import { useReorderCategoryGroups } from '../hooks/use-reorder-category-groups';
import { useDragReorder } from '../hooks/use-drag-reorder';
import { reorderCategoryGroupsFailedToast } from '../../../lib/toast';
import { CategoryGroupCard } from './CategoryGroupCard';
import type { Category, CategoryGroup } from '../utils/types';

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
  } = useDragReorder(
    groups,
    (group) => group.id,
    reorderCategoryGroupsMutation,
    reorderCategoryGroupsFailedToast,
  );

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
              categories={categories.filter(
                (category) => category.group_id === group.id,
              )}
              language={language}
              onEditGroup={() => onEditGroup(group)}
              onEditCategory={onEditCategory}
              onNewCategory={onNewCategory}
              onRowDragOver={handleDragOver(index)}
              onRowDrop={handleDrop}
              onHandleDragStart={handleDragStart(index)}
              onHandleDragEnd={handleDragEnd}
            />
          ))}
        </ul>
      )}
    </section>
  );
};
