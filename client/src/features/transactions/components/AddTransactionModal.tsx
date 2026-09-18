import { useEffect, useRef, useState } from 'react';
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { useCategories } from '../../categories/hooks/use-categories';
import { useCreateTransaction } from '../hooks/use-create-transaction';
import { normalizeAmount, sanitizeAmountInput } from '../utils/amount';
import { CategoryPicker } from './CategoryPicker';
import './AddTransactionModal.css';

/**
 * The form doesn't collect an account (there's no multi-account UI yet), but the API requires
 * one — this is the value manually-added transactions carry until accounts exist.
 */
const MANUAL_ACCOUNT = 'Manual entry';

type AddTransactionModalProps = {
  open: boolean;
  onClose: () => void;
};

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

export const AddTransactionModal = ({
  open,
  onClose,
}: AddTransactionModalProps) => {
  const { t, i18n } = useTranslation();
  const { data: categories } = useCategories();
  const [amount, setAmount] = useState('');
  const [merchant, setMerchant] = useState('');
  const [date, setDate] = useState('');
  const [categoryId, setCategoryId] = useState<number | null>(null);
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);

  const createTransactionMutation = useCreateTransaction();

  useEffect(() => {
    if (!open) return;

    const focusable =
      dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR);
    focusable?.[0]?.focus();
  }, [open]);

  if (!open) return null;

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

  const handleAmountChange = (e: ChangeEvent<HTMLInputElement>) => {
    setError('');
    setAmount(sanitizeAmountInput(e.target.value));
  };

  const handleMerchantChange = (e: ChangeEvent<HTMLInputElement>) => {
    setError('');
    setMerchant(e.target.value);
  };

  const handleDateChange = (e: ChangeEvent<HTMLInputElement>) => {
    setError('');
    setDate(e.target.value);
  };

  const handleCategorySelect = (id: number) => {
    setError('');
    setCategoryId(id);
  };

  const handleSubmit = () => {
    if (
      !amount.trim() ||
      !merchant.trim() ||
      !date.trim() ||
      categoryId === null
    ) {
      setError(t('transactions.add.required', 'All fields are required.'));
      return;
    }

    const normalizedAmount = normalizeAmount(amount.trim());
    if (normalizedAmount === null) {
      setError(
        t(
          'transactions.add.invalidAmount',
          'Enter a plain amount, e.g. 12.50, without thousands separators.',
        ),
      );
      return;
    }

    setError('');
    createTransactionMutation.mutate(
      {
        date,
        merchant: merchant.trim(),
        amount: normalizedAmount,
        category_id: categoryId,
        account: MANUAL_ACCOUNT,
      },
      {
        onSuccess: () => {
          setAmount('');
          setMerchant('');
          setDate('');
          setCategoryId(null);
          onClose();
        },
        onError: () => {
          setError(
            t(
              'transactions.add.failed',
              'Failed to add the transaction. Please try again.',
            ),
          );
        },
      },
    );
  };

  const selectedCategory = categories?.find((c) => c.id === categoryId);
  const categoryLabel = selectedCategory
    ? i18n.language === 'fr'
      ? selectedCategory.name_fr
      : selectedCategory.name_en
    : t('transactions.add.selectCategory', 'Select category');

  return (
    <div className="modal-overlay">
      <div
        ref={dialogRef}
        className="add-transaction-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="add-transaction-title"
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header">
          <h2 id="add-transaction-title">
            {t('transactions.add.title', 'Add transaction')}
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
          <label htmlFor="amount">
            {t('transactions.add.amount', 'Amount')}
          </label>
          <div className="amount-input-wrapper">
            <span className="amount-prefix" aria-hidden="true">
              $
            </span>
            <input
              id="amount"
              type="text"
              inputMode="decimal"
              placeholder="0.00"
              value={amount}
              onChange={handleAmountChange}
            />
          </div>

          <label htmlFor="merchant">
            {t('transactions.add.merchant', 'Merchant')}
          </label>
          <input
            id="merchant"
            type="text"
            placeholder={t(
              'transactions.add.merchantPlaceholder',
              'Merchant Name',
            )}
            value={merchant}
            onChange={handleMerchantChange}
          />

          <label htmlFor="date">{t('transactions.add.date', 'Date')}</label>
          <input
            id="date"
            type="date"
            value={date}
            onChange={handleDateChange}
          />

          <label>{t('transactions.add.category', 'Category')}</label>
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
            {t('transactions.add.cancel', 'Cancel')}
          </button>

          <button
            type="button"
            className="modal-button modal-button--primary"
            onClick={handleSubmit}
            disabled={createTransactionMutation.isPending}
          >
            {t('transactions.add.submit', 'Add transaction')}
          </button>
        </div>
      </div>
    </div>
  );
};
