import { useTranslation } from 'react-i18next';
import { useCategories } from '../hooks/use-categories';
import { useCategoryGroups } from '../hooks/use-category-groups';
import { CategoryTypeSection } from './CategoryTypeSection';
import type { Category, CategoryGroup } from '../utils/types';
import './CategoriesList.css';

/** The order sections are shown in — matches the order `type` appears in the backend's default
 * seed data (income first, then expense groups, then transfer groups). */
const TYPES: CategoryGroup['type'][] = ['income', 'expense', 'transfer'];

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
          groups={categoryGroups.filter((group) => group.type === type)}
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
