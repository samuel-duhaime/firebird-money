import { useEffect, useRef, useState } from 'react';
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { ApiError } from '../../../lib/api-client';
import { useCreateCategoryGroup } from '../hooks/use-create-category-group';
import { useUpdateCategoryGroup } from '../hooks/use-update-category-group';
import type { CategoryGroup } from '../utils/types';
import '../../transactions/components/AddTransactionModal.css';
import './CategoryGroupModal.css';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

type CategoryGroupModalProps = {
  /** An existing group to edit, or the `type` a new group should start in (preselected from
   * whichever section's "Create group" button was clicked). */
  group: CategoryGroup | { type: CategoryGroup['type'] };
  onClose: () => void;
};

export const CategoryGroupModal = ({
  group,
  onClose,
}: CategoryGroupModalProps) => {
  const { t } = useTranslation();
  const existing = 'id' in group ? group : undefined;
  const [nameEn, setNameEn] = useState(existing?.name_en ?? '');
  const [nameFr, setNameFr] = useState(existing?.name_fr ?? '');
  const [type, setType] = useState<CategoryGroup['type']>(group.type);
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  const firstFieldRef = useRef<HTMLInputElement>(null);

  const createGroupMutation = useCreateCategoryGroup();
  const updateGroupMutation = useUpdateCategoryGroup();
  const isPending =
    createGroupMutation.isPending || updateGroupMutation.isPending;

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

  const handleNameEnChange = (e: ChangeEvent<HTMLInputElement>) => {
    setError('');
    setNameEn(e.target.value);
  };

  const handleNameFrChange = (e: ChangeEvent<HTMLInputElement>) => {
    setError('');
    setNameFr(e.target.value);
  };

  const handleSubmit = () => {
    const trimmedEn = nameEn.trim();
    const trimmedFr = nameFr.trim();
    if (!trimmedEn && !trimmedFr) {
      setError(t('settings.categories.groupModal.required'));
      return;
    }

    const onError = (mutationError: unknown) => {
      setError(
        mutationError instanceof ApiError && mutationError.status === 409
          ? t('settings.categories.groupModal.duplicateName')
          : t('settings.categories.groupModal.failed'),
      );
    };

    // A household that only uses one language can leave the other name blank — it's filled in
    // with whatever was given, rather than forcing a translation nobody asked for.
    const patch = {
      name_en: trimmedEn || trimmedFr,
      name_fr: trimmedFr || trimmedEn,
      type,
    };

    if (existing) {
      updateGroupMutation.mutate(
        { id: existing.id, patch },
        { onSuccess: onClose, onError },
      );
    } else {
      createGroupMutation.mutate(patch, { onSuccess: onClose, onError });
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
        className="category-group-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="category-group-modal-title"
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header">
          <h2 id="category-group-modal-title">
            {existing
              ? t('settings.categories.groupModal.editTitle')
              : t('settings.categories.groupModal.newTitle')}
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
          <label htmlFor="category-group-name-en">
            {t('settings.categories.groupModal.nameEn')}
          </label>
          <input
            ref={firstFieldRef}
            id="category-group-name-en"
            type="text"
            value={nameEn}
            onChange={handleNameEnChange}
          />

          <label htmlFor="category-group-name-fr">
            {t('settings.categories.groupModal.nameFr')}
          </label>
          <input
            id="category-group-name-fr"
            type="text"
            value={nameFr}
            onChange={handleNameFrChange}
          />

          <label htmlFor="category-group-type">
            {t('settings.categories.groupModal.type')}
          </label>
          <select
            id="category-group-type"
            value={type}
            onChange={(e) =>
              setType(e.target.value as CategoryGroup['type'])
            }
          >
            <option value="income">
              {t('settings.categories.types.income')}
            </option>
            <option value="expense">
              {t('settings.categories.types.expense')}
            </option>
            <option value="transfer">
              {t('settings.categories.types.transfer')}
            </option>
          </select>

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
            {t('settings.categories.groupModal.cancel')}
          </button>

          <button
            type="button"
            className="modal-button modal-button--primary"
            onClick={handleSubmit}
            disabled={isPending}
          >
            {t('settings.categories.groupModal.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
