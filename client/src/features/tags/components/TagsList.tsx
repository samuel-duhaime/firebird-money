import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faAngleDown,
  faAngleUp,
  faGripVertical,
} from '@fortawesome/free-solid-svg-icons';
import { useTags } from '../hooks/use-tags';
import { useDeleteTag } from '../hooks/use-delete-tag';
import { useReorderTags } from '../hooks/use-reorder-tags';
import { useDragReorder } from '../../../lib/use-drag-reorder';
import {
  deleteTagFailedToast,
  reorderTagsFailedToast,
} from '../../../lib/toast';
import type { Tag } from '../utils/types';
import '../../../components/MoveButtons.css';
import './TagsList.css';

type TagsListProps = {
  onEdit: (tag: Tag) => void;
};

/** A stable empty-array reference for while `tags` hasn't loaded yet — `useDragReorder` needs an
 * array every render, and a fresh `?? []` each time would defeat its memoized-input check. */
const NO_TAGS: Tag[] = [];

export const TagsList = ({ onEdit }: TagsListProps) => {
  const { t } = useTranslation();
  const { data: tags, isPending, isError } = useTags();
  const deleteTagMutation = useDeleteTag();
  const reorderTagsMutation = useReorderTags();
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<number | null>(
    null,
  );

  const {
    items,
    handleDragStart,
    handleDragOver,
    handleDrop,
    handleDragEnd,
    moveUp,
    moveDown,
  } = useDragReorder(
    tags ?? NO_TAGS,
    (tag) => tag.id,
    reorderTagsMutation,
    reorderTagsFailedToast,
  );

  const handleDelete = (id: number) => {
    deleteTagMutation.mutate(id, {
      onSuccess: () => setConfirmingDeleteId(null),
      onError: deleteTagFailedToast,
    });
  };

  if (isPending) return <p>{t('settings.tags.loading')}</p>;
  if (isError) return <p>{t('settings.tags.error')}</p>;
  if (tags.length === 0) return <p>{t('settings.tags.empty')}</p>;

  return (
    <ul className="tags-list">
      {items.map((tag, index) => (
        <li
          key={tag.id}
          className="tags-list-row"
          onDragOver={handleDragOver(index)}
          onDrop={handleDrop}
        >
          <span
            className="tags-list-drag-handle"
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
              aria-label={t('settings.tags.moveUp', { name: tag.name })}
            >
              <FontAwesomeIcon icon={faAngleUp} />
            </button>
            <button
              type="button"
              className="move-button"
              onClick={() => moveDown(index)}
              disabled={index === items.length - 1}
              aria-label={t('settings.tags.moveDown', { name: tag.name })}
            >
              <FontAwesomeIcon icon={faAngleDown} />
            </button>
          </div>
          <span
            className="tags-list-color"
            style={{ backgroundColor: tag.color }}
            aria-hidden="true"
          />
          <span className="tags-list-name">{tag.name}</span>
          <span className="tags-list-meta">
            {t('settings.tags.transactionCount', {
              count: tag.transaction_count,
            })}
          </span>

          {confirmingDeleteId === tag.id ? (
            <div className="tags-list-confirm">
              <span className="tags-list-confirm-text">
                {t('settings.tags.deleteConfirm', { name: tag.name })}
              </span>
              <div className="tags-list-actions">
                <button
                  type="button"
                  className="tags-list-action"
                  onClick={() => setConfirmingDeleteId(null)}
                >
                  {t('settings.tags.modal.cancel')}
                </button>
                <button
                  type="button"
                  className="tags-list-action tags-list-action--danger"
                  onClick={() => handleDelete(tag.id)}
                  disabled={deleteTagMutation.isPending}
                >
                  {t('settings.tags.deleteConfirmButton')}
                </button>
              </div>
            </div>
          ) : (
            <div className="tags-list-actions">
              <button
                type="button"
                className="tags-list-action"
                onClick={() => onEdit(tag)}
              >
                {t('settings.tags.edit')}
              </button>
              <button
                type="button"
                className="tags-list-action tags-list-action--danger"
                onClick={() => setConfirmingDeleteId(tag.id)}
              >
                {t('settings.tags.delete')}
              </button>
            </div>
          )}
        </li>
      ))}
    </ul>
  );
};
