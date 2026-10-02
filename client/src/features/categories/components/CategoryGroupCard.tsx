import { useState } from 'react';
import type { DragEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faAngleDown,
  faAngleUp,
  faGripVertical,
  faPlus,
} from '@fortawesome/free-solid-svg-icons';
import { useDeleteCategory } from '../hooks/use-delete-category';
import { useDeleteCategoryGroup } from '../hooks/use-delete-category-group';
import { useReorderCategories } from '../hooks/use-reorder-categories';
import { useDragReorder } from '../../../lib/use-drag-reorder';
import {
  deleteCategoryFailedToast,
  deleteCategoryGroupFailedToast,
  reorderCategoriesFailedToast,
} from '../../../lib/toast';
import type { Category, CategoryGroup } from '../utils/types';
import '../../../components/MoveButtons.css';
import './CategoriesList.css';

type CategoryGroupCardProps = {
  group: CategoryGroup;
  categories: Category[];
  language: string;
  onEditGroup: () => void;
  onEditCategory: (category: Category) => void;
  onNewCategory: (groupId: number) => void;
  /** Drag props for this group's own row in its section's list — group-level drag state lives one
   * level up, in `CategoryTypeSection`. `onRowDragOver`/`onRowDrop` go on the card's own root
   * element, `onHandleDragStart`/`onHandleDragEnd` on its drag handle. */
  onRowDragOver: (event: DragEvent<HTMLLIElement>) => void;
  onRowDrop: (event: DragEvent<HTMLLIElement>) => void;
  onHandleDragStart: (event: DragEvent<HTMLSpanElement>) => void;
  onHandleDragEnd: () => void;
  /** Keyboard/button equivalent of dragging this group within its section — the mouse-only drag
   * handle above has no keyboard or screen-reader path otherwise. */
  onMoveUp: () => void;
  onMoveDown: () => void;
  canMoveUp: boolean;
  canMoveDown: boolean;
};

