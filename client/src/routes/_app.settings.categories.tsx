import { useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { CategoriesList } from '../features/categories/components/CategoriesList';
import { CategoryModal } from '../features/categories/components/CategoryModal';
import { CategoryGroupModal } from '../features/categories/components/CategoryGroupModal';
import type {
  Category,
  CategoryGroup,
} from '../features/categories/utils/types';
import './_app.settings.css';

type CategoryModalState = Category | { groupId: number } | null;
type CategoryGroupModalState =
  | CategoryGroup
  | { type: CategoryGroup['type'] }
  | null;

const CategoriesPage = () => {
  const { t } = useTranslation();
  const [categoryModal, setCategoryModal] = useState<CategoryModalState>(
    null,
  );
  const [groupModal, setGroupModal] = useState<CategoryGroupModalState>(null);

  return (
    <div className="settings-panel">
      <div className="settings-panel-header">
        <h2>{t('settings.categories.heading')}</h2>
      </div>

      <CategoriesList
        onNewGroup={(type) => setGroupModal({ type })}
        onEditGroup={setGroupModal}
        onNewCategory={(groupId) => setCategoryModal({ groupId })}
        onEditCategory={setCategoryModal}
      />

      {groupModal && (
        <CategoryGroupModal
          group={groupModal}
          onClose={() => setGroupModal(null)}
        />
      )}

      {categoryModal && (
        <CategoryModal
          category={categoryModal}
          onClose={() => setCategoryModal(null)}
        />
      )}
    </div>
  );
};

export const Route = createFileRoute('/_app/settings/categories')({
  component: CategoriesPage,
});
