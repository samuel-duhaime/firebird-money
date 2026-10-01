import { useEffect, useRef, useState } from 'react';
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { ApiError } from '../../../lib/api-client';
import { useCreateTag } from '../hooks/use-create-tag';
import { useUpdateTag } from '../hooks/use-update-tag';
import type { Tag } from '../utils/types';
import '../../transactions/components/AddTransactionModal.css';
import './TagModal.css';

const DEFAULT_COLOR = '#2F80ED';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

type TagModalProps = {
  tag?: Tag;
  onClose: () => void;
};

export const TagModal = ({ tag, onClose }: TagModalProps) => {
  const { t } = useTranslation();
  const [name, setName] = useState(tag?.name ?? '');
  const [color, setColor] = useState(tag?.color ?? DEFAULT_COLOR);
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  const firstFieldRef = useRef<HTMLInputElement>(null);

  const createTagMutation = useCreateTag();
  const updateTagMutation = useUpdateTag();
  const isPending = createTagMutation.isPending || updateTagMutation.isPending;

  useEffect(() => {
    firstFieldRef.current?.focus();
  }, []);

  const handleKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      onClose();
      return;
    }

    if (e.key !== 'Tab') return;

    const focusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR) ??
        [],
    );
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };

  const handleNameChange = (e: ChangeEvent<HTMLInputElement>) => {
    setError('');
    setName(e.target.value);
  };

  const handleColorChange = (e: ChangeEvent<HTMLInputElement>) => {
    setColor(e.target.value);
  };

  const handleSubmit = () => {
    if (!name.trim()) {
      setError(t('settings.tags.modal.required'));
      return;
    }

    const onError = (mutationError: unknown) => {
      setError(
        mutationError instanceof ApiError && mutationError.status === 409
          ? t('settings.tags.modal.duplicateName')
          : t('settings.tags.modal.failed'),
      );
    };

    if (tag) {
      updateTagMutation.mutate(
        { id: tag.id, patch: { name: name.trim(), color } },
        { onSuccess: onClose, onError },
      );
    } else {
      createTagMutation.mutate(
        { name: name.trim(), color },
        { onSuccess: onClose, onError },
      );
    }
  };

  return (
    <div
      className="modal-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        className="tag-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="tag-modal-title"
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header">
          <h2 id="tag-modal-title">
            {tag
              ? t('settings.tags.modal.editTitle')
              : t('settings.tags.modal.newTitle')}
          </h2>

          <button
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label={t('transactions.add.close', 'Close')}
          >
            ×
          </button>
        </div>

        <div className="modal-body">
          <label htmlFor="tag-name">
            {t('settings.tags.modal.colorAndName')}
          </label>
          <div className="tag-modal-color-and-name">
            <input
              type="color"
              className="tag-modal-color-input"
              value={color}
              onChange={handleColorChange}
              aria-label={t('settings.tags.modal.color')}
            />
            <input
              ref={firstFieldRef}
              id="tag-name"
              type="text"
              value={name}
              onChange={handleNameChange}
            />
          </div>

          {error && (
            <p className="modal-error" role="alert">
              {error}
            </p>
          )}
        </div>

        <div className="modal-footer">
          <button
            type="button"
            className="modal-button modal-button--cancel"
            onClick={onClose}
          >
            {t('settings.tags.modal.cancel')}
          </button>

          <button
            type="button"
            className="modal-button modal-button--primary"
            onClick={handleSubmit}
            disabled={isPending}
          >
            {t('settings.tags.modal.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
