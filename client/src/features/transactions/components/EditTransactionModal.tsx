import { useEffect, useRef, useState } from 'react';
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { useCategories } from '../../categories/hooks/use-categories';
import { useTransaction } from '../hooks/use-transaction';
import { useUpdateTransaction } from '../hooks/use-update-transaction';
import { useDeleteTransaction } from '../hooks/use-delete-transaction';
import {
  amountTooLongToast,
  deleteTransactionFailedToast,
  deleteTransactionSucceededToast,
  invalidAmountToast,
  requiredFieldToast,
  updateTransactionFailedToast,
} from '../../../lib/toast';
import { normalizeAmount, sanitizeAmountInput } from '../utils/amount';
import { CategoryPicker } from './CategoryPicker';
import type { TransactionPatch } from '../utils/api';
import './AddTransactionModal.css';
import './EditTransactionModal.css';

type EditTransactionModalProps = {
  transactionId: number;
  onClose: () => void;
};

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

export const EditTransactionModal = ({
  transactionId,
  onClose,
}: EditTransactionModalProps) => {
  const { t, i18n } = useTranslation();
  const { data: categories } = useCategories();
  const {
    data: transaction,
    isPending,
    isError,
  } = useTransaction(transactionId);

  const [amount, setAmount] = useState('');
  const [merchant, setMerchant] = useState('');
  const [date, setDate] = useState('');
  const [categoryId, setCategoryId] = useState<number | null>(null);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const dialogRef = useRef<HTMLDivElement>(null);

  const updateTransactionMutation = useUpdateTransaction();
  const deleteTransactionMutation = useDeleteTransaction();

  useEffect(() => {
    if (!transaction) return;
    setAmount(transaction.amount);
    setMerchant(transaction.merchant);
    setDate(transaction.date);
    setCategoryId(transaction.category_id);
  }, [transaction]);

  useEffect(() => {
    if (isPending) return;

    const focusable =
      dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR);
    focusable?.[0]?.focus();
  }, [isPending]);

  /** The dialog's own focusable controls, plus — while it's open — the category popover's,
   * which portals to `document.body` and so isn't a DOM descendant of `dialogRef`. Without this,
   * Tab from the popover's last control (or Shift+Tab from its first) would leak focus out of the
   * modal instead of wrapping. */
  const getTrapFocusable = (): HTMLElement[] => {
    const dialogFocusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR) ??
        [],
    );
    const popover = document.querySelector('.category-picker-popover');
    const popoverFocusable = popover
      ? Array.from(popover.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR))
      : [];
    return [...dialogFocusable, ...popoverFocusable];
  };

  const handleKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      onClose();
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

  /** Every field commits on its own, immediately — there's no separate "Save" step. */
  const save = (patch: TransactionPatch) => {
    updateTransactionMutation.mutate(
      { id: transactionId, patch },
      { onError: () => updateTransactionFailedToast() },
    );
  };

  const handleAmountChange = (e: ChangeEvent<HTMLInputElement>) => {
    setAmount(sanitizeAmountInput(e.target.value));
  };

  const commitAmount = () => {
    if (!transaction) return;
    const result = normalizeAmount(amount.trim());
    if (!result.valid) {
      if (result.error === 'tooLong') amountTooLongToast();
      else invalidAmountToast();
      setAmount(transaction.amount);
      return;
    }
    setAmount(result.value);
    if (result.value !== transaction.amount) save({ amount: result.value });
  };

  const handleAmountKeyDown = (e: ReactKeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.currentTarget.blur();
    } else if (e.key === 'Escape') {
      e.stopPropagation();
      setAmount(transaction?.amount ?? '');
      e.currentTarget.blur();
    }
  };

  const handleMerchantChange = (e: ChangeEvent<HTMLInputElement>) => {
    setMerchant(e.target.value);
  };

  const commitMerchant = () => {
    if (!transaction) return;
    const trimmed = merchant.trim();
    if (trimmed === '') {
      requiredFieldToast();
      setMerchant(transaction.merchant);
      return;
    }
    setMerchant(trimmed);
    if (trimmed !== transaction.merchant) save({ merchant: trimmed });
  };

  const handleMerchantKeyDown = (e: ReactKeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.currentTarget.blur();
    } else if (e.key === 'Escape') {
      e.stopPropagation();
      setMerchant(transaction?.merchant ?? '');
      e.currentTarget.blur();
    }
  };

  const handleDateChange = (e: ChangeEvent<HTMLInputElement>) => {
    if (!transaction) return;
    const value = e.target.value;
    if (!value) {
      requiredFieldToast();
      setDate(transaction.date);
      return;
    }
    setDate(value);
    if (value !== transaction.date) save({ date: value });
  };

  const handleCategorySelect = (id: number) => {
    setCategoryId(id);
    if (transaction && id !== transaction.category_id) {
      save({ category_id: id });
    }
  };

  const handleDelete = () => {
    deleteTransactionMutation.mutate(transactionId, {
      onSuccess: () => {
        deleteTransactionSucceededToast();
        onClose();
      },
      onError: () => {
        deleteTransactionFailedToast();
        setConfirmingDelete(false);
      },
    });
  };

  const selectedCategory = categories?.find((c) => c.id === categoryId);
  const categoryLabel = selectedCategory
    ? i18n.language === 'fr'
      ? selectedCategory.name_fr
      : selectedCategory.name_en
    : t('transactions.add.selectCategory');

  return (
    <div
      className="edit-transaction-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        className="edit-transaction-panel"
        role="dialog"
        aria-modal="true"
        aria-label={t('transactions.edit.title')}
        onKeyDown={handleKeyDown}
      >
        <div className="edit-transaction-header">
          <button
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label={t('transactions.add.close')}
          >
            ×
          </button>
        </div>

        <div className="edit-transaction-body">
          {isPending && (
            <p className="modal-status">{t('transactions.edit.loading')}</p>
          )}

          {isError && (
            <p className="modal-status">{t('transactions.edit.notFound')}</p>
          )}

          {transaction && (
            <>
              <div className="edit-transaction-field">
                <label htmlFor="edit-amount">
                  {t('transactions.add.amount')}
                </label>
                <div className="amount-input-wrapper">
                  <span className="amount-prefix" aria-hidden="true">
                    $
                  </span>
                  <input
                    id="edit-amount"
                    type="text"
                    inputMode="decimal"
                    placeholder="0.00"
                    value={amount}
                    onChange={handleAmountChange}
                    onBlur={commitAmount}
                    onKeyDown={handleAmountKeyDown}
                  />
                </div>
              </div>

              <div className="edit-transaction-field">
                <label htmlFor="edit-merchant">
                  {t('transactions.add.merchant')}
                </label>
                <input
                  id="edit-merchant"
                  type="text"
                  placeholder={t('transactions.add.merchantPlaceholder')}
                  value={merchant}
                  onChange={handleMerchantChange}
                  onBlur={commitMerchant}
                  onKeyDown={handleMerchantKeyDown}
                />
              </div>

              <div className="edit-transaction-field">
                <label htmlFor="edit-date">
                  {t('transactions.add.date')}
                </label>
                <input
                  id="edit-date"
                  type="date"
                  value={date}
                  onChange={handleDateChange}
                />
              </div>

              <div className="edit-transaction-field">
                <label>{t('transactions.add.category')}</label>
                <CategoryPicker
                  categoryId={categoryId}
                  label={categoryLabel}
                  className={
                    selectedCategory
                      ? 'modal-category-trigger'
                      : 'modal-category-trigger modal-category-trigger--placeholder'
                  }
                  onSelect={handleCategorySelect}
                />
              </div>
            </>
          )}
        </div>

        {transaction && (
          <div className="edit-transaction-footer">
            {confirmingDelete ? (
              <div className="delete-confirm">
                <span className="delete-confirm-text">
                  {t('transactions.edit.deleteConfirmTitle')}{' '}
                  {t('transactions.edit.deleteConfirmDescription')}
                </span>
                <div className="modal-footer-actions">
                  <button
                    type="button"
                    className="modal-button modal-button--cancel"
                    onClick={() => setConfirmingDelete(false)}
                  >
                    {t('transactions.add.cancel')}
                  </button>
                  <button
                    type="button"
                    className="modal-button modal-button--delete-confirm"
                    onClick={handleDelete}
                    disabled={deleteTransactionMutation.isPending}
                  >
                    {t('transactions.edit.deleteConfirmButton')}
                  </button>
                </div>
              </div>
            ) : (
              <button
                type="button"
                className="modal-button modal-button--delete"
                onClick={() => setConfirmingDelete(true)}
              >
                {t('transactions.edit.delete')}
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
