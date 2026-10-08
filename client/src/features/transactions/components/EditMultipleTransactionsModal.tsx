import { useEffect, useRef, useState } from 'react';
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { useCategories } from '../../categories/hooks/use-categories';
import { useMerchants } from '../../merchants/hooks/use-merchants';
import { TagPicker } from '../../tags/components/TagPicker';
import { useBulkUpdateTransactions } from '../hooks/use-bulk-update-transactions';
import { useBulkDeleteTransactions } from '../hooks/use-bulk-delete-transactions';
import {
  bulkDeleteTransactionsFailedToast,
  bulkDeleteTransactionsSucceededToast,
  bulkUpdateTransactionsFailedToast,
  noBulkChangesToast,
} from '../../../lib/toast';
import { CategoryPicker } from './CategoryPicker';
import { MerchantPicker } from './MerchantPicker';
import type { BulkTransactionPatch } from '../utils/api';
import './AddTransactionModal.css';
import './EditTransactionModal.css';
import './EditMultipleTransactionsModal.css';

type EditMultipleTransactionsModalProps = {
  transactionIds: number[];
  onClose: () => void;
  /** Called after a successful bulk save or delete, so the caller can clear the selection. */
  onSaved: () => void;
};

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

export const EditMultipleTransactionsModal = ({
  transactionIds,
  onClose,
  onSaved,
}: EditMultipleTransactionsModalProps) => {
  const { t, i18n } = useTranslation();
  const { data: categories } = useCategories();
  const { data: merchants } = useMerchants();
  const count = transactionIds.length;

  // Each field is `undefined` ("no change") until the user activates it. Activating commits to
  // sending a value on Save — there's no separate per-field auto-save like the single-transaction
  // panel, everything here is buffered until Save is clicked. merchant is a pick-from-a-list field
  // (like category), so its "no change" sentinel is `undefined` directly rather than an
  // active/text pair the way the old free-text merchant field needed.
  const [merchantId, setMerchantId] = useState<number | undefined>(undefined);
  const [dateActive, setDateActive] = useState(false);
  const [date, setDate] = useState('');
  const [categoryId, setCategoryId] = useState<number | undefined>(undefined);
  // Picking a tag here only ever *adds* it to each selected transaction's existing tags (see
  // `BulkTransactionPatch.tag_ids`) — there's no "no change" vs "clear" distinction to track like
  // the other fields, so a plain array (empty = nothing picked yet) is enough.
  const [tagIds, setTagIds] = useState<number[]>([]);
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  const dialogRef = useRef<HTMLDivElement>(null);
  const firstFieldRef = useRef<HTMLButtonElement>(null);
  // Captured once, synchronously, on first render — before the autofocus effect below moves
  // focus into the dialog — so closing can return keyboard focus to whatever opened it (the
  // transactions list stays mounted underneath the whole time this panel is open).
  const [returnFocusTarget] = useState<HTMLElement | null>(
    () => document.activeElement as HTMLElement | null,
  );

  const bulkUpdateMutation = useBulkUpdateTransactions();
  const bulkDeleteMutation = useBulkDeleteTransactions();

  useEffect(() => {
    firstFieldRef.current?.focus();
  }, []);

  const handleClose = () => {
    onClose();
    returnFocusTarget?.focus();
  };

  const getTrapFocusable = (): HTMLElement[] => {
    const dialogFocusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR) ??
        [],
    );
    const popovers = document.querySelectorAll(
      '.category-picker-popover, .merchant-picker-popover, .tag-picker-popover',
    );
    const popoverFocusable = Array.from(popovers).flatMap((popover) =>
      Array.from(popover.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)),
    );
    return [...dialogFocusable, ...popoverFocusable];
  };

  const handleKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      handleClose();
      return;
    }

    if (e.key !== 'Tab') return;

    const focusable = getTrapFocusable();
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

  const handleDateChange = (e: ChangeEvent<HTMLInputElement>) => {
    setDate(e.target.value);
  };

  const clearDate = () => {
    setDateActive(false);
    setDate('');
  };

  const selectedCategory = categories?.find((c) => c.id === categoryId);
  const categoryLabel =
    categoryId === undefined
      ? t('transactions.editMultiple.noChange')
      : selectedCategory
        ? i18n.language === 'fr'
          ? selectedCategory.name_fr
          : selectedCategory.name_en
        : t('transactions.add.selectCategory');

  const selectedMerchant = merchants?.find((m) => m.id === merchantId);
  const merchantLabel =
    merchantId === undefined
      ? t('transactions.editMultiple.noChange')
      : (selectedMerchant?.name ?? t('transactions.add.selectMerchant'));

  // A field counts as changed only once it actually holds a value — merely activating it (e.g.
  // clicking the Date trigger without picking a date yet) stays equivalent to "no change".
  const dateHasValue = dateActive && date !== '';

  const handleSave = () => {
    const patch: BulkTransactionPatch = {};
    if (merchantId !== undefined) patch.merchant_id = merchantId;
    if (dateHasValue) patch.date = date;
    if (categoryId !== undefined) patch.category_id = categoryId;
    if (tagIds.length > 0) patch.tag_ids = tagIds;

    if (Object.keys(patch).length === 0) {
      noBulkChangesToast();
      return;
    }

    bulkUpdateMutation.mutate(
      { ids: transactionIds, patch },
      {
        onSuccess: () => onSaved(),
        onError: () => bulkUpdateTransactionsFailedToast(),
      },
    );
  };

  const handleDelete = () => {
    bulkDeleteMutation.mutate(transactionIds, {
      onSuccess: () => {
        bulkDeleteTransactionsSucceededToast(count);
        onSaved();
      },
      onError: () => {
        bulkDeleteTransactionsFailedToast();
        setConfirmingDelete(false);
      },
    });
  };

  return (
    <div
      className="edit-transaction-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <div
        ref={dialogRef}
        className="edit-transaction-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="edit-multiple-title"
        onKeyDown={handleKeyDown}
      >
        <div className="edit-transaction-header">
          <button
            type="button"
            className="modal-close"
            onClick={handleClose}
            aria-label={t('transactions.add.close')}
          >
            ×
          </button>
        </div>

        <div className="edit-transaction-body">
          <h2 id="edit-multiple-title" className="edit-multiple-title">
            {t('transactions.editMultiple.title', { count })}
          </h2>

          <div className="edit-transaction-field">
            <label>{t('transactions.add.merchant')}</label>
            <div className="edit-multiple-field-control">
              <MerchantPicker
                merchantId={merchantId ?? null}
                label={merchantLabel}
                className={
                  merchantId !== undefined
                    ? 'modal-category-trigger'
                    : 'modal-category-trigger modal-category-trigger--placeholder'
                }
                onSelect={(id) => setMerchantId(id)}
                ariaLabel={t('transactions.add.merchant')}
              />
              {merchantId !== undefined && (
                <button
                  type="button"
                  className="edit-multiple-field-clear"
                  onClick={() => setMerchantId(undefined)}
                  aria-label={t('transactions.editMultiple.noChange')}
                >
                  ×
                </button>
              )}
            </div>
          </div>

          <div className="edit-transaction-field">
            <label htmlFor="bulk-edit-date">{t('transactions.add.date')}</label>
            {dateActive ? (
              <div className="edit-multiple-field-control">
                <input
                  id="bulk-edit-date"
                  type="date"
                  value={date}
                  onChange={handleDateChange}
                  autoFocus
                />
                <button
                  type="button"
                  className="edit-multiple-field-clear"
                  onClick={clearDate}
                  aria-label={t('transactions.editMultiple.noChange')}
                >
                  ×
                </button>
              </div>
            ) : (
              <button
                ref={firstFieldRef}
                type="button"
                className="edit-multiple-field-trigger"
                onClick={() => setDateActive(true)}
                aria-label={t('transactions.add.date')}
              >
                {t('transactions.editMultiple.noChange')}
              </button>
            )}
          </div>

          <div className="edit-transaction-field">
            <label>{t('transactions.add.category')}</label>
            <div className="edit-multiple-field-control">
              <CategoryPicker
                categoryId={categoryId ?? null}
                label={categoryLabel}
                className={
                  categoryId !== undefined
                    ? 'modal-category-trigger'
                    : 'modal-category-trigger modal-category-trigger--placeholder'
                }
                onSelect={(id) => setCategoryId(id)}
                ariaLabel={t('transactions.add.category')}
              />
              {categoryId !== undefined && (
                <button
                  type="button"
                  className="edit-multiple-field-clear"
                  onClick={() => setCategoryId(undefined)}
                  aria-label={t('transactions.editMultiple.noChange')}
                >
                  ×
                </button>
              )}
            </div>
          </div>

          <div className="edit-transaction-field">
            <label>
              {t('transactions.add.tags')}{' '}
              <span className="edit-multiple-field-hint">
                ({t('transactions.editMultiple.tagsHint')})
              </span>
            </label>
            <div className="edit-multiple-field-control">
              <TagPicker
                selectedTagIds={tagIds}
                onToggle={(tagId) =>
                  setTagIds((previous) =>
                    previous.includes(tagId)
                      ? previous.filter((id) => id !== tagId)
                      : [...previous, tagId],
                  )
                }
                triggerClassName="modal-category-trigger"
                ariaLabel={t('transactions.add.tags')}
              />
              {tagIds.length > 0 && (
                <button
                  type="button"
                  className="edit-multiple-field-clear"
                  onClick={() => setTagIds([])}
                  aria-label={t('transactions.editMultiple.noChange')}
                >
                  ×
                </button>
              )}
            </div>
          </div>
        </div>

        <div className="edit-transaction-footer">
          {confirmingDelete ? (
            <div className="delete-confirm">
              <span className="delete-confirm-text">
                {t('transactions.editMultiple.deleteConfirmTitle', { count })}{' '}
                {t('transactions.editMultiple.deleteConfirmDescription')}
              </span>
              <div className="modal-footer-actions">
                <button
                  type="button"
                  className="modal-button modal-button--cancel"
                  onClick={() => setConfirmingDelete(false)}
                >
                  {t('transactions.editMultiple.cancel')}
                </button>
                <button
                  type="button"
                  className="modal-button modal-button--delete-confirm"
                  onClick={handleDelete}
                  disabled={bulkDeleteMutation.isPending}
                >
                  {t('transactions.editMultiple.deleteConfirmButton')}
                </button>
              </div>
            </div>
          ) : (
            <>
              <button
                type="button"
                className="modal-button modal-button--delete"
                onClick={() => setConfirmingDelete(true)}
              >
                {t('transactions.editMultiple.delete', { count })}
              </button>
              <div className="modal-footer-actions">
                <button
                  type="button"
                  className="modal-button modal-button--cancel"
                  onClick={handleClose}
                >
                  {t('transactions.editMultiple.cancel')}
                </button>
                <button
                  type="button"
                  className="modal-button modal-button--primary"
                  onClick={handleSave}
                  disabled={bulkUpdateMutation.isPending}
                >
                  {t('transactions.editMultiple.save')}
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
};
