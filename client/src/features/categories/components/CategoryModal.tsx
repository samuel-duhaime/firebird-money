import { useEffect, useRef, useState } from 'react';
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { ApiError } from '../../../lib/api-client';
import { useCategoryGroups } from '../hooks/use-category-groups';
import { useCreateCategory } from '../hooks/use-create-category';
import { useUpdateCategory } from '../hooks/use-update-category';
import type { Category } from '../utils/types';
import '../../transactions/components/AddTransactionModal.css';
import './CategoryGroupModal.css';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

type CategoryModalProps = {
  /** An existing category to edit, or the `groupId` a new category should start in (preselected
   * from whichever group's "Create category" button was clicked). */
  category: Category | { groupId: number };
  onClose: () => void;
};

export const CategoryModal = ({ category, onClose }: CategoryModalProps) => {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage ?? i18n.language;
  const existing = 'id' in category ? category : undefined;
  const [nameEn, setNameEn] = useState(existing?.name_en ?? '');
  const [nameFr, setNameFr] = useState(existing?.name_fr ?? '');
  const [groupId, setGroupId] = useState(
    'groupId' in category ? category.groupId : category.group_id,
  );
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  const firstFieldRef = useRef<HTMLInputElement>(null);

  const { data: categoryGroups } = useCategoryGroups();
  const createCategoryMutation = useCreateCategory();
  const updateCategoryMutation = useUpdateCategory();
  const isPending =
    createCategoryMutation.isPending || updateCategoryMutation.isPending;

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
      setError(t('settings.categories.categoryModal.required'));
      return;
    }

    const onError = (mutationError: unknown) => {
      setError(
        mutationError instanceof ApiError && mutationError.status === 409
          ? t('settings.categories.categoryModal.duplicateName')
          : t('settings.categories.categoryModal.failed'),
      );
    };

    // A household that only uses one language can leave the other name blank — it's filled in
    // with whatever was given, rather than forcing a translation nobody asked for.
    const patch = {
      name_en: trimmedEn || trimmedFr,
      name_fr: trimmedFr || trimmedEn,
      group_id: groupId,
    };

    if (existing) {
      updateCategoryMutation.mutate(
        { id: existing.id, patch },
        { onSuccess: onClose, onError },
      );
    } else {
      createCategoryMutation.mutate(patch, { onSuccess: onClose, onError });
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
        aria-labelledby="category-modal-title"
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header">
          <h2 id="category-modal-title">
            {existing
              ? t('settings.categories.categoryModal.editTitle')
              : t('settings.categories.categoryModal.newTitle')}
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
          <label htmlFor="category-name-en">
            {t('settings.categories.categoryModal.nameEn')}
          </label>
          <input
            ref={firstFieldRef}
            id="category-name-en"
            type="text"
            value={nameEn}
            onChange={handleNameEnChange}
          />

          <label htmlFor="category-name-fr">
            {t('settings.categories.categoryModal.nameFr')}
          </label>
          <input
            id="category-name-fr"
            type="text"
            value={nameFr}
            onChange={handleNameFrChange}
          />

          <label htmlFor="category-group">
            {t('settings.categories.categoryModal.group')}
          </label>
          <select
            id="category-group"
            value={groupId}
            onChange={(e) => setGroupId(Number(e.target.value))}
          >
            {categoryGroups?.map((group) => (
              <option key={group.id} value={group.id}>
                {language === 'fr' ? group.name_fr : group.name_en}
              </option>
            ))}
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
            {t('settings.categories.categoryModal.cancel')}
          </button>

          <button
            type="button"
            className="modal-button modal-button--primary"
            onClick={handleSubmit}
            disabled={isPending}
          >
            {t('settings.categories.categoryModal.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
