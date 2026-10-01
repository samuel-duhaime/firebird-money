import { useEffect, useRef, useState } from 'react';
import type { DragEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faGripVertical } from '@fortawesome/free-solid-svg-icons';
import { useTags } from '../hooks/use-tags';
import { useDeleteTag } from '../hooks/use-delete-tag';
import { useReorderTags } from '../hooks/use-reorder-tags';
import {
  deleteTagFailedToast,
  reorderTagsFailedToast,
} from '../../../lib/toast';
import type { Tag } from '../utils/types';
import './TagsList.css';

type TagsListProps = {
  onEdit: (tag: Tag) => void;
};

export const TagsList = ({ onEdit }: TagsListProps) => {
  const { t } = useTranslation();
  const { data: tags, isPending, isError } = useTags();
  const deleteTagMutation = useDeleteTag();
  const reorderTagsMutation = useReorderTags();
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<number | null>(
    null,
  );

  // A local copy the drag gesture reorders live, independent of the query cache until the drop
  // lands and the server confirms it — kept in sync with `tags` the rest of the time (e.g. after
  // a create/delete elsewhere), but never while `isDraggingRef` is true, so an in-flight gesture
  // isn't clobbered by a refetch landing mid-drag.
  const [items, setItems] = useState<Tag[]>([]);
  const isDraggingRef = useRef(false);
  const draggedIndexRef = useRef<number | null>(null);

  useEffect(() => {
    if (tags && !isDraggingRef.current) setItems(tags);
  }, [tags]);

  const handleDelete = (id: number) => {
    deleteTagMutation.mutate(id, {
      onSuccess: () => setConfirmingDeleteId(null),
      onError: deleteTagFailedToast,
    });
  };

  const handleDragStart =
    (index: number) => (event: DragEvent<HTMLSpanElement>) => {
      draggedIndexRef.current = index;
      isDraggingRef.current = true;
      event.dataTransfer.effectAllowed = 'move';
    };

  const handleDragOver =
    (index: number) => (event: DragEvent<HTMLLIElement>) => {
      event.preventDefault();
      const draggedIndex = draggedIndexRef.current;
      if (draggedIndex === null || draggedIndex === index) return;
      setItems((previous) => {
        const next = [...previous];
        const [moved] = next.splice(draggedIndex, 1);
        next.splice(index, 0, moved);
        return next;
      });
      draggedIndexRef.current = index;
    };

  const handleDrop = (event: DragEvent<HTMLLIElement>) => {
    event.preventDefault();
  };

  const handleDragEnd = () => {
    isDraggingRef.current = false;
    draggedIndexRef.current = null;
    reorderTagsMutation.mutate(
      items.map((tag) => tag.id),
      {
        onError: () => {
          reorderTagsFailedToast();
          if (tags) setItems(tags);
        },
      },
    );
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
