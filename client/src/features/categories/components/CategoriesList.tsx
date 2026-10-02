import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useCategories } from '../hooks/use-categories';
import { useCategoryGroups } from '../hooks/use-category-groups';
import { CategoryTypeSection } from './CategoryTypeSection';
import type { Category, CategoryGroup } from '../utils/types';
import './CategoriesList.css';

/** The order sections are shown in — matches the order `type` appears in the backend's default
 * seed data (income first, then expense groups, then transfer groups). */
const TYPES: CategoryGroup['type'][] = ['income', 'expense', 'transfer'];

/** A stable empty-array reference for a type with no groups yet — see `NO_CATEGORIES` in
 * `CategoryTypeSection` for why this needs to be stable rather than a fresh `?? []` each time. */
const NO_GROUPS: CategoryGroup[] = [];

type CategoriesListProps = {
  onNewGroup: (type: CategoryGroup['type']) => void;
  onEditGroup: (group: CategoryGroup) => void;
  onNewCategory: (groupId: number) => void;
  onEditCategory: (category: Category) => void;
};

export const CategoriesList = ({
  onNewGroup,
  onEditGroup,
  onNewCategory,
  onEditCategory,
}: CategoriesListProps) => {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage ?? i18n.language;
  const {
    data: categoryGroups,
    isPending: groupsPending,
    isError: groupsError,
  } = useCategoryGroups();
  const {
    data: categories,
    isPending: categoriesPending,
    isError: categoriesError,
  } = useCategories();

  // Grouped once per `categoryGroups` change rather than re-filtered on every render for each of
  // the 3 sections — a fresh array identity every render would otherwise look like a real change
  // to `useDragReorder`'s `sourceItems` effect downstream, in `CategoryTypeSection`.
  const groupsByType = useMemo(() => {
    const map = new Map<CategoryGroup['type'], CategoryGroup[]>();
    for (const group of categoryGroups ?? []) {
      const list = map.get(group.type);
      if (list) list.push(group);
      else map.set(group.type, [group]);
    }
    return map;
  }, [categoryGroups]);

  if (groupsPending || categoriesPending) {
    return <p>{t('settings.categories.loading')}</p>;
  }
  if (groupsError || categoriesError) {
    return <p>{t('settings.categories.error')}</p>;
  }

  return (
    <div className="categories-sections">
      {TYPES.map((type) => (
        <CategoryTypeSection
          key={type}
          type={type}
          groups={groupsByType.get(type) ?? NO_GROUPS}
          categories={categories}
          language={language}
          onNewGroup={onNewGroup}
          onEditGroup={onEditGroup}
          onNewCategory={onNewCategory}
          onEditCategory={onEditCategory}
        />
      ))}
    </div>
  );
};
