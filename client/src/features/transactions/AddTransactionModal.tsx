import { useEffect, useRef, useState } from 'react';
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown } from '@fortawesome/free-solid-svg-icons';
import { useCategories } from '../categories/use-categories';
import { useCreateTransaction } from './use-create-transaction';
import './AddTransactionModal.css';

/**
 * The form doesn't collect an account (there's no multi-account UI yet), but the API requires
 * one — this is the value manually-added transactions carry until accounts exist.
 */
const MANUAL_ACCOUNT = 'Manual entry';

/**
 * The amount input accepts a comma or a period as the decimal separator (French keyboards
 * produce a comma), but a value with both — e.g. "1,234.56" — or a single separator followed by
 * 3+ digits — e.g. "1,234" — reads as thousands-grouped. Guessing which separator was meant would
 * silently change the amount rather than fail loudly, so both are rejected instead. Returns the
 * amount as a server-compatible decimal string ("1234.56"), or null if it's ambiguous.
 */
const normalizeAmount = (rawAmount: string): string | null => {
  const separators = rawAmount.match(/[.,]/g) ?? [];
  if (separators.length > 1) return null;

  const [wholePart, fractionPart] = rawAmount.split(/[.,]/);
  if (fractionPart !== undefined && fractionPart.length > 2) return null;

  return fractionPart === undefined ? wholePart : `${wholePart}.${fractionPart}`;
};

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
  const [category, setCategory] = useState('');
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);

  const createTransactionMutation = useCreateTransaction();

  useEffect(() => {
    if (!open) return;

    const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
      FOCUSABLE_SELECTOR,
    );
    focusable?.[0]?.focus();
  }, [open]);

  if (!open) return null;

  const handleKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      onClose();
      return;
    }

    if (e.key !== 'Tab') return;

    const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
      FOCUSABLE_SELECTOR,
    );
    if (!focusable || focusable.length === 0) return;

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
    setAmount(e.target.value.replace(/[^0-9.,]/g, ''));
  };

  const handleMerchantChange = (e: ChangeEvent<HTMLInputElement>) => {
    setError('');
    setMerchant(e.target.value);
  };

  const handleDateChange = (e: ChangeEvent<HTMLInputElement>) => {
    setError('');
    setDate(e.target.value);
  };

  const handleCategoryChange = (e: ChangeEvent<HTMLSelectElement>) => {
    setError('');
    setCategory(e.target.value);
  };

  const handleSubmit = () => {
    if (!amount.trim() || !merchant.trim() || !date.trim() || !category.trim()) {
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
        category_id: Number(category),
        account: MANUAL_ACCOUNT,
      },
      {
        onSuccess: () => {
          setAmount('');
          setMerchant('');
          setDate('');
          setCategory('');
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

          <label htmlFor="date">
            {t('transactions.add.date', 'Date')}
          </label>
          <input
            id="date"
            type="date"
            value={date}
            onChange={handleDateChange}
          />

          <label htmlFor="category">
            {t('transactions.add.category', 'Category')}
          </label>
          <div className="category-select-wrapper">
            <select
              id="category"
              value={category}
              onChange={handleCategoryChange}
            >
              <option value="">
                {t('transactions.add.selectCategory', 'Select a category')}
              </option>

              {categories?.map((category) => (
                <option key={category.id} value={category.id}>
                  {i18n.language === 'fr'
                    ? category.name_fr
                    : category.name_en}
                </option>
              ))}
            </select>
            <FontAwesomeIcon
              icon={faChevronDown}
              className="category-select-arrow"
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
