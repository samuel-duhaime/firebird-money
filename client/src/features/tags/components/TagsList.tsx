import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useTags } from '../hooks/use-tags';
import { useDeleteTag } from '../hooks/use-delete-tag';
import { deleteTagFailedToast } from '../../../lib/toast';
import type { Tag } from '../utils/types';
import './TagsList.css';

type TagsListProps = {
  onEdit: (tag: Tag) => void;
};

export const TagsList = ({ onEdit }: TagsListProps) => {
  const { t } = useTranslation();
  const { data: tags, isPending, isError } = useTags();
  const deleteTagMutation = useDeleteTag();
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<number | null>(
    null,
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
      {tags.map((tag) => (
        <li key={tag.id} className="tags-list-row">
          <span
            className="tags-list-color"
            style={{ backgroundColor: tag.color }}
            aria-hidden="true"
          />
          <span className="tags-list-name">{tag.name}</span>
          <span className="tags-list-meta">
            {t('settings.tags.transactionCount', { count: 0 })}
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