export const CategoryGroupCard = ({
  group,
  categories,
  language,
  onEditGroup,
  onEditCategory,
  onNewCategory,
  onRowDragOver,
  onRowDrop,
  onHandleDragStart,
  onHandleDragEnd,
  onMoveUp,
  onMoveDown,
  canMoveUp,
  canMoveDown,
}: CategoryGroupCardProps) => {
  const { t } = useTranslation();
  const deleteCategoryMutation = useDeleteCategory();
  const deleteCategoryGroupMutation = useDeleteCategoryGroup();
  const reorderCategoriesMutation = useReorderCategories();
  const [confirmingDeleteGroup, setConfirmingDeleteGroup] = useState(false);
  const [confirmingDeleteCategoryId, setConfirmingDeleteCategoryId] =
    useState<number | null>(null);

  const {
    items,
    handleDragStart,
    handleDragOver,
    handleDrop,
    handleDragEnd,
    moveUp,
    moveDown,
  } = useDragReorder(
    categories,
    (category) => category.id,
    reorderCategoriesMutation,
    reorderCategoriesFailedToast,
  );

  const handleDeleteCategory = (id: number) => {
    deleteCategoryMutation.mutate(id, {
      onSuccess: () => setConfirmingDeleteCategoryId(null),
      onError: deleteCategoryFailedToast,
    });
  };

  const handleDeleteGroup = () => {
    deleteCategoryGroupMutation.mutate(group.id, {
      onError: deleteCategoryGroupFailedToast,
    });
  };

  return (
    <li
      className="category-group-card"
      onDragOver={onRowDragOver}
      onDrop={onRowDrop}
    >
      <div className="category-group-header">
        <span
          className="category-group-drag-handle"
          aria-hidden="true"
          draggable
          onDragStart={onHandleDragStart}
          onDragEnd={onHandleDragEnd}
        >
          <FontAwesomeIcon icon={faGripVertical} />
        </span>
        <div className="move-buttons">
          <button
            type="button"
            className="move-button"
            onClick={onMoveUp}
            disabled={!canMoveUp}
            aria-label={t('settings.categories.moveUp', {
              name: language === 'fr' ? group.name_fr : group.name_en,
            })}
          >
            <FontAwesomeIcon icon={faAngleUp} />
          </button>
          <button
            type="button"
            className="move-button"
            onClick={onMoveDown}
            disabled={!canMoveDown}
            aria-label={t('settings.categories.moveDown', {
              name: language === 'fr' ? group.name_fr : group.name_en,
            })}
          >
            <FontAwesomeIcon icon={faAngleDown} />
          </button>
        </div>
        <span className="category-group-name">
          {language === 'fr' ? group.name_fr : group.name_en}
        </span>

        {confirmingDeleteGroup ? (
          <div className="category-group-actions">
            <span className="category-group-confirm-text">
              {t('settings.categories.deleteGroupConfirm', {
                name: language === 'fr' ? group.name_fr : group.name_en,
              })}
            </span>
            <button
              type="button"
              className="category-group-action"
              onClick={() => setConfirmingDeleteGroup(false)}
            >
              {t('settings.categories.groupModal.cancel')}
            </button>
            <button
              type="button"
              className="category-group-action category-group-action--danger"
              onClick={handleDeleteGroup}
              disabled={deleteCategoryGroupMutation.isPending}
            >
              {t('settings.categories.deleteConfirmButton')}
            </button>
          </div>
        ) : (
          <div className="category-group-actions">
            <button
              type="button"
              className="category-group-action"
              onClick={onEditGroup}
            >
              {t('settings.categories.edit')}
            </button>
            <button
              type="button"
              className="category-group-action category-group-action--danger"
              onClick={() => setConfirmingDeleteGroup(true)}
            >
              {t('settings.categories.delete')}
            </button>
          </div>
        )}
      </div>

      <ul className="categories-list">
        {items.map((category, index) => (
          <li
            key={category.id}
            className="categories-list-row"
            onDragOver={handleDragOver(index)}
            onDrop={handleDrop}
          >
            <span
              className="categories-list-drag-handle"
              draggable
              onDragStart={handleDragStart(index)}
              onDragEnd={handleDragEnd}
              aria-hidden="true"
            >
              <FontAwesomeIcon icon={faGripVertical} />
            </span>
            <div className="move-buttons">
              <button
                type="button"
                className="move-button"
                onClick={() => moveUp(index)}
                disabled={index === 0}
                aria-label={t('settings.categories.moveUp', {
                  name: language === 'fr' ? category.name_fr : category.name_en,
                })}
              >
                <FontAwesomeIcon icon={faAngleUp} />
              </button>
              <button
                type="button"
                className="move-button"
                onClick={() => moveDown(index)}
                disabled={index === items.length - 1}
                aria-label={t('settings.categories.moveDown', {
                  name: language === 'fr' ? category.name_fr : category.name_en,
                })}
              >
                <FontAwesomeIcon icon={faAngleDown} />
              </button>
            </div>
            <span className="categories-list-name">
              {language === 'fr' ? category.name_fr : category.name_en}
            </span>

            {confirmingDeleteCategoryId === category.id ? (
              <div className="categories-list-actions">
                <span className="category-group-confirm-text">
                  {t('settings.categories.deleteCategoryConfirm', {
                    name:
                      language === 'fr'
                        ? category.name_fr
                        : category.name_en,
                  })}
                </span>
                <button
                  type="button"
                  className="categories-list-action"
                  onClick={() => setConfirmingDeleteCategoryId(null)}
                >
                  {t('settings.categories.groupModal.cancel')}
                </button>
                <button
                  type="button"
                  className="categories-list-action categories-list-action--danger"
                  onClick={() => handleDeleteCategory(category.id)}
                  disabled={deleteCategoryMutation.isPending}
                >
                  {t('settings.categories.deleteConfirmButton')}
                </button>
              </div>
            ) : (
              <div className="categories-list-actions">
                <button
                  type="button"
                  className="categories-list-action"
                  onClick={() => onEditCategory(category)}
                >
                  {t('settings.categories.edit')}
                </button>
                <button
                  type="button"
                  className="categories-list-action categories-list-action--danger"
                  onClick={() => setConfirmingDeleteCategoryId(category.id)}
                >
                  {t('settings.categories.delete')}
                </button>
              </div>
            )}
          </li>
        ))}
      </ul>

      <button
        type="button"
        className="category-create-row-button"
        onClick={() => onNewCategory(group.id)}
      >
        <FontAwesomeIcon icon={faPlus} />
        {t('settings.categories.newCategory')}
      </button>
    </li>
  );
};
